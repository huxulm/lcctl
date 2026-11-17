use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Persistent user configuration stored in `config.toml`.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub session: Option<String>,
    pub csrftoken: Option<String>,
    pub default_lang: Option<String>,
    pub workspace_dir: Option<PathBuf>,
}

/// Resolved authentication credentials used by the HTTP client.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Credentials {
    pub session: Option<String>,
    pub csrftoken: Option<String>,
    pub raw_cookie: Option<String>,
}

impl Credentials {
    /// Returns true if any session cookie material is available.
    pub fn has_session(&self) -> bool {
        self.session.is_some() || self.raw_cookie.is_some()
    }

    /// Formats the `Cookie` header value if credentials are present.
    pub fn cookie_header(&self) -> Option<String> {
        if let Some(raw) = &self.raw_cookie {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        let mut parts = Vec::new();
        if let Some(session) = &self.session
            && !session.trim().is_empty()
        {
            parts.push(format!("LEETCODE_SESSION={}", session.trim()));
        }
        if let Some(csrf) = &self.csrftoken
            && !csrf.trim().is_empty()
        {
            parts.push(format!("csrftoken={}", csrf.trim()));
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("; "))
        }
    }
}

impl Config {
    /// Returns the configuration file path (`LCCTL_CONFIG` override or user config dir).
    pub fn path() -> Result<PathBuf> {
        if let Ok(custom) = std::env::var("LCCTL_CONFIG")
            && !custom.trim().is_empty()
        {
            return Ok(PathBuf::from(custom));
        }
        let base = dirs::config_dir().context("Unable to determine user config directory")?;
        Ok(base.join("lcctl").join("config.toml"))
    }

    /// Loads configuration from disk, returning default if the file does not exist.
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        Self::load_from(&path)
    }

    /// Loads configuration from a specific path.
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        toml::from_str(&text)
            .with_context(|| format!("Failed to parse TOML config at {}", path.display()))
    }

    /// Saves configuration to disk, creating parent directories if needed.
    pub fn save(&self) -> Result<PathBuf> {
        let path = Self::path()?;
        self.save_to(&path)?;
        Ok(path)
    }

    /// Saves configuration to a specific path.
    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory {}", parent.display())
            })?;
        }
        let text = toml::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(path, text)
            .with_context(|| format!("Failed to write config to {}", path.display()))
    }

    /// Resolves effective credentials from CLI override, environment variables, and saved config.
    pub fn resolve_credentials(&self, cookie_override: Option<&str>) -> Credentials {
        if let Some(raw) = cookie_override.map(str::trim).filter(|s| !s.is_empty()) {
            let (session, csrftoken) = parse_cookie_header(raw);
            return Credentials {
                session,
                csrftoken,
                raw_cookie: Some(raw.to_string()),
            };
        }

        if let Ok(raw) = std::env::var("LCCTL_COOKIE")
            && !raw.trim().is_empty()
        {
            let trimmed = raw.trim().to_string();
            let (session, csrftoken) = parse_cookie_header(&trimmed);
            return Credentials {
                session,
                csrftoken,
                raw_cookie: Some(trimmed),
            };
        }

        let env_session = std::env::var("LEETCODE_SESSION")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let env_csrf = std::env::var("LEETCODE_CSRFTOKEN")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        Credentials {
            session: env_session.or_else(|| self.session.clone()),
            csrftoken: env_csrf.or_else(|| self.csrftoken.clone()),
            raw_cookie: None,
        }
    }
}

/// Parses `LEETCODE_SESSION` and `csrftoken` values from a raw `Cookie` header string.
pub fn parse_cookie_header(raw: &str) -> (Option<String>, Option<String>) {
    let mut session = None;
    let mut csrftoken = None;
    for pair in raw.split(';') {
        let Some((k, v)) = pair.trim().split_once('=') else {
            continue;
        };
        let key = k.trim();
        let val = v.trim();
        if val.is_empty() {
            continue;
        }
        if key.eq_ignore_ascii_case("LEETCODE_SESSION") {
            session = Some(val.to_string());
        } else if key.eq_ignore_ascii_case("csrftoken") {
            csrftoken = Some(val.to_string());
        }
    }
    (session, csrftoken)
}

/// Validates that at least one credential field is supplied when saving auth settings.
pub fn extract_auth_fields(
    session: Option<String>,
    csrf: Option<String>,
    cookie: Option<String>,
) -> Result<(Option<String>, Option<String>)> {
    if let Some(raw) = cookie {
        let (parsed_session, parsed_csrf) = parse_cookie_header(&raw);
        let final_session = session.or(parsed_session);
        let final_csrf = csrf.or(parsed_csrf);
        if final_session.is_none() && final_csrf.is_none() {
            bail!(
                "Could not find `LEETCODE_SESSION` or `csrftoken` in the provided cookie string"
            );
        }
        return Ok((final_session, final_csrf));
    }
    if session.is_none() && csrf.is_none() {
        bail!("Provide at least one of `--session`, `--csrf`, or `--cookie`");
    }
    Ok((session, csrf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_cookie_header_values() {
        let raw = "foo=1; LEETCODE_SESSION=abc.def.ghi; csrftoken=xyz123; bar=2";
        let (session, csrf) = parse_cookie_header(raw);
        assert_eq!(session.as_deref(), Some("abc.def.ghi"));
        assert_eq!(csrf.as_deref(), Some("xyz123"));
    }

    #[test]
    fn builds_cookie_header_from_fields() {
        let creds = Credentials {
            session: Some("sess".into()),
            csrftoken: Some("csrf".into()),
            raw_cookie: None,
        };
        assert_eq!(
            creds.cookie_header().as_deref(),
            Some("LEETCODE_SESSION=sess; csrftoken=csrf")
        );
    }

    #[test]
    fn config_roundtrip() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir()
            .join(format!("lcctl_test_{unique}"))
            .join("config.toml");
        let cfg = Config {
            session: Some("s1".into()),
            csrftoken: Some("c1".into()),
            default_lang: Some("rust".into()),
            workspace_dir: Some(PathBuf::from("problems")),
        };
        cfg.save_to(&path).unwrap();
        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(cfg, loaded);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }
}
