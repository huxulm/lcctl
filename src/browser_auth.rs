use anyhow::{Context, Result, bail};
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures_util::StreamExt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::client::LeetCodeClient;
use crate::config::Credentials;
use crate::models::UserStatus;

const LOGIN_URL: &str = "https://leetcode.cn/accounts/login/";
const POLL_INTERVAL: Duration = Duration::from_millis(1500);

/// Options controlling the interactive Chromium login flow.
#[derive(Debug, Clone)]
pub struct BrowserLoginOptions {
    pub browser_path: Option<PathBuf>,
    pub profile_dir: Option<PathBuf>,
    pub timeout_secs: u64,
    pub fresh: bool,
}

/// Authenticated cookies and verified user status captured from the browser.
#[derive(Debug, Clone)]
pub struct BrowserLoginResult {
    pub session: String,
    pub csrftoken: Option<String>,
    pub user_status: UserStatus,
}

/// Returns the default dedicated Chromium profile directory for `lcctl`.
pub fn default_profile_dir() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .or_else(dirs::config_dir)
        .context("Unable to determine local data directory for browser profile")?;
    Ok(base.join("lcctl").join("browser-profile"))
}

/// Finds a Chrome, Edge, or Chromium binary when `--browser-path` is not explicitly provided.
pub fn detect_browser_executable() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let candidates = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ];
    #[cfg(target_os = "macos")]
    let candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
    ];
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let candidates = [
        "/usr/bin/google-chrome",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/microsoft-edge",
    ];

    candidates
        .iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
}

/// Extracts `LEETCODE_SESSION` and `csrftoken` from `(name, value, domain)` cookie tuples.
pub fn extract_leetcode_cookies<'a, I>(cookies: I) -> (Option<String>, Option<String>)
where
    I: IntoIterator<Item = (&'a str, &'a str, &'a str)>,
{
    let mut session = None;
    let mut csrftoken = None;
    for (name, value, domain) in cookies {
        let domain_lower = domain.to_ascii_lowercase();
        if !domain_lower.contains("leetcode.cn") || value.trim().is_empty() {
            continue;
        }
        if name == "LEETCODE_SESSION" {
            session = Some(value.to_string());
        } else if name == "csrftoken" {
            csrftoken = Some(value.to_string());
        }
    }
    (session, csrftoken)
}

/// Launches a visible Chromium window for the user to sign in on `leetcode.cn`,
/// then captures and verifies the authenticated session cookies via CDP.
pub async fn login_via_browser(opts: BrowserLoginOptions) -> Result<BrowserLoginResult> {
    let profile_dir = match opts.profile_dir {
        Some(dir) => dir,
        None => default_profile_dir()?,
    };
    std::fs::create_dir_all(&profile_dir).with_context(|| {
        format!(
            "Failed to create browser profile directory {}",
            profile_dir.display()
        )
    })?;

    let mut builder = BrowserConfig::builder()
        .user_data_dir(&profile_dir)
        .with_head()
        .arg("--disable-blink-features=AutomationControlled");

    if let Some(exe) = opts.browser_path.or_else(detect_browser_executable) {
        builder = builder.chrome_executable(exe);
    }

    let browser_config = builder
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build Chromium launch config: {e}"))?;

    let (mut browser, mut handler) = Browser::launch(browser_config)
        .await
        .context("Failed to launch Chromium/Edge via chromiumoxide")?;

    let handler_task = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if event.is_err() {
                break;
            }
        }
    });

    let outcome = wait_for_authenticated_cookies(&mut browser, opts.timeout_secs, opts.fresh).await;

    let _ = browser.close().await;
    handler_task.abort();

    outcome
}

async fn wait_for_authenticated_cookies(
    browser: &mut Browser,
    timeout_secs: u64,
    fresh: bool,
) -> Result<BrowserLoginResult> {
    if fresh {
        let _ = browser.clear_cookies().await;
    }

    let _page = browser
        .new_page(LOGIN_URL)
        .await
        .context("Failed to open leetcode.cn login page in browser")?;

    eprintln!(
        "Opened {LOGIN_URL} in browser. Please complete login (timeout: {timeout_secs}s)..."
    );

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let mut last_rejected_session: Option<String> = None;

    loop {
        if Instant::now() >= deadline {
            bail!("Timed out after {timeout_secs}s waiting for leetcode.cn login to complete");
        }

        let cookies = browser
            .get_cookies()
            .await
            .context("Browser closed or disconnected before login completed")?;

        let (session_opt, csrf_opt) = extract_leetcode_cookies(
            cookies
                .iter()
                .map(|c| (c.name.as_str(), c.value.as_str(), c.domain.as_str())),
        );

        if let Some(session) = session_opt
            && last_rejected_session.as_deref() != Some(session.as_str())
        {
            let creds = Credentials {
                session: Some(session.clone()),
                csrftoken: csrf_opt.clone(),
                raw_cookie: None,
            };
            let client = LeetCodeClient::new(creds)?;
            match client.global_data().await {
                Ok(user_status) if user_status.is_signed_in => {
                    return Ok(BrowserLoginResult {
                        session,
                        csrftoken: csrf_opt,
                        user_status,
                    });
                }
                Ok(_) => {
                    last_rejected_session = Some(session);
                }
                Err(_) => {}
            }
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_only_leetcode_cn_session_and_csrf() {
        let raw = vec![
            ("LEETCODE_SESSION", "wrong-domain", ".leetcode.com"),
            ("LEETCODE_SESSION", "valid-sess", ".leetcode.cn"),
            ("csrftoken", "valid-csrf", "leetcode.cn"),
            ("other", "ignored", ".leetcode.cn"),
        ];
        let (session, csrf) = extract_leetcode_cookies(raw);
        assert_eq!(session.as_deref(), Some("valid-sess"));
        assert_eq!(csrf.as_deref(), Some("valid-csrf"));
    }
}
