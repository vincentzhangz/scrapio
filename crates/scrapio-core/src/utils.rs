//! Utility functions for Scrapio

pub mod url {
    use regex::Regex;

    /// Check if a URL is valid (http or https only)
    pub fn is_valid(url: &str) -> bool {
        let url_regex = Regex::new(r"^https?://[^\s]+$").unwrap();
        url_regex.is_match(url)
    }

    /// Extract domain from a URL
    pub fn get_domain(url: &str) -> Option<String> {
        let url_parsed = url::Url::parse(url).ok()?;
        url_parsed.host_str().map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::url;

    #[test]
    fn test_url_validation() {
        assert!(url::is_valid("https://example.com"));
        assert!(url::is_valid("http://example.com"));
        assert!(!url::is_valid("ftp://example.com"));
        assert!(!url::is_valid("not a url"));
        assert!(!url::is_valid(""));
    }

    #[test]
    fn test_domain_extraction() {
        assert_eq!(
            url::get_domain("https://example.com/page"),
            Some("example.com".to_string())
        );
        assert_eq!(
            url::get_domain("https://sub.example.com/page"),
            Some("sub.example.com".to_string())
        );
        assert_eq!(url::get_domain("invalid"), None);
    }
}
