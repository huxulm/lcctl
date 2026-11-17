use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::cli::{RunArgs, SubmitArgs};
use crate::client::LeetCodeClient;
use crate::config::Config;
use crate::models::{JudgeCheckResult, QuestionDetail};
use crate::render::{lang_slug_from_extension, normalize_lang_slug, print_json};

const POLL_INTERVAL: Duration = Duration::from_millis(1000);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunJsonOutput {
    slug: String,
    lang: String,
    interpret_id: String,
    result: JudgeCheckResult,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmitJsonOutput {
    slug: String,
    lang: String,
    submission_id: u64,
    result: JudgeCheckResult,
}

struct PreparedSubmission {
    detail: QuestionDetail,
    lang: String,
    code: String,
}

pub async fn run_interpret(
    args: RunArgs,
    config: &Config,
    client: &LeetCodeClient,
    json: bool,
) -> Result<Option<String>> {
    let prep = prepare_submission(
        &args.file,
        args.target.as_deref(),
        args.lang.as_deref(),
        config,
        client,
    )
    .await?;

    let data_input = match args.input {
        Some(custom) => custom,
        None => prep
            .detail
            .example_testcases
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .or(prep
                .detail
                .sample_test_case
                .as_deref()
                .filter(|s| !s.trim().is_empty()))
            .context("No sample/example testcases found; provide `--input` explicitly")?
            .to_string(),
    };

    let interpret_id = client
        .interpret_solution(
            &prep.detail.title_slug,
            &prep.detail.question_id,
            &prep.lang,
            &prep.code,
            &data_input,
        )
        .await?;

    let result = poll_judge(
        client,
        &interpret_id,
        args.timeout,
        &format!("Running `{}` ({})...", prep.detail.title_slug, prep.lang),
        json,
    )
    .await?;

    if json {
        print_json(&RunJsonOutput {
            slug: prep.detail.title_slug.clone(),
            lang: prep.lang,
            interpret_id,
            result,
        })?;
        return Ok(Some(prep.detail.title_slug));
    }

    print_verdict(
        "Run",
        &prep.detail.question_frontend_id,
        &prep.detail.title_slug,
        &prep.lang,
        &interpret_id,
        &result,
    );
    Ok(Some(prep.detail.title_slug))
}

pub async fn run_submit(
    args: SubmitArgs,
    config: &Config,
    client: &LeetCodeClient,
    json: bool,
) -> Result<Option<String>> {
    let prep = prepare_submission(
        &args.file,
        args.target.as_deref(),
        args.lang.as_deref(),
        config,
        client,
    )
    .await?;

    let submission_id = client
        .submit_solution(
            &prep.detail.title_slug,
            &prep.detail.question_id,
            &prep.lang,
            &prep.code,
        )
        .await?;

    let id_str = submission_id.to_string();
    let result = poll_judge(
        client,
        &id_str,
        args.timeout,
        &format!("Submitting `{}` ({})...", prep.detail.title_slug, prep.lang),
        json,
    )
    .await?;

    if json {
        print_json(&SubmitJsonOutput {
            slug: prep.detail.title_slug.clone(),
            lang: prep.lang,
            submission_id,
            result,
        })?;
        return Ok(Some(prep.detail.title_slug));
    }

    print_verdict(
        "Submit",
        &prep.detail.question_frontend_id,
        &prep.detail.title_slug,
        &prep.lang,
        &id_str,
        &result,
    );
    Ok(Some(prep.detail.title_slug))
}

async fn prepare_submission(
    file: &Path,
    explicit_target: Option<&str>,
    explicit_lang: Option<&str>,
    config: &Config,
    client: &LeetCodeClient,
) -> Result<PreparedSubmission> {
    if !client.has_session() {
        bail!("Running or submitting code requires login (`lcctl auth login` or `lcctl auth set`).");
    }

    let code = fs::read_to_string(file)
        .with_context(|| format!("Failed to read solution file {}", file.display()))?;
    if code.trim().is_empty() {
        bail!("Solution file {} is empty", file.display());
    }

    let target = infer_target(explicit_target, &code, file)?;
    let slug = client.resolve_title_slug(&target).await?;
    let detail = client.question_detail(&slug).await?;
    let lang = infer_lang(explicit_lang, file, config)?;

    Ok(PreparedSubmission { detail, lang, code })
}

async fn poll_judge(
    client: &LeetCodeClient,
    check_id: &str,
    timeout_secs: u64,
    spinner_msg: &str,
    quiet: bool,
) -> Result<JudgeCheckResult> {
    let pb = if quiet {
        None
    } else {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        bar.set_message(spinner_msg.to_string());
        bar.enable_steady_tick(Duration::from_millis(80));
        Some(bar)
    };

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let outcome = loop {
        let check = client.check_submission(check_id).await;
        match check {
            Ok(res) if res.state == "SUCCESS" => break Ok(res),
            Ok(_) => {}
            Err(err) => break Err(err),
        }
        if Instant::now() >= deadline {
            break Err(anyhow::anyhow!(
                "Timed out after {timeout_secs}s waiting for judge result (`{check_id}`)"
            ));
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    };

    if let Some(bar) = pb {
        bar.finish_and_clear();
    }
    outcome
}

fn print_verdict(
    mode: &str,
    frontend_id: &str,
    slug: &str,
    lang: &str,
    check_id: &str,
    res: &JudgeCheckResult,
) {
    let status = res.status_msg.as_deref().unwrap_or(res.state.as_str());
    println!("{mode} [{frontend_id}] {slug} ({lang}) | ID: {check_id}");
    println!("Status    : {status}");

    if let (Some(passed), Some(total)) = (res.total_correct, res.total_testcases) {
        println!("Testcases : {passed}/{total}");
    }
    if let Some(correct) = res.correct_answer {
        println!("Correct   : {correct}");
    }
    if let Some(rt) = &res.status_runtime {
        let pct = res
            .runtime_percentile
            .map(|p| format!(" (beats {p:.1}%)"))
            .unwrap_or_default();
        println!("Runtime   : {rt}{pct}");
    }
    if let Some(mem) = &res.status_memory {
        let pct = res
            .memory_percentile
            .map(|p| format!(" (beats {p:.1}%)"))
            .unwrap_or_default();
        println!("Memory    : {mem}{pct}");
    }

    if !res.code_answer.is_empty() {
        let actual: Vec<&str> = res
            .code_answer
            .iter()
            .map(String::as_str)
            .filter(|s| !s.is_empty())
            .collect();
        let expected: Vec<&str> = res
            .expected_code_answer
            .iter()
            .map(String::as_str)
            .filter(|s| !s.is_empty())
            .collect();
        if !actual.is_empty() {
            println!("Output    : [{}]", actual.join(", "));
        }
        if !expected.is_empty() {
            println!("Expected  : [{}]", expected.join(", "));
        }
    }

    let stdouts: Vec<&str> = res
        .std_output_list
        .iter()
        .map(String::as_str)
        .filter(|s| !s.is_empty())
        .collect();
    if !stdouts.is_empty() {
        println!("Stdout    :\n{}", stdouts.join("\n---\n"));
    }

    if let Some(err) = res
        .full_compile_error
        .as_deref()
        .or(res.compile_error.as_deref())
        .filter(|s| !s.trim().is_empty())
    {
        println!("\nCompile Error:\n{err}");
    }
    if let Some(err) = res
        .full_runtime_error
        .as_deref()
        .or(res.runtime_error.as_deref())
        .filter(|s| !s.trim().is_empty())
    {
        println!("\nRuntime Error:\n{err}");
    }
    if let Some(tc) = res
        .last_testcase
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        println!("\nFailed Testcase:\n{tc}");
        if let Some(out) = res.code_output.as_deref() {
            println!("Your Output    : {out}");
        }
        if let Some(exp) = res.expected_output.as_deref() {
            println!("Expected Output: {exp}");
        }
    }
}

/// Resolves the target problem slug/ID from `--target`, scaffolded file URL header, or filename.
pub fn infer_target(explicit: Option<&str>, source: &str, file: &Path) -> Result<String> {
    if let Some(target) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(target.to_string());
    }

    for line in source.lines().take(20) {
        if let Some(pos) = line.find("leetcode.cn/problems/") {
            let after = &line[pos + "leetcode.cn/problems/".len()..];
            let slug = after
                .split(['/', ' ', '\t', ')'])
                .next()
                .unwrap_or("")
                .trim();
            if !slug.is_empty() {
                return Ok(slug.to_string());
            }
        }
    }

    if let Some(stem) = file.file_stem().and_then(|s| s.to_str()) {
        let digits: String = stem.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return Ok(digits);
        }
        let cleaned = stem.trim().replace('_', "-");
        if !cleaned.is_empty() {
            return Ok(cleaned);
        }
    }

    bail!("Unable to infer problem target from `{}`. Pass `--target <slug|id>`.", file.display())
}

