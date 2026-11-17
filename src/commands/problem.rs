use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;

use crate::cli::ShowArgs;
use crate::client::LeetCodeClient;
use crate::config::Config;
use crate::models::QuestionDetail;
use crate::render::{html_to_text, print_json, scaffold_problem_file};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ShowJsonOutput {
    question: QuestionDetail,
    scaffolded_file: Option<String>,
}

pub async fn run(
    args: ShowArgs,
    config: &Config,
    client: &LeetCodeClient,
    json: bool,
) -> Result<Option<String>> {
    let slug = client.resolve_title_slug(&args.target).await?;
    let detail = client.question_detail(&slug).await?;

    let scaffolded = if args.fetch {
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
        Some(scaffold_problem_file(&detail, lang, &dir, args.force)?)
    } else {
        None
    };

    if json {
        print_json(&ShowJsonOutput {
            question: detail,
            scaffolded_file: scaffolded.map(|p| p.display().to_string()),
        })?;
        return Ok(Some(slug));
    }

    let title = detail
        .translated_title
        .as_deref()
        .filter(|t| !t.is_empty())
        .unwrap_or(&detail.title);
    let tags: Vec<&str> = detail
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
        "[{}] {} ({}) [{}]{}",
        detail.question_frontend_id,
        title,
        detail.title,
        detail.difficulty,
        if detail.is_paid_only { " [VIP]" } else { "" }
    );
    println!("URL : https://leetcode.cn/problems/{}/", detail.title_slug);
    if !tags.is_empty() {
        println!("Tags: {}", tags.join(", "));
    }

    let html = if args.english {
        detail
            .content
            .as_deref()
            .or(detail.translated_content.as_deref())
    } else {
        detail
            .translated_content
            .as_deref()
            .or(detail.content.as_deref())
    };
    if let Some(body) = html {
        println!("\n{}", html_to_text(body));
    }

    if let Some(sample) = &detail.sample_test_case
        && !sample.trim().is_empty()
    {
        println!("\nSample Test Case:\n{sample}");
    }

    if args.hints && !detail.hints.is_empty() {
        println!("\nHints:");
        for (idx, hint) in detail.hints.iter().enumerate() {
            println!("  {}. {}", idx + 1, html_to_text(hint));
        }
    }

    if let Some(path) = scaffolded {
        println!("\nScaffolded: {}", path.display());
    }

    Ok(Some(slug))
}
