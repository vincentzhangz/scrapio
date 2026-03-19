//! Scope validation for URL filtering

use url::Url;

use super::types::Scope;
use super::types::ScopeMode;

/// Validates URLs against scope rules
pub struct ScopeValidator {
    root_url: Url,
    root_hostname: String,
    root_domain: String,
    scope: Scope,
}

impl ScopeValidator {
    /// Create a new scope validator from a root URL and scope config
    pub fn new(root_url: &str, scope: Scope) -> Result<Self, ScopeError> {
        let url = Url::parse(root_url).map_err(|e| ScopeError::InvalidRootUrl(e.to_string()))?;

        let hostname = url
            .host_str()
            .ok_or_else(|| ScopeError::InvalidRootUrl("no hostname".to_string()))?
            .to_string();

        // Extract domain (e.g., "example.com" from "www.example.com")
        let domain = extract_domain(&hostname);

        Ok(Self {
            root_url: url,
            root_hostname: hostname,
            root_domain: domain,
            scope,
        })
    }

    /// Check if a URL is in scope
    pub fn is_in_scope(&self, url: &str) -> bool {
        // First check regex exclusions
        for regex in &self.scope.regex_exclude {
            if regex.is_match(url) {
                return false;
            }
        }

        // Check regex inclusions (if any)
        if !self.scope.regex_include.is_empty() {
            let mut matched = false;
            for regex in &self.scope.regex_include {
                if regex.is_match(url) {
                    matched = true;
                    break;
                }
            }
            if !matched {
                return false;
            }
        }

        // Parse the URL
        let parsed = match Url::parse(url) {
            Ok(u) => u,
            Err(_) => return false,
        };

        // Check scheme
        if !is_allowed_scheme(parsed.scheme()) {
            return false;
        }

        // Check based on scope mode
        match self.scope.mode {
            ScopeMode::Host => self.is_same_host(&parsed),
            ScopeMode::Domain => self.is_same_domain(&parsed),
            ScopeMode::Subdomain => self.is_same_subdomain(&parsed),
            ScopeMode::Custom => true, // Already checked regex above
        }
    }

    /// Check if URL has the same host
    fn is_same_host(&self, url: &Url) -> bool {
        url.host_str() == Some(&self.root_hostname)
    }

    /// Check if URL has the same domain
    fn is_same_domain(&self, url: &Url) -> bool {
        let hostname = match url.host_str() {
            Some(h) => h,
            None => return false,
        };

        let domain = extract_domain(hostname);

        if self.scope.include_subdomains {
            // Include subdomains: domain must end with root domain
            hostname == self.root_domain || hostname.ends_with(&format!(".{}", self.root_domain))
        } else {
            // Exact domain match
            domain == self.root_domain
        }
    }

    /// Check if URL is same or subdomain
    fn is_same_subdomain(&self, url: &Url) -> bool {
        let hostname = match url.host_str() {
            Some(h) => h,
            None => return false,
        };

        // Must end with root domain
        hostname == self.root_domain || hostname.ends_with(&format!(".{}", self.root_domain))
    }

    /// Canonicalize a URL for deduplication
    pub fn canonicalize(&self, url: &str) -> String {
        // Parse URL
        let parsed = match Url::parse(url) {
            Ok(u) => u,
            Err(_) => return url.to_string(),
        };

        // Build canonical form
        let mut canonical = String::new();

        // Scheme (lowercase)
        canonical.push_str(&parsed.scheme().to_lowercase());
        canonical.push_str("://");

        // Host (lowercase)
        if let Some(host) = parsed.host_str() {
            canonical.push_str(&host.to_lowercase());
        }

        // Port (if non-standard)
        if let Some(port) = parsed.port() {
            let default_port = match parsed.scheme() {
                "http" => 80,
                "https" => 443,
                _ => 0,
            };
            if port != default_port {
                canonical.push(':');
                canonical.push_str(&port.to_string());
            }
        }

        // Path (lowercase and decode percent-encoded)
        let path = parsed.path();
        if path.is_empty() || path == "/" {
            canonical.push('/');
        } else {
            canonical.push_str(&path.to_lowercase());
        }

        // Remove trailing slash (except for root)
        if canonical.ends_with('/') && canonical.len() > 1 {
            canonical.pop();
        }

        // Remove query parameters (optional - configurable)
        // For now, keep query params for more accurate deduplication
        if let Some(query) = parsed.query() {
            canonical.push('?');
            canonical.push_str(query);
        }

        canonical
    }

    /// Get the root URL
    pub fn root_url(&self) -> &Url {
        &self.root_url
    }

    /// Get the root hostname
    pub fn root_hostname(&self) -> &str {
        &self.root_hostname
    }
}

/// Extract domain from hostname (e.g., "www.example.com" -> "example.com")
fn extract_domain(hostname: &str) -> String {
    let parts: Vec<&str> = hostname.split('.').collect();
    if parts.len() >= 2 {
        parts[parts.len() - 2..].join(".")
    } else {
        hostname.to_string()
    }
}

/// Check if scheme is allowed (http/https)
fn is_allowed_scheme(scheme: &str) -> bool {
    matches!(scheme, "http" | "https")
}

/// Scope-related errors
#[derive(Debug, thiserror::Error)]
pub enum ScopeError {
    #[error("Invalid root URL: {0}")]
    InvalidRootUrl(String),

    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn test_scope_host_mode() {
        let scope = Scope::host();
        let validator = ScopeValidator::new("https://www.example.com", scope).unwrap();

        // Same host
        assert!(validator.is_in_scope("https://www.example.com/about"));
        assert!(validator.is_in_scope("https://www.example.com/page.html"));

        // Different host
        assert!(!validator.is_in_scope("https://api.example.com/page"));
        assert!(!validator.is_in_scope("https://example.com/page"));
        assert!(!validator.is_in_scope("https://other.com/page"));
    }

    #[test]
    fn test_scope_domain_mode_with_subdomains() {
        let mut scope = Scope::domain().with_include_subdomains(true);
        scope.regex_exclude.push(Regex::new(r".*\.png$").unwrap());

        let validator = ScopeValidator::new("https://www.example.com", scope).unwrap();

        // Same domain with subdomains
        assert!(validator.is_in_scope("https://www.example.com/page"));
        assert!(validator.is_in_scope("https://api.example.com/page"));
        assert!(validator.is_in_scope("https://deep.sub.example.com/page"));

        // Different domain
        assert!(!validator.is_in_scope("https://example.org/page"));

        // Excluded by regex
        assert!(!validator.is_in_scope("https://www.example.com/image.png"));
    }

    #[test]
    fn test_canonicalize() {
        let scope = Scope::domain();
        let validator = ScopeValidator::new("https://www.example.com", scope).unwrap();

        // Path normalization
        let canonical = validator.canonicalize("https://www.example.com/page/");
        assert_eq!(canonical, "https://www.example.com/page");

        // Case normalization
        let canonical = validator.canonicalize("HTTPS://WWW.EXAMPLE.COM/PAGE");
        assert_eq!(canonical, "https://www.example.com/page");

        // Query params
        let canonical = validator.canonicalize("https://www.example.com/page?id=123");
        assert_eq!(canonical, "https://www.example.com/page?id=123");
    }
}
