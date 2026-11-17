use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::path::PathBuf;

use crate::cli::DailyArgs;
use crate::client::LeetCodeClient;
use crate::config::Config;
use crate::models::{DailyRecordEntry, StreakCounter, TodayRecord};
use crate::render::{print_json, scaffold_problem_file};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DailyJsonOutput {
    today_record: Vec<TodayRecord>,
    streak_counter: Option<StreakCounter>,
    scaffolded_file: Option<String>,
}

pub async fn run(
    args: DailyArgs,
    config: &Config,
    client: &LeetCodeClient,
    json: bool,
) -> Result<Option<String>> {
    if let Some(month_str) = args.month {
        let (year, month) = parse_year_month(&month_str)?;
        let records: Vec<DailyRecordEntry> = client.daily_question_records(year, month).await?;
        if json {
            print_json(&records)?;
            return Ok(None);
        }
        println!("Daily Question Records ({year:04}-{month:02}):");
        for entry in records {
            let title = entry
                .question
                .translated_title
                .as_deref()
                .filter(|t| !t.is_empty())
                .unwrap_or(&entry.question.title);
            let status = entry.user_status.as_deref().unwrap_or("-");
            println!(
                "  {}  [{:>4}] {} ({})  [{}]",
                entry.date,
                entry.question.question_frontend_id,
                title,
                entry.question.title_slug,
                status
            );
        }
        return Ok(None);
    }

    let records = client.question_of_today().await?;
    let streak = if client.has_session() {
        client.streak_counter().await.ok().flatten()
    } else {
        None
    };

    let first = records
        .first()
        .context("No daily challenge returned by `questionOfTodayV2`")?;
    let active_slug = first.question.title_slug.clone();

    let scaffolded = if args.fetch {
        let detail = client.question_detail(&active_slug).await?;
        let lang = args
            .lang
            .as_deref()
            .or(config.default_lang.as_deref())
            .unwrap_or("rust");
        let dir = args
            .out_dir
            .clone()
            .or_else(|| config.workspace_dir.clone())
            .unwrap_or_else(|| PathBuf::from("."));
        let written = scaffold_problem_file(&detail, lang, &dir, args.force)?;
        Some(written)
    } else {
        None
    };

    if json {
        print_json(&DailyJsonOutput {
            today_record: records,
            streak_counter: streak,
            scaffolded_file: scaffolded.map(|p| p.display().to_string()),
        })?;
        return Ok(Some(active_slug));
    }

    let q = &first.question;
    let title = q
        .translated_title
        .as_deref()
        .filter(|t| !t.is_empty())
        .unwrap_or(&q.title);
    let tags: Vec<&str> = q
        .topic_tags
        .iter()
        .map(|t| {
            t.name_translated
                .as_deref()
                .filter(|n| !n.is_empty())
                .unwrap_or(&t.name)
        })
        .collect();

    println!(
        "Daily Challenge ({}) : [{}] {} ({})",
        first.date, q.question_frontend_id, title, q.title
    );
    println!(
        "Slug / URL               : {} -> https://leetcode.cn/problems/{}/",
        q.title_slug, q.title_slug
    );
    println!(
        "Difficulty / AC Rate     : {} | {:.1}%",
        q.difficulty,
        q.ac_rate * 100.0
    );
    println!(
        "Tags                     : {}",
        if tags.is_empty() {
            "-".to_string()
        } else {
            tags.join(", ")
        }
    );
    if let Some(status) = first.user_status.as_deref().or(q.status.as_deref()) {
        println!("My Status                : {status}");
    }
    if let Some(s) = streak {
        println!(
            "Streak                   : {} days (today completed: {})",
            s.streak_count, s.today_completed
        );
    }
    if let Some(path) = scaffolded {
        println!("Scaffolded               : {}", path.display());
    }

    Ok(Some(active_slug))
}

fn parse_year_month(input: &str) -> Result<(i32, u32)> {
    let (y, m) = input
        .trim()
        .split_once('-')
        .context("Expected `--month` in YYYY-MM format (e.g. 2026-11)")?;
    let year: i32 = y.parse().context("Invalid year in `--month`")?;
    let month: u32 = m.parse().context("Invalid month in `--month`")?;
    if !(1..=12).contains(&month) {
        bail!("Month must be between 1 and 12");
    }
    Ok((year, month))
}
