//! URL parser - extracts URLs from HTTP responses

use scraper::{Html, Selector};
use url::Url;

use super::frontier::{FrontierEntry, UrlSource};
use super::types::DiscoverOptions;
use crate::Response;

/// Response parser that extracts URLs from HTTP responses
#[derive(Debug, Clone)]
pub struct ResponseParser {
    discover: DiscoverOptions,
}

impl ResponseParser {
    pub fn new(discover: DiscoverOptions) -> Self {
        Self { discover }
    }

    /// Parse response and extract new URLs
    pub fn parse(&self, response: &Response, parent_url: &str) -> Vec<FrontierEntry> {
        let mut entries = Vec::new();

        // Parse based on discover options
        if self.discover.anchors {
            entries.extend(self.parse_anchors(response, parent_url));
        }

        if self.discover.forms {
            entries.extend(self.parse_forms(response, parent_url));
        }

        if self.discover.scripts {
            entries.extend(self.parse_scripts(response, parent_url));
        }

        if self.discover.iframes {
            entries.extend(self.parse_iframes(response, parent_url));
        }

        if self.discover.meta {
            entries.extend(self.parse_meta(response, parent_url));
        }

        if self.discover.canonical {
            entries.extend(self.parse_canonical(response, parent_url));
        }

        entries
    }

    /// Parse raw HTML string and extract new URLs (for browser-rendered content)
    #[allow(clippy::collapsible_if, clippy::needless_borrow)]
    pub fn parse_from_html(&self, html: &str, parent_url: &str) -> Vec<FrontierEntry> {
        let document = Html::parse_document(html);

        let mut entries = Vec::new();

        // Parse anchor tags from browser-rendered HTML
        if self.discover.anchors {
            if let Ok(selector) = Selector::parse("a[href]") {
                for element in document.select(&selector) {
                    if let Some(href) = element.value().attr("href") {
                        if !is_valid_nav_url(href) {
                            continue;
                        }
                        let resolved = resolve_url(parent_url, href);
                        entries.push(
                            FrontierEntry::new(resolved, 0)
                                .with_source(UrlSource::Anchor)
                                .with_attribute("href")
                                .with_source_url(parent_url.to_string()),
                        );
                    }
                }
            }
        }

        // Parse forms
        if self.discover.forms {
            if let Ok(selector) = Selector::parse("form[action]") {
                for element in document.select(&selector) {
                    if let Some(action) = element.value().attr("action") {
                        if action.is_empty() {
                            entries.push(
                                FrontierEntry::new(parent_url.to_string(), 0)
                                    .with_source(UrlSource::Form)
                                    .with_attribute("action")
                                    .with_source_url(parent_url.to_string()),
                            );
                        } else if is_valid_nav_url(&action) {
                            let resolved = resolve_url(parent_url, &action);
                            entries.push(
                                FrontierEntry::new(resolved, 0)
                                    .with_source(UrlSource::Form)
                                    .with_attribute("action")
                                    .with_source_url(parent_url.to_string()),
                            );
                        }
                    }
                }
            }
        }

        entries
    }

