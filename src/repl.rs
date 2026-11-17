use anyhow::Result;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{Config as EditorConfig, Context as LineContext, Editor, Helper};
use std::borrow::Cow;
use std::path::PathBuf;
use std::time::Duration;

use crate::cli::{Cli, Command};
use crate::client::LeetCodeClient;
use crate::config::Config;
use crate::dispatch_command;

const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/daily", "Show today's daily challenge (or --month YYYY-MM)"),
    ("/show", "Show problem details (defaults to active problem)"),
    ("/fetch", "Scaffold starter code for active or given problem"),
    ("/hints", "Show hints for active or given problem"),
    ("/use", "Set active problem context by slug or ID"),
    ("/list", "Search or list problems (requires login)"),
    ("/submissions", "List submissions for active or given problem"),
    ("/submission", "Inspect a submission by ID"),
    ("/stats", "Show solved stats, streak calendar, and contest rating"),
    ("/points", "Show points balance and recent points history"),
    ("/auth", "Manage login credentials (login/status/set/clear/config)"),
    ("/clear", "Clear terminal screen"),
    ("/help", "Show interactive command help"),
    ("/exit", "Exit interactive mode"),
];

const AUTH_SUBCOMMANDS: &[&str] = &["login", "status", "set", "clear", "config"];

/// Normalized action parsed from one line of interactive input.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplInput {
    Noop,
    Exit,
    ClearScreen,
    Help,
    UseTarget(String),
    CliArgv(Vec<String>),
}

/// Interactive session state retained across commands.
#[derive(Debug, Default)]
struct SessionState {
    active_slug: Option<String>,
    user_badge: Option<String>,
}

/// Rustyline helper providing `/` slash-command completion and inline hints.
#[derive(Default)]
pub struct SlashHelper;

impl Helper for SlashHelper {}
impl Validator for SlashHelper {}

impl Highlighter for SlashHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if !default {
            return Cow::Borrowed(prompt);
        }
        if prompt == "lcctl>" {
            return Cow::Owned("\x1b[36mlcctl\x1b[0m>".to_string());
        }
        if let Some(slug) = prompt
            .strip_prefix("lcctl (")
            .and_then(|rest| rest.strip_suffix(")>"))
        {
            return Cow::Owned(format!("\x1b[36mlcctl\x1b[0m \x1b[33m({slug})\x1b[0m>"));
        }
        Cow::Borrowed(prompt)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("\x1b[2m{hint}\x1b[0m"))
    }
}

impl Hinter for SlashHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &LineContext<'_>) -> Option<String> {
        if pos < line.len() || !line.starts_with('/') {
            return None;
        }
        if line == "/show " || line == "/use " {
            return Some("<slug|id>".to_string());
        }
        if line == "/submission " {
            return Some("<submission-id>".to_string());
        }
        if line == "/auth " {
            return Some("login | status | set | clear | config".to_string());
        }
        if line == "/list " {
            return Some("--keyword <text> --limit 20".to_string());
        }
        if !line.contains(' ') && line.len() > 1 {
            return SLASH_COMMANDS
                .iter()
                .find(|(cmd, _)| cmd.starts_with(line) && *cmd != line)
                .map(|(cmd, _)| cmd[line.len()..].to_string());
        }
        None
    }
}

impl Completer for SlashHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &LineContext<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let prefix = &line[..pos];
        if let Some(rest) = prefix.strip_prefix("/auth ")
            && !rest.contains(' ')
        {
            let start = "/auth ".len();
            let matches = AUTH_SUBCOMMANDS
                .iter()
                .filter(|sub| sub.starts_with(rest))
                .map(|sub| Pair {
                    display: (*sub).to_string(),
                    replacement: (*sub).to_string(),
                })
                .collect();
            return Ok((start, matches));
        }

        if prefix.starts_with('/') && !prefix.contains(' ') {
            let matches = SLASH_COMMANDS
                .iter()
                .filter(|(cmd, _)| cmd.starts_with(prefix))
                .map(|(cmd, desc)| Pair {
                    display: format!("{cmd:<14} {desc}"),
                    replacement: format!("{cmd} "),
                })
                .collect();
            return Ok((0, matches));
        }

        Ok((pos, Vec::new()))
    }
}

