use anyhow::{Result, bail};
use serde::Serialize;

use crate::cli::StatsArgs;
use crate::client::LeetCodeClient;
use crate::models::{
    LanguageProblemCount, RecentAcSubmission, UserCalendar, UserContestRanking,
    UserQuestionProgressV2,
};
use crate::render::{format_timestamp, print_json};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatsJsonOutput {
    user_slug: String,
    progress: Option<UserQuestionProgressV2>,
    languages: Vec<LanguageProblemCount>,
    calendar: Option<UserCalendar>,
    contest: Option<UserContestRanking>,
    recent_ac: Vec<RecentAcSubmission>,
}

pub async fn run(args: StatsArgs, client: &LeetCodeClient, json: bool) -> Result<()> {
    let user_slug = match args.user_slug {
        Some(slug) => slug,
        None => {
            let status = client.global_data().await?;
            if !status.is_signed_in {
                bail!("Pass `<USER_SLUG>` or configure credentials via `lcctl auth login` / `lcctl auth set`.");
            }
            status.user_slug.unwrap_or(status.username)
        }
    };

    let (progress, languages, calendar, contest, recent_ac) = tokio::try_join!(
        client.user_question_progress(&user_slug),
        client.language_stats(&user_slug),
        client.user_calendar(&user_slug, args.year),
        client.user_contest_ranking(&user_slug),
        client.recent_ac_submissions(&user_slug),
    )?;

    if json {
        return print_json(&StatsJsonOutput {
            user_slug,
            progress,
            languages,
            calendar,
            contest,
            recent_ac,
        });
    }

    println!("=== Stats for `{user_slug}` ===");
    if let Some(p) = progress {
        let total_ac: u64 = p.num_accepted_questions.iter().map(|x| x.count).sum();
        println!(
            "\nSolved ({total_ac} total, beats {:.1}%):",
            p.total_question_beats_percentage.unwrap_or(0.0)
        );
        for item in &p.num_accepted_questions {
            let beats = p
                .user_session_beats_percentage
                .iter()
                .find(|b| b.difficulty == item.difficulty)
                .map(|b| format!("{:.1}%", b.percentage))
                .unwrap_or_else(|| "-".to_string());
            println!("  {:<8} {:>4}  (beats {beats})", item.difficulty, item.count);
        }
    }

    if let Some(cal) = calendar {
        println!(
            "\nCalendar: streak={} | recentStreak={} | totalActiveDays={}",
            cal.streak, cal.recent_streak, cal.total_active_days
        );
    }

    if let Some(c) = contest {
        println!(
            "\nContest : rating={:.1} | attended={} | global={}/{} (top {:.2}%)",
            c.rating,
            c.attended_contests_count,
            c.global_ranking,
            c.global_total_participants,
            c.top_percentage
        );
    }

    if !languages.is_empty() {
        let mut sorted = languages;
        sorted.sort_by_key(|item| std::cmp::Reverse(item.problems_solved));
        println!("\nLanguages:");
        for lang in sorted.into_iter().take(8) {
            println!("  {:<12} {}", lang.language_name, lang.problems_solved);
        }
    }

    if !recent_ac.is_empty() {
        println!("\nRecent AC Submissions:");
        for item in recent_ac.into_iter().take(8) {
            let title = item
                .question
                .translated_title
                .as_deref()
                .filter(|t| !t.is_empty())
                .unwrap_or(&item.question.title);
            println!(
                "  [{}] {} ({}) - {}",
                item.question.question_frontend_id,
                title,
                item.question.title_slug,
                format_timestamp(item.submit_time)
            );
        }
    }

    Ok(())
}
