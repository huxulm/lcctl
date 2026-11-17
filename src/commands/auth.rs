use anyhow::Result;
use serde::Serialize;

use crate::browser_auth::{BrowserLoginOptions, login_via_browser};
use crate::cli::AuthAction;
use crate::client::LeetCodeClient;
use crate::config::{Config, extract_auth_fields};
use crate::models::{StreakCounter, UserStatus};
use crate::render::{normalize_lang_slug, print_json};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthStatusOutput {
    has_local_credentials: bool,
    config_path: String,
    user_status: UserStatus,
    streak_counter: Option<StreakCounter>,
}

pub async fn run(
    action: AuthAction,
    config: &mut Config,
    client: &LeetCodeClient,
    json: bool,
) -> Result<()> {
    match action {
        AuthAction::Login {
            browser_path,
            profile_dir,
            timeout,
            fresh,
        } => {
            let result = login_via_browser(BrowserLoginOptions {
                browser_path,
                profile_dir,
                timeout_secs: timeout,
                fresh,
            })
            .await?;

            config.session = Some(result.session);
            if let Some(csrf) = result.csrftoken {
                config.csrftoken = Some(csrf);
            }
            let path = config.save()?;

            if json {
                return print_json(&serde_json::json!({
                    "saved": true,
                    "path": path.display().to_string(),
                    "userStatus": result.user_status,
                }));
            }

            let display = result
                .user_status
                .real_name
                .as_deref()
                .filter(|n| !n.is_empty())
                .unwrap_or(&result.user_status.username);
            println!(
                "Logged in as {} (slug: {})",
                display,
                result.user_status.user_slug.as_deref().unwrap_or("-")
            );
            println!("Saved session cookies to {}", path.display());
        }
        AuthAction::Status => {
            let user_status = client.global_data().await?;
            let streak_counter = if user_status.is_signed_in {
                client.streak_counter().await?
            } else {
                None
            };
            let path = Config::path()?.display().to_string();
            if json {
                return print_json(&AuthStatusOutput {
                    has_local_credentials: client.has_session(),
                    config_path: path,
                    user_status,
                    streak_counter,
                });
            }

            println!("Config file : {path}");
            println!(
                "Credentials : {}",
                if client.has_session() {
                    "configured"
                } else {
                    "not configured"
                }
            );
            if user_status.is_signed_in {
                let display = user_status
                    .real_name
                    .as_deref()
                    .filter(|n| !n.is_empty())
                    .unwrap_or(&user_status.username);
                println!(
                    "Signed in   : {} (slug: {}, premium: {}, checkedInToday: {})",
                    display,
                    user_status.user_slug.as_deref().unwrap_or("-"),
                    user_status.is_premium,
                    user_status.checked_in_today
                );
                if let Some(streak) = streak_counter {
                    println!(
                        "Streak      : {} days (today completed: {}, skipped: {})",
                        streak.streak_count, streak.today_completed, streak.days_skipped
                    );
                }
            } else {
                println!("Signed in   : false (anonymous)");
            }
        }
        AuthAction::Set {
            session,
            csrf,
            cookie,
        } => {
            let (new_session, new_csrf) = extract_auth_fields(session, csrf, cookie)?;
            if let Some(s) = new_session {
                config.session = Some(s);
            }
            if let Some(c) = new_csrf {
                config.csrftoken = Some(c);
            }
            let path = config.save()?;
            if json {
                return print_json(&serde_json::json!({
                    "saved": true,
                    "path": path.display().to_string(),
                    "hasSession": config.session.is_some(),
                    "hasCsrfToken": config.csrftoken.is_some(),
                }));
            }
            println!("Saved credentials to {}", path.display());
            println!("Run `lcctl auth status` to verify the session against leetcode.cn.");
        }
        AuthAction::Clear => {
            config.session = None;
            config.csrftoken = None;
            let path = config.save()?;
            if json {
                return print_json(&serde_json::json!({
                    "cleared": true,
                    "path": path.display().to_string(),
                }));
            }
            println!("Cleared saved credentials in {}", path.display());
        }
        AuthAction::Config { lang, dir } => {
            let mut changed = false;
            if let Some(l) = lang {
                config.default_lang = Some(normalize_lang_slug(&l).to_string());
                changed = true;
            }
            if let Some(d) = dir {
                config.workspace_dir = Some(d);
                changed = true;
            }
            let path = if changed {
                config.save()?
            } else {
                Config::path()?
            };
            if json {
                return print_json(&serde_json::json!({
                    "path": path.display().to_string(),
                    "defaultLang": config.default_lang,
                    "workspaceDir": config.workspace_dir,
                }));
            }
            println!("Config file  : {}", path.display());
            println!(
                "Default lang : {}",
                config.default_lang.as_deref().unwrap_or("rust (fallback)")
            );
            println!(
                "Workspace    : {}",
                config
                    .workspace_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| ".".to_string())
            );
        }
    }
    Ok(())
}