/// Resolves the LeetCode `langSlug` from `--lang`, file extension, or `config.default_lang`.
pub fn infer_lang(explicit: Option<&str>, file: &Path, config: &Config) -> Result<String> {
    if let Some(lang) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(normalize_lang_slug(lang).to_string());
    }
    if let Some(ext) = file.extension().and_then(|e| e.to_str())
        && let Some(slug) = lang_slug_from_extension(ext)
    {
        return Ok(slug.to_string());
    }
    if let Some(default) = config.default_lang.as_deref() {
        return Ok(normalize_lang_slug(default).to_string());
    }
    bail!(
        "Unable to infer language from `{}`. Pass `--lang <lang>`.",
        file.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn infers_target_from_url_header_or_filename() {
        let src_with_header = "// [2911] Title\n// URL: https://leetcode.cn/problems/minimum-changes-to-make-k-semi-palindromes/\n";
        assert_eq!(
            infer_target(None, src_with_header, Path::new("solution.rs")).unwrap(),
            "minimum-changes-to-make-k-semi-palindromes"
        );

        assert_eq!(
            infer_target(
                None,
                "impl Solution {}",
                Path::new("2911_minimum_changes.rs")
            )
            .unwrap(),
            "2911"
        );

        assert_eq!(
            infer_target(Some("two-sum"), src_with_header, Path::new("x.rs")).unwrap(),
            "two-sum"
        );
    }

    #[test]
    fn infers_language_from_extension_or_override() {
        let cfg = Config::default();
        assert_eq!(
            infer_lang(None, &PathBuf::from("a.rs"), &cfg).unwrap(),
            "rust"
        );
        assert_eq!(
            infer_lang(None, &PathBuf::from("a.py"), &cfg).unwrap(),
            "python3"
        );
        assert_eq!(
            infer_lang(Some("go"), &PathBuf::from("a.txt"), &cfg).unwrap(),
            "golang"
        );
    }
}
