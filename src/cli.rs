use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// LeetCode China (`leetcode.cn`) CLI assistant for daily coding tasks.
#[derive(Debug, Parser)]
#[command(name = "lcctl", version, about)]
pub struct Cli {
    /// Output raw JSON instead of human-readable text.
    #[arg(long, global = true)]
    pub json: bool,

    /// Override Cookie header for this invocation (e.g. "LEETCODE_SESSION=...; csrftoken=...").
    #[arg(long, global = true, env = "LCCTL_COOKIE", hide_env_values = true)]
    pub cookie: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start interactive terminal mode (also used when running `lcctl` with no arguments).
    #[command(alias = "i")]
    Repl,
    /// Manage authentication credentials and default CLI preferences.
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Show today's daily challenge and check-in status (optionally scaffold code).
    Daily(DailyArgs),
    /// Show full details of a problem by slug or ID (optionally scaffold code).
    Show(ShowArgs),
    /// Search or list problems from the problemset (requires login).
    List(ListArgs),
    /// List your submissions for a problem (requires login).
    Submissions(SubmissionsArgs),
    /// Inspect code and metrics for a specific submission ID (requires login).
    Submission(SubmissionArgs),
    /// Show user progress, language stats, streak calendar, recent ACs, and contest rating.
    Stats(StatsArgs),
    /// Show current points balance and recent points history (requires login).
    Points(PointsArgs),
}

#[derive(Debug, Subcommand)]
pub enum AuthAction {
    /// Open a browser window for interactive login on leetcode.cn and save cookies via CDP.
    Login {
        /// Path to Chrome/Edge/Chromium executable (auto-detected if omitted).
        #[arg(long)]
        browser_path: Option<PathBuf>,
        /// Custom Chromium user-data-dir (defaults to a dedicated lcctl profile).
        #[arg(long)]
        profile_dir: Option<PathBuf>,
        /// Maximum seconds to wait for login completion.
        #[arg(long, default_value_t = 300)]
        timeout: u64,
        /// Clear existing browser cookies in the dedicated profile before opening login.
        #[arg(long)]
        fresh: bool,
    },
    /// Verify whether the current session credentials are active on leetcode.cn.
    Status,
    /// Save LEETCODE_SESSION / csrftoken or a full Cookie header into config.toml.
    Set {
        #[arg(long)]
        session: Option<String>,
        #[arg(long)]
        csrf: Option<String>,
        #[arg(long)]
        cookie: Option<String>,
    },
    /// Clear saved credentials from config.toml.
    Clear,
    /// Set or display default language and output directory.
    Config {
        #[arg(long)]
        lang: Option<String>,
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

#[derive(Debug, Args)]
pub struct DailyArgs {
    /// Also fetch full problem details and write a starter code file.
    #[arg(long)]
    pub fetch: bool,
    /// Programming language for `--fetch` (e.g. rust, golang, python3, cpp).
    #[arg(long)]
    pub lang: Option<String>,
    /// Output directory for `--fetch` (defaults to config `workspace_dir` or `.`).
    #[arg(long)]
    pub out_dir: Option<PathBuf>,
    /// Overwrite existing file when using `--fetch`.
    #[arg(long)]
    pub force: bool,
    /// Show recent monthly daily question records for YYYY-MM (e.g. 2026-11).
    #[arg(long)]
    pub month: Option<String>,
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    /// Problem titleSlug (e.g. `two-sum`) or numeric ID (e.g. `1`, requires login).
    pub target: String,
    /// Write a starter code file locally.
    #[arg(long)]
    pub fetch: bool,
    /// Programming language for `--fetch` (e.g. rust, golang, python3, cpp).
    #[arg(long)]
    pub lang: Option<String>,
    /// Output directory for `--fetch`.
    #[arg(long)]
    pub out_dir: Option<PathBuf>,
    /// Overwrite existing file when using `--fetch`.
    #[arg(long)]
    pub force: bool,
    /// Show problem hints.
    #[arg(long)]
    pub hints: bool,
    /// Prefer English description instead of Chinese translation.
    #[arg(long)]
    pub english: bool,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Optional search keyword.
    #[arg(short, long)]
    pub keyword: Option<String>,
    /// Maximum number of problems to return.
    #[arg(short, long, default_value_t = 20)]
    pub limit: u32,
    /// Pagination offset.
    #[arg(short, long, default_value_t = 0)]
    pub skip: u32,
}

#[derive(Debug, Args)]
pub struct SubmissionsArgs {
    /// Problem titleSlug (e.g. `two-sum`) or numeric ID (e.g. `1`).
    pub target: String,
    /// Maximum number of submissions to return.
    #[arg(short, long, default_value_t = 10)]
    pub limit: u32,
    /// Pagination offset.
    #[arg(short, long, default_value_t = 0)]
    pub offset: u32,
}

#[derive(Debug, Args)]
pub struct SubmissionArgs {
    /// Submission ID (e.g. `607391099`).
    pub id: String,
}

#[derive(Debug, Args)]
pub struct StatsArgs {
    /// Target userSlug (defaults to current logged-in user).
    pub user_slug: Option<String>,
    /// Calendar year to query (optional).
    #[arg(long)]
    pub year: Option<i32>,
}

#[derive(Debug, Args)]
pub struct PointsArgs {
    /// Maximum number of recent points records to show (0 for all returned records).
    #[arg(short, long, default_value_t = 15)]
    pub limit: usize,
}