/// Parses one interactive line into a `ReplInput`, injecting `active_slug` where appropriate.
pub fn prepare_input(line: &str, active_slug: Option<&str>) -> Result<ReplInput> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(ReplInput::Noop);
    }

    let tokens = shell_words::split(trimmed)
        .map_err(|e| anyhow::anyhow!("Invalid command syntax: {e}"))?;
    if tokens.is_empty() {
        return Ok(ReplInput::Noop);
    }

    let head = tokens[0].strip_prefix('/').unwrap_or(&tokens[0]);
    let rest = &tokens[1..];

    match head {
        "exit" | "quit" | "q" => return Ok(ReplInput::Exit),
        "clear" | "cls" => return Ok(ReplInput::ClearScreen),
        "help" | "?" => return Ok(ReplInput::Help),
        "use" => {
            let target = rest
                .first()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| anyhow::anyhow!("Usage: /use <slug|id>"))?;
            return Ok(ReplInput::UseTarget(target.to_string()));
        }
        "fetch" => return build_contextual_show_argv(rest, active_slug, "--fetch", "/fetch"),
        "hints" => return build_contextual_show_argv(rest, active_slug, "--hints", "/hints"),
        "show" | "submissions" => {
            let mut argv = vec!["lcctl".to_string(), head.to_string()];
            let has_target = rest.first().is_some_and(|a| !a.starts_with('-'));
            if !has_target
                && !rest.iter().any(|a| a == "-h" || a == "--help")
                && let Some(slug) = active_slug
            {
                argv.push(slug.to_string());
            }
            argv.extend(rest.iter().cloned());
            return Ok(ReplInput::CliArgv(argv));
        }
        _ => {}
    }

    let mut argv = Vec::with_capacity(tokens.len() + 1);
    argv.push("lcctl".to_string());
    argv.push(head.to_string());
    argv.extend(rest.iter().cloned());
    Ok(ReplInput::CliArgv(argv))
}

fn build_contextual_show_argv(
    rest: &[String],
    active_slug: Option<&str>,
    flag: &str,
    cmd_name: &str,
) -> Result<ReplInput> {
    let mut argv = vec!["lcctl".to_string(), "show".to_string()];
    let has_target = rest.first().is_some_and(|a| !a.starts_with('-'));
    if !has_target {
        let slug = active_slug.ok_or_else(|| {
            anyhow::anyhow!(
                "No active problem. Pass a slug (`{cmd_name} two-sum`) or run `/daily` / `/use` first."
            )
        })?;
        argv.push(slug.to_string());
    }
    argv.push(flag.to_string());
    argv.extend(rest.iter().cloned());
    Ok(ReplInput::CliArgv(argv))
}

/// Runs the interactive Claude Code / AGY-style REPL loop.
pub async fn run(
    config: &mut Config,
    cookie_override: Option<&str>,
    default_json: bool,
) -> Result<()> {
    let mut client = LeetCodeClient::new(config.resolve_credentials(cookie_override))?;
    let mut state = SessionState::default();

    refresh_banner_state(&client, &mut state).await;
    print_welcome(&state);

    let editor_cfg = EditorConfig::builder().auto_add_history(true).build();
    let mut rl = Editor::<SlashHelper, DefaultHistory>::with_config(editor_cfg)?;
    rl.set_helper(Some(SlashHelper));

    let hist_path = history_file_path();
    if let Some(path) = &hist_path {
        let _ = rl.load_history(path);
    }

    loop {
        let prompt = format_prompt(&state);
        match rl.readline(&prompt) {
            Ok(line) => match prepare_input(&line, state.active_slug.as_deref()) {
                Ok(ReplInput::Noop) => {}
                Ok(ReplInput::Exit) => break,
                Ok(ReplInput::ClearScreen) => {
                    print!("\x1b[2J\x1b[H");
                }
                Ok(ReplInput::Help) => print_help(),
                Ok(ReplInput::UseTarget(target)) => {
                    let spinner = start_spinner("Resolving problem...");
                    let res = client.resolve_title_slug(&target).await;
                    spinner.finish_and_clear();
                    match res {
                        Ok(slug) => {
                            state.active_slug = Some(slug.clone());
                            println!("Active problem set to `{slug}`.");
                        }
                        Err(err) => eprintln!("Error: {err:#}"),
                    }
                }
                Ok(ReplInput::CliArgv(argv)) => {
                    match Cli::try_parse_from(argv) {
                        Ok(parsed) => {
                            let use_json = default_json || parsed.json;
                            let Some(cmd) = parsed.command else {
                                continue;
                            };
                            if matches!(cmd, Command::Repl) {
                                println!("Already in interactive mode.");
                                continue;
                            }
                            let is_auth_cmd = matches!(cmd, Command::Auth { .. });
                            match dispatch_command(cmd, config, &client, use_json).await {
                                Ok(maybe_slug) => {
                                    if let Some(slug) = maybe_slug {
                                        state.active_slug = Some(slug);
                                    }
                                    if is_auth_cmd {
                                        client = LeetCodeClient::new(
                                            config.resolve_credentials(cookie_override),
                                        )?;
                                        refresh_banner_state(&client, &mut state).await;
                                    }
                                }
                                Err(err) => eprintln!("Error: {err:#}"),
                            }
                        }
                        Err(err) => {
                            let _ = err.print();
                        }
                    }
                }
                Err(err) => eprintln!("Error: {err:#}"),
            },
            Err(ReadlineError::Interrupted) => {
                println!("^C");
            }
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("Readline error: {err}");
                break;
            }
        }
    }

    if let Some(path) = &hist_path {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = rl.save_history(path);
    }

    Ok(())
}

fn format_prompt(state: &SessionState) -> String {
    match &state.active_slug {
        Some(slug) => format!("lcctl ({slug})>"),
        None => "lcctl>".to_string(),
    }
}

