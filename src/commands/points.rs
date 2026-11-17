use anyhow::{Result, bail};
use chrono::{DateTime, Local};
use serde::Serialize;

use crate::cli::PointsArgs;
use crate::client::LeetCodeClient;
use crate::models::PointScoreItem;
use crate::render::print_json;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PointsJsonOutput {
    points: i64,
    scores: Vec<PointScoreItem>,
}

pub async fn run(args: PointsArgs, client: &LeetCodeClient, json: bool) -> Result<()> {
    if !client.has_session() {
        bail!(
            "Viewing personal points requires login (`lcctl auth login` or `lcctl auth set`)."
        );
    }

    let (total, history) = tokio::try_join!(client.points_total(), client.points_history())?;

    let mut scores = history.scores;
    scores.sort_by(|a, b| b.date.cmp(&a.date));
    if args.limit > 0 {
        scores.truncate(args.limit);
    }

    if json {
        return print_json(&PointsJsonOutput {
            points: total.points,
            scores,
        });
    }

    println!("Points Balance: {}", total.points);
    if scores.is_empty() {
        println!("History       : (no records)");
        return Ok(());
    }

    println!("\nRecent Points History:");
    for item in scores {
        let sign = if item.score >= 0 { "+" } else { "" };
        let when = format_rfc3339(&item.date);
        println!("  {when}  {sign}{:<5}  {}", item.score, item.description);
    }

    Ok(())
}

fn format_rfc3339(raw: &str) -> String {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| {
            dt.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|_| raw.to_string())
}
