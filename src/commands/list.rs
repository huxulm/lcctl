use anyhow::{Result, bail};

use crate::cli::ListArgs;
use crate::client::LeetCodeClient;
use crate::render::print_json;

pub async fn run(args: ListArgs, client: &LeetCodeClient, json: bool) -> Result<()> {
    if !client.has_session() {
        bail!(
            "`problemsetQuestionListV2` on leetcode.cn requires authentication. Configure credentials first with `lcctl auth login` or `lcctl auth set --cookie \"...\"`."
        );
    }

    let list = client
        .problemset_question_list(args.keyword.as_deref(), args.limit, args.skip)
        .await?;

    if json {
        return print_json(&list);
    }

    if let Some(total) = list.total_length {
        println!(
            "Problems (showing {} of {}, hasMore: {}):",
            list.questions.len(),
            total,
            list.has_more.unwrap_or(false)
        );
    }
    for q in list.questions {
        let title = q
            .translated_title
            .as_deref()
            .filter(|t| !t.is_empty())
            .unwrap_or(&q.title);
        let status = q.status.as_deref().unwrap_or("-");
        println!(
            "  [{:>5}] {:<6} {:<9} {:5.1}%  {} ({})",
            q.question_frontend_id,
            q.difficulty,
            status,
            q.ac_rate * 100.0,
            title,
            q.title_slug
        );
    }
    Ok(())
}
