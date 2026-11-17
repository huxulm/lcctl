use anyhow::{Result, bail};

use crate::cli::{SubmissionArgs, SubmissionsArgs};
use crate::client::LeetCodeClient;
use crate::render::{format_timestamp, print_json};

pub async fn run_list(args: SubmissionsArgs, client: &LeetCodeClient, json: bool) -> Result<()> {
    if !client.has_session() {
        bail!("Viewing your submission list requires login (`lcctl auth login` or `lcctl auth set`).");
    }
    let slug = client.resolve_title_slug(&args.target).await?;
    let list = client
        .submission_list(&slug, args.offset, args.limit, None)
        .await?;

    if json {
        return print_json(&list);
    }

    println!("Submissions for `{slug}` (hasNext: {}):", list.has_next);
    if list.submissions.is_empty() {
        println!("  (no submissions found)");
        return Ok(());
    }

    for sub in list.submissions {
        let ts = sub
            .timestamp
            .parse::<i64>()
            .map(format_timestamp)
            .unwrap_or(sub.timestamp);
        println!(
            "  #{}  {:<18}  {:<10}  {:<8}  {:<8}  {}",
            sub.id, sub.status_display, sub.lang, sub.runtime, sub.memory, ts
        );
    }
    Ok(())
}

pub async fn run_detail(args: SubmissionArgs, client: &LeetCodeClient, json: bool) -> Result<()> {
    if !client.has_session() {
        bail!("Viewing submission details requires login (`lcctl auth login` or `lcctl auth set`).");
    }
    let detail = client.submission_detail(&args.id).await?;
    if json {
        return print_json(&detail);
    }

    println!(
        "Submission #{} | {} | {} ({})",
        args.id, detail.question.title_slug, detail.status_display, detail.lang_verbose_name
    );
    println!(
        "Time      : {}",
        format_timestamp(detail.timestamp)
    );
    println!(
        "Runtime   : {} (beats {:.1}%) | Memory: {} (beats {:.1}%)",
        detail.runtime,
        detail.runtime_percentile.unwrap_or(0.0),
        detail.memory,
        detail.memory_percentile.unwrap_or(0.0)
    );
    if let (Some(passed), Some(total)) = (detail.passed_test_case_cnt, detail.total_test_case_cnt) {
        println!("Testcases : {passed}/{total}");
    }
    if let Some(stdout) = detail.std_output.as_deref().filter(|s| !s.is_empty()) {
        println!("\nStdout:\n{stdout}");
    }
    println!("\nCode:\n{}", detail.code);
    Ok(())
}
