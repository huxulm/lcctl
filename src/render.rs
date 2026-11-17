use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::models::QuestionDetail;

/// Prints any serializable value as pretty-printed JSON.
pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(value).context("Failed to serialize JSON output")?;
    println!("{text}");
    Ok(())
}

/// Normalizes common CLI language aliases to LeetCode `langSlug` values.
pub fn normalize_lang_slug(input: &str) -> &str {
    match input.trim().to_ascii_lowercase().as_str() {
        "rs" | "rust" => "rust",
        "go" | "golang" => "golang",
        "py" | "py3" | "python3" => "python3",
        "python" | "py2" => "python",
        "c++" | "cpp" => "cpp",
        "c" => "c",
        "java" => "java",
        "ts" | "typescript" => "typescript",
        "js" | "javascript" => "javascript",
        "cs" | "c#" | "csharp" => "csharp",
        "kt" | "kotlin" => "kotlin",
        "swift" => "swift",
        "rb" | "ruby" => "ruby",
        "php" => "php",
        "dart" => "dart",
        "scala" => "scala",
        "cj" | "cangjie" => "cangjie",
        _ => input.trim(),
    }
}

/// Returns file extension and single-line comment prefix for a LeetCode `langSlug`.
pub fn lang_file_meta(lang_slug: &str) -> (&'static str, &'static str) {
    match lang_slug {
        "rust" => ("rs", "//"),
        "golang" => ("go", "//"),
        "python3" | "python" => ("py", "#"),
        "cpp" => ("cpp", "//"),
        "c" => ("c", "//"),
        "java" => ("java", "//"),
        "typescript" => ("ts", "//"),
        "javascript" => ("js", "//"),
        "csharp" => ("cs", "//"),
        "kotlin" => ("kt", "//"),
        "swift" => ("swift", "//"),
        "ruby" => ("rb", "#"),
        "php" => ("php", "//"),
        "dart" => ("dart", "//"),
        "scala" => ("scala", "//"),
        "elixir" => ("ex", "#"),
        "erlang" => ("erl", "%"),
        "racket" => ("rkt", ";"),
        "cangjie" => ("cj", "//"),
        _ => ("txt", "//"),
    }
}

/// Formats a Unix timestamp (seconds) into local `YYYY-MM-DD HH:MM:SS`.
pub fn format_timestamp(ts: i64) -> String {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|dt| {
            dt.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|| ts.to_string())
}

/// Converts LeetCode HTML problem description into clean terminal-friendly plain text.
pub fn html_to_text(html: &str) -> String {
    let with_breaks = html
        .replace("\r\n", "\n")
        .replace("</p>", "\n\n")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</li>", "\n")
        .replace("<li>", "  - ")
        .replace("</pre>", "\n")
        .replace("<pre>", "\n")
        .replace("<sup>", "^");

    let mut stripped = String::with_capacity(with_breaks.len());
    let mut in_tag = false;
    for ch in with_breaks.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => stripped.push(ch),
            _ => {}
        }
    }

    let decoded = decode_html_entities(&stripped);
    collapse_blank_lines(&decoded)
}

fn decode_html_entities(input: &str) -> String {
    const ENTITIES: &[(&str, &str)] = &[
        ("&nbsp;", " "),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&amp;", "&"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&apos;", "'"),
        ("&le;", "<="),
        ("&ge;", ">="),
        ("&times;", "*"),
    ];
    let mut out = input.to_string();
    for &(entity, replacement) in ENTITIES {
        out = out.replace(entity, replacement);
    }
    out
}

fn collapse_blank_lines(input: &str) -> String {
    let mut lines = Vec::new();
    let mut empty_run = 0usize;
    for line in input.lines() {
        let trimmed_end = line.trim_end();
        if trimmed_end.trim().is_empty() {
            empty_run += 1;
            if empty_run <= 1 {
                lines.push(String::new());
            }
        } else {
            empty_run = 0;
            lines.push(trimmed_end.to_string());
        }
    }
    lines.join("\n").trim().to_string()
}

/// Writes a starter source file for a problem and returns the created file path.
pub fn scaffold_problem_file(
    question: &QuestionDetail,
    lang: &str,
    out_dir: &Path,
    force: bool,
) -> Result<PathBuf> {
    let slug = normalize_lang_slug(lang);
    let snippet = question
        .code_snippets
        .iter()
        .find(|s| s.lang_slug.eq_ignore_ascii_case(slug))
        .with_context(|| {
            let available: Vec<&str> = question
                .code_snippets
                .iter()
                .map(|s| s.lang_slug.as_str())
                .collect();
            format!(
                "Language `{lang}` is unavailable for `{}`. Available: {}",
                question.title_slug,
                available.join(", ")
            )
        })?;

    let (ext, comment) = lang_file_meta(&snippet.lang_slug);
    let safe_id = question
        .question_frontend_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let filename = format!("{safe_id}_{}.{ext}", question.title_slug.replace('-', "_"));
    fs::create_dir_all(out_dir)
        .with_context(|| format!("Failed to create output directory {}", out_dir.display()))?;
    let target = out_dir.join(filename);

    if target.exists() && !force {
        anyhow::bail!(
            "File {} already exists (pass `--force` to overwrite)",
            target.display()
        );
    }

    let title = question
        .translated_title
        .as_deref()
        .filter(|t| !t.is_empty())
        .unwrap_or(&question.title);

    let header = format!(
        "{comment} [{}] {} ({})\n{comment} Difficulty: {} | URL: https://leetcode.cn/problems/{}/\n\n{}\n",
        question.question_frontend_id,
        title,
        question.title,
        question.difficulty,
        question.title_slug,
        snippet.code.trim_end()
    );

    fs::write(&target, header)
        .with_context(|| format!("Failed to write file {}", target.display()))?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_html_entities_and_tags() {
        let html = "<p>Given <code>nums</code> &amp; <code>target</code> &le; 10<sup>4</sup>.</p><p>&nbsp;</p><pre>Input: [2,7]</pre>";
        let text = html_to_text(html);
        assert_eq!(text, "Given nums & target <= 10^4.\n\nInput: [2,7]");
    }

    #[test]
    fn normalizes_language_aliases() {
        assert_eq!(normalize_lang_slug("rs"), "rust");
        assert_eq!(normalize_lang_slug("go"), "golang");
        assert_eq!(normalize_lang_slug("py"), "python3");
        assert_eq!(lang_file_meta("rust"), ("rs", "//"));
        assert_eq!(lang_file_meta("python3"), ("py", "#"));
    }
}