async fn refresh_banner_state(client: &LeetCodeClient, state: &mut SessionState) {
    if !client.has_session() {
        state.user_badge = Some("anonymous (run /auth login)".to_string());
        return;
    }

    let spinner = start_spinner("Checking leetcode.cn session...");
    let (status_res, streak_res, points_res) = tokio::join!(
        client.global_data(),
        client.streak_counter(),
        client.points_total(),
    );
    spinner.finish_and_clear();

    match status_res {
        Ok(status) if status.is_signed_in => {
            let name = status
                .user_slug
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or(&status.username);
            let mut parts = vec![name.to_string()];
            if let Ok(Some(streak)) = streak_res {
                parts.push(format!("streak:{}d", streak.streak_count));
            }
            if let Ok(pts) = points_res {
                parts.push(format!("pts:{}", pts.points));
            }
            state.user_badge = Some(parts.join(" | "));
        }
        Ok(_) => {
            state.user_badge = Some("session expired (run /auth login)".to_string());
        }
        Err(_) => {
            state.user_badge = Some("offline".to_string());
        }
    }
}

fn start_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

fn print_welcome(state: &SessionState) {
    let badge = state.user_badge.as_deref().unwrap_or("ready");
    println!(
        "\x1b[1mlcctl\x1b[0m v{} \x1b[2m({badge})\x1b[0m",
        env!("CARGO_PKG_VERSION")
    );
    println!(
        "\x1b[2mType `/` + Tab to complete commands, `/help` for cheatsheet, `/exit` to quit.\x1b[0m"
    );
}

fn print_help() {
    println!("Interactive Commands:");
    for (cmd, desc) in SLASH_COMMANDS {
        println!("  {cmd:<14} {desc}");
    }
    println!("\nTip: `/daily` and `/show <slug>` set the active problem for `/fetch`, `/hints`, and `/submissions`.");
}

fn history_file_path() -> Option<PathBuf> {
    dirs::data_local_dir()
        .or_else(dirs::config_dir)
        .map(|d| d.join("lcctl").join("history.txt"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyline::history::DefaultHistory;

    #[test]
    fn parses_slash_and_bare_commands_with_active_context() {
        assert_eq!(prepare_input("  ", None).unwrap(), ReplInput::Noop);
        assert_eq!(prepare_input("/exit", None).unwrap(), ReplInput::Exit);
        assert_eq!(
            prepare_input("/use two-sum", None).unwrap(),
            ReplInput::UseTarget("two-sum".into())
        );

        assert_eq!(
            prepare_input("/fetch --lang go", Some("two-sum")).unwrap(),
            ReplInput::CliArgv(vec![
                "lcctl".into(),
                "show".into(),
                "two-sum".into(),
                "--fetch".into(),
                "--lang".into(),
                "go".into(),
            ])
        );

        assert_eq!(
            prepare_input("/submissions", Some("two-sum")).unwrap(),
            ReplInput::CliArgv(vec![
                "lcctl".into(),
                "submissions".into(),
                "two-sum".into(),
            ])
        );

        assert_eq!(
            prepare_input("/list --keyword \"two sum\" -l 5", None).unwrap(),
            ReplInput::CliArgv(vec![
                "lcctl".into(),
                "list".into(),
                "--keyword".into(),
                "two sum".into(),
                "-l".into(),
                "5".into(),
            ])
        );
    }

    #[test]
    fn requires_active_problem_for_context_fetch_without_target() {
        assert!(prepare_input("/fetch", None).is_err());
    }

    #[test]
    fn completes_slash_commands_and_auth_subcommands() {
        let helper = SlashHelper;
        let history = DefaultHistory::new();
        let ctx = LineContext::new(&history);

        let (start, pairs) = helper.complete("/da", 3, &ctx).unwrap();
        assert_eq!(start, 0);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].replacement, "/daily ");

        let (auth_start, auth_pairs) = helper.complete("/auth lo", 8, &ctx).unwrap();
        assert_eq!(auth_start, 6);
        assert_eq!(auth_pairs.len(), 1);
        assert_eq!(auth_pairs[0].replacement, "login");
    }

    #[test]
    fn prompt_has_no_raw_ansi_and_highlights_cleanly() {
        let state = SessionState::default();
        let plain = format_prompt(&state);
        assert_eq!(plain, "lcctl>");
        assert!(!plain.contains('\x1b'));

        let helper = SlashHelper;
        assert_eq!(
            helper.highlight_prompt(&plain, true).as_ref(),
            "\x1b[36mlcctl\x1b[0m>"
        );

        let active_state = SessionState {
            active_slug: Some("two-sum".into()),
            user_badge: None,
        };
        let active_plain = format_prompt(&active_state);
        assert_eq!(active_plain, "lcctl (two-sum)>");
        assert_eq!(
            helper.highlight_prompt(&active_plain, true).as_ref(),
            "\x1b[36mlcctl\x1b[0m \x1b[33m(two-sum)\x1b[0m>"
        );
    }
}