    #[allow(clippy::collapsible_if)]
    /// Parse anchor tags
    fn parse_anchors(&self, response: &Response, parent_url: &str) -> Vec<FrontierEntry> {
        let selector = match Selector::parse("a[href]") {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        response
            .document
            .select(&selector)
            .filter_map(|el| {
                let href = el.value().attr("href")?;
                if !is_valid_nav_url(href) {
                    return None;
                }
                let resolved = resolve_url(parent_url, href);
                Some(
                    FrontierEntry::new(resolved, 0)
                        .with_source(UrlSource::Anchor)
                        .with_attribute("href")
                        .with_source_url(response.url.clone()),
                )
            })
            .collect()
    }

    /// Parse form actions
    fn parse_forms(&self, response: &Response, parent_url: &str) -> Vec<FrontierEntry> {
        let selector = match Selector::parse("form[action]") {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        response
            .document
            .select(&selector)
            .filter_map(|el| {
                let action = el.value().attr("action")?;
                if action.is_empty() {
                    return Some(
                        FrontierEntry::new(parent_url.to_string(), 0)
                            .with_source(UrlSource::Form)
                            .with_attribute("action")
                            .with_source_url(response.url.clone()),
                    );
                }

                if is_valid_nav_url(action) {
                    Some(
                        FrontierEntry::new(resolve_url(parent_url, action), 0)
                            .with_source(UrlSource::Form)
                            .with_attribute("action")
                            .with_source_url(response.url.clone()),
                    )
                } else {
                    None
                }
            })
            .collect()
    }

    /// Parse script sources
    fn parse_scripts(&self, response: &Response, parent_url: &str) -> Vec<FrontierEntry> {
        let selector = match Selector::parse("script[src]") {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        response
            .document
            .select(&selector)
            .filter_map(|el| {
                let src = el.value().attr("src")?;
                if is_valid_nav_url(src) {
                    Some(
                        FrontierEntry::new(resolve_url(parent_url, src), 0)
                            .with_source(UrlSource::Script)
                            .with_attribute("src")
                            .with_source_url(response.url.clone()),
                    )
                } else {
                    None
                }
            })
            .collect()
    }

    /// Parse iframe sources
    fn parse_iframes(&self, response: &Response, parent_url: &str) -> Vec<FrontierEntry> {
        let selector = match Selector::parse("iframe[src], frame[src]") {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        response
            .document
            .select(&selector)
            .filter_map(|el| {
                let src = el.value().attr("src")?;
                if is_valid_nav_url(src) {
                    Some(
                        FrontierEntry::new(resolve_url(parent_url, src), 0)
                            .with_source(UrlSource::Iframe)
                            .with_attribute("src")
                            .with_source_url(response.url.clone()),
                    )
                } else {
                    None
                }
            })
            .collect()
    }

    /// Parse meta tags (refresh, og:url, etc.)
    #[allow(clippy::collapsible_if)]
    fn parse_meta(&self, response: &Response, parent_url: &str) -> Vec<FrontierEntry> {
        let mut entries = Vec::new();

        // Parse meta refresh (e.g., <meta http-equiv="refresh" content="5;url=http://example.com">
        let selector = match Selector::parse("meta[http-equiv='refresh'][content]") {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        for el in response.document.select(&selector) {
            if let Some(content) = el.value().attr("content") {
                if let Some(url) = extract_refresh_url(content) {
                    entries.push(
                        FrontierEntry::new(resolve_url(parent_url, &url), 0)
                            .with_source(UrlSource::Meta)
                            .with_attribute("content")
                            .with_source_url(response.url.clone()),
                    );
                }
            }
        }

        // Parse OpenGraph URL
        let og_selector = match Selector::parse("meta[property='og:url'][content]") {
            Ok(s) => s,
            Err(_) => return entries,
        };

        for el in response.document.select(&og_selector) {
            if let Some(content) = el.value().attr("content") {
                if is_valid_nav_url(content) {
                    entries.push(
                        FrontierEntry::new(resolve_url(parent_url, content), 0)
                            .with_source(UrlSource::Meta)
                            .with_attribute("property")
                            .with_source_url(response.url.clone()),
                    );
                }
            }
        }

        entries
    }

    /// Parse canonical link tag
    fn parse_canonical(&self, response: &Response, parent_url: &str) -> Vec<FrontierEntry> {
        let selector = match Selector::parse("link[rel='canonical'][href]") {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        response
            .document
            .select(&selector)
            .filter_map(|el| {
                let href = el.value().attr("href")?;
                if is_valid_nav_url(href) {
                    Some(
                        FrontierEntry::new(resolve_url(parent_url, href), 0)
                            .with_source(UrlSource::Canonical)
                            .with_attribute("href")
                            .with_source_url(response.url.clone()),
                    )
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Resolve a relative URL against a base URL
fn resolve_url(base: &str, relative: &str) -> String {
    // Handle fragment-only URLs
    if relative.starts_with('#') {
        return base.to_string();
    }

    // Handle absolute URLs
    if relative.starts_with("http://") || relative.starts_with("https://") {
        return relative.to_string();
    }

    // Resolve relative URL
    match Url::parse(base) {
        Ok(base_url) => base_url
            .join(relative)
            .map(|u| u.to_string())
            .unwrap_or_else(|_| relative.to_string()),
        Err(_) => relative.to_string(),
    }
}

/// Check if a URL is valid for navigation
fn is_valid_nav_url(url: &str) -> bool {
    let url = url.trim();
    if url.is_empty() {
        return false;
    }

    let lower = url.to_lowercase();
    // Skip non-navigation URLs
    if lower.starts_with("data:")
        || lower.starts_with("mailto:")
        || lower.starts_with("javascript:")
        || lower.starts_with("vbscript:")
        || lower.starts_with("tel:")
        || lower.starts_with("ftp:")
    {
        return false;
    }

    // Allow http, https, and relative URLs
    true
}

/// Extract URL from meta refresh content
fn extract_refresh_url(content: &str) -> Option<String> {
    // Content format: "5;url=http://example.com" or just "http://example.com"
    let parts: Vec<&str> = content.split(';').collect();

    if parts.len() == 1 {
        // Direct URL
        let url = parts[0].trim();
        if is_valid_nav_url(url) {
            return Some(url.to_string());
        }
    } else if parts.len() == 2 {
        // Timeout;url=format
        let url_part = parts[1].trim();
        if let Some(stripped) = url_part.strip_prefix("url=") {
            let url = stripped.trim();
            if is_valid_nav_url(url) {
                return Some(url.to_string());
            }
        }
    }

    None
}

/// Count links in a response
pub fn count_links(response: &Response) -> usize {
    let selector = match Selector::parse("a[href]") {
        Ok(s) => s,
        Err(_) => return 0,
    };

    response.document.select(&selector).count()
}

/// Count forms in a response
pub fn count_forms(response: &Response) -> usize {
    let selector = match Selector::parse("form") {
        Ok(s) => s,
        Err(_) => return 0,
    };

    response.document.select(&selector).count()
}

/// Parse sitemap.xml and extract URLs
pub fn parse_sitemap(xml: &str) -> Vec<String> {
    let mut urls = Vec::new();

    // Check if this is a sitemap index or urlset
    let is_sitemap_index = xml.contains("<sitemapindex");

    // Simple regex-based sitemap parsing
    // Look for <loc>...</loc> tags
    let loc_pattern = regex::Regex::new(r"<loc>([^<]+)</loc>").ok();

    if let Some(re) = loc_pattern {
        for cap in re.captures_iter(xml) {
            if let Some(url) = cap.get(1) {
                let url_str = url.as_str();
                // For sitemap index, only add .xml URLs
                // For urlset, add all URLs
                if is_sitemap_index {
                    if url_str.ends_with(".xml") {
                        urls.push(url_str.to_string());
                    }
                } else {
                    urls.push(url_str.to_string());
                }
            }
        }
    }

    urls
}
