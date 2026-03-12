//! HTML extraction utilities — strip HTML to plain text and extract links/metadata

use std::sync::LazyLock;

use regex::Regex;
use scraper::{Html, Selector};

use crate::AiExtractionResult;

static LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"https?://[^\s]+").unwrap());

/// Strip HTML tags to plain text, preferring main content containers
pub fn strip_html(html: &str) -> String {
    let document = Html::parse_fragment(html);

    let main_content = Selector::parse("main, article, .content, #content, body")
        .ok()
        .and_then(|s| document.select(&s).next())
        .map(|el| el.inner_html());

    let content = main_content.unwrap_or_else(|| html.to_string());

    content
        .replace(&['<', '>'][..], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract absolute URLs from plain text using regex
pub fn extract_links(text: &str) -> Vec<String> {
    LINK_REGEX
        .find_iter(text)
        .map(|m| {
            m.as_str()
                .trim_end_matches(|c: char| {
                    !c.is_alphanumeric()
                        && c != ':'
                        && c != '/'
                        && c != '.'
                        && c != '-'
                        && c != '_'
                        && c != '~'
                })
                .to_string()
        })
        .collect()
}

/// Fallback extraction without AI — uses CSS selectors to pull title, headings, links, and description
pub fn fallback_extraction(content: &str, url: &str) -> AiExtractionResult {
    let document = Html::parse_fragment(content);

    let title = select_first_inner_html(&document, "title");

    let headings: Vec<String> = Selector::parse("h1, h2, h3")
        .ok()
        .map(|s| {
            document
                .select(&s)
                .map(|el| el.inner_html().trim().to_string())
                .collect()
        })
        .unwrap_or_default();

    let links: Vec<String> = Selector::parse("a[href]")
        .ok()
        .map(|s| {
            document
                .select(&s)
                .filter_map(|el| el.value().attr("href").map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let description = Selector::parse("meta[name=description]")
        .ok()
        .and_then(|s| document.select(&s).next())
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.to_string());

    let main_content = select_first_inner_html(&document, "main, article, .content, #content");

    let data = serde_json::json!({
        "title": title,
        "description": description,
        "headings": headings,
        "content": main_content,
        "url": url,
    });

    AiExtractionResult {
        url: url.to_string(),
        data,
        markdown: None,
        links,
        used_fallback: true,
        model: "fallback".to_string(),
    }
}

fn select_first_inner_html(document: &Html, selector: &str) -> Option<String> {
    Selector::parse(selector)
        .ok()
        .and_then(|s| document.select(&s).next())
        .map(|el| el.inner_html().trim().to_string())
}
