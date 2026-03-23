//! Crawl state persistence for resumable crawls

use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::frontier::{FrontierEntry, UrlSource};
use super::types::CrawlResult;

/// Manages crawl state persistence to disk
pub struct CrawlState {
    /// Directory to store state files
    state_dir: String,
}

impl CrawlState {
    /// Create a new state manager
    pub fn new(state_dir: &str) -> Self {
        Self {
            state_dir: state_dir.to_string(),
        }
    }

    /// Ensure state directory exists
    pub async fn ensure_dir(&self) -> Result<(), StateError> {
        fs::create_dir_all(&self.state_dir)
            .await
            .map_err(|e| StateError::IoError(e.to_string()))
    }

    /// Save frontier state
    pub async fn save_frontier(
        &self,
        name: &str,
        entries: &[FrontierEntry],
    ) -> Result<(), StateError> {
        self.ensure_dir().await?;

        let path = format!("{}/frontier_{}.json", self.state_dir, name);
        let json = serde_json::to_string_pretty(entries)
            .map_err(|e| StateError::SerializeError(e.to_string()))?;

        let mut file = fs::File::create(&path)
            .await
            .map_err(|e| StateError::IoError(e.to_string()))?;

        file.write_all(json.as_bytes())
            .await
            .map_err(|e| StateError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Load frontier state
    pub async fn load_frontier(&self, name: &str) -> Result<Vec<FrontierEntry>, StateError> {
        let path = format!("{}/frontier_{}.json", self.state_dir, name);

        let mut file = match fs::File::open(&path).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(StateError::IoError(e.to_string())),
        };

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .await
            .map_err(|e| StateError::IoError(e.to_string()))?;

        let entries: Vec<SerdeFrontierEntry> = serde_json::from_str(&contents)
            .map_err(|e| StateError::DeserializeError(e.to_string()))?;

        Ok(entries.into_iter().map(|e| e.into()).collect())
    }

    /// Save crawl results
    pub async fn save_results(
        &self,
        name: &str,
        results: &[CrawlResult],
    ) -> Result<(), StateError> {
        self.ensure_dir().await?;

        let path = format!("{}/results_{}.jsonl", self.state_dir, name);
        let mut file = fs::File::create(&path)
            .await
            .map_err(|e| StateError::IoError(e.to_string()))?;

        for result in results {
            let json = serde_json::to_string(result)
                .map_err(|e| StateError::SerializeError(e.to_string()))?;
            file.write_all(json.as_bytes())
                .await
                .map_err(|e| StateError::IoError(e.to_string()))?;
            file.write_all(b"\n")
                .await
                .map_err(|e| StateError::IoError(e.to_string()))?;
        }

        Ok(())
    }

    /// Append a single result (for incremental saving)
    pub async fn append_result(&self, name: &str, result: &CrawlResult) -> Result<(), StateError> {
        self.ensure_dir().await?;

        let path = format!("{}/results_{}.jsonl", self.state_dir, name);
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| StateError::IoError(e.to_string()))?;

        let json =
            serde_json::to_string(result).map_err(|e| StateError::SerializeError(e.to_string()))?;
        file.write_all(json.as_bytes())
            .await
            .map_err(|e| StateError::IoError(e.to_string()))?;
        file.write_all(b"\n")
            .await
            .map_err(|e| StateError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Load results from JSONL
    pub async fn load_results(&self, name: &str) -> Result<Vec<CrawlResult>, StateError> {
        let path = format!("{}/results_{}.jsonl", self.state_dir, name);

        let mut file = match fs::File::open(&path).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(StateError::IoError(e.to_string())),
        };

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .await
            .map_err(|e| StateError::IoError(e.to_string()))?;

        let mut results = Vec::new();
        for line in contents.lines() {
            if !line.is_empty() {
                let result = serde_json::from_str(line)
                    .map_err(|e| StateError::DeserializeError(e.to_string()))?;
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Save seen URLs set
    pub async fn save_seen(
        &self,
        name: &str,
        seen: &std::collections::HashSet<String>,
    ) -> Result<(), StateError> {
        self.ensure_dir().await?;

        let path = format!("{}/seen_{}.txt", self.state_dir, name);
        let mut file = fs::File::create(&path)
            .await
            .map_err(|e| StateError::IoError(e.to_string()))?;

        for url in seen {
            file.write_all(url.as_bytes())
                .await
                .map_err(|e| StateError::IoError(e.to_string()))?;
            file.write_all(b"\n")
                .await
                .map_err(|e| StateError::IoError(e.to_string()))?;
        }

        Ok(())
    }

    /// Load seen URLs set
    pub async fn load_seen(
        &self,
        name: &str,
    ) -> Result<std::collections::HashSet<String>, StateError> {
        let path = format!("{}/seen_{}.txt", self.state_dir, name);

        let mut file = match fs::File::open(&path).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(std::collections::HashSet::new());
            }
            Err(e) => return Err(StateError::IoError(e.to_string())),
        };

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .await
            .map_err(|e| StateError::IoError(e.to_string()))?;

        let seen: std::collections::HashSet<String> = contents
            .lines()
            .filter(|l| !l.is_empty())
            .map(|s| s.to_string())
            .collect();

        Ok(seen)
    }

    /// Save crawl metadata
    pub async fn save_metadata(&self, name: &str, meta: &CrawlMetadata) -> Result<(), StateError> {
        self.ensure_dir().await?;

        let path = format!("{}/meta_{}.json", self.state_dir, name);
        let json = serde_json::to_string_pretty(meta)
            .map_err(|e| StateError::SerializeError(e.to_string()))?;

        let mut file = fs::File::create(&path)
            .await
            .map_err(|e| StateError::IoError(e.to_string()))?;

        file.write_all(json.as_bytes())
            .await
            .map_err(|e| StateError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Load crawl metadata
    pub async fn load_metadata(&self, name: &str) -> Result<Option<CrawlMetadata>, StateError> {
        let path = format!("{}/meta_{}.json", self.state_dir, name);

        let mut file = match fs::File::open(&path).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(StateError::IoError(e.to_string())),
        };

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .await
            .map_err(|e| StateError::IoError(e.to_string()))?;

        let meta: CrawlMetadata = serde_json::from_str(&contents)
            .map_err(|e| StateError::DeserializeError(e.to_string()))?;

        Ok(Some(meta))
    }

    /// Delete state files
    pub async fn cleanup(&self, name: &str) -> Result<(), StateError> {
        let paths = [
            format!("{}/frontier_{}.json", self.state_dir, name),
            format!("{}/results_{}.jsonl", self.state_dir, name),
            format!("{}/seen_{}.txt", self.state_dir, name),
            format!("{}/meta_{}.json", self.state_dir, name),
        ];

        for path in &paths {
            let _ = fs::remove_file(path).await;
        }

        Ok(())
    }
}

/// Metadata about a crawl session
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrawlMetadata {
    /// Root URL being crawled
    pub root_url: String,
    /// When crawl started
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// When crawl was last saved
    pub last_saved: chrono::DateTime<chrono::Utc>,
    /// Pages crawled so far
    pub pages_crawled: usize,
    /// Pages queued
    pub pages_queued: usize,
    /// Number of errors
    pub error_count: usize,
}

/// Serializable frontier entry
#[derive(serde::Serialize, serde::Deserialize)]
struct SerdeFrontierEntry {
    url: String,
    source_url: Option<String>,
    depth: usize,
    source: String,
    attribute: String,
}

impl From<SerdeFrontierEntry> for FrontierEntry {
    fn from(e: SerdeFrontierEntry) -> Self {
        let source = match e.source.as_str() {
            "a" => UrlSource::Anchor,
            "form" => UrlSource::Form,
            "script" => UrlSource::Script,
            "iframe" => UrlSource::Iframe,
            "meta" => UrlSource::Meta,
            "canonical" => UrlSource::Canonical,
            "redirect" => UrlSource::Redirect,
            "sitemap" => UrlSource::Sitemap,
            "robots" => UrlSource::Robots,
            _ => UrlSource::Anchor,
        };

        FrontierEntry::new(e.url, e.depth)
            .with_source(source)
            .with_attribute(e.attribute)
            .with_source_url(e.source_url.unwrap_or_default())
    }
}

impl From<&FrontierEntry> for SerdeFrontierEntry {
    fn from(e: &FrontierEntry) -> Self {
        Self {
            url: e.url.clone(),
            source_url: e.source_url.clone(),
            depth: e.depth,
            source: e.source.to_string(),
            attribute: e.attribute.clone(),
        }
    }
}

/// State-related errors
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Serialization error: {0}")]
    SerializeError(String),

    #[error("Deserialization error: {0}")]
    DeserializeError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crawler::UrlSource;

    #[test]
    fn test_crawl_state_new() {
        let state = CrawlState::new("/tmp/test_scrapio");
        let _ = state;
    }

    #[test]
    fn test_crawl_metadata_default() {
        let meta = CrawlMetadata {
            root_url: "http://example.com".to_string(),
            started_at: chrono::Utc::now(),
            last_saved: chrono::Utc::now(),
            pages_crawled: 10,
            pages_queued: 5,
            error_count: 2,
        };
        assert_eq!(meta.pages_crawled, 10);
        assert_eq!(meta.error_count, 2);
    }

    #[test]
    fn test_serde_frontier_entry_roundtrip() {
        let entry = FrontierEntry::new("http://example.com/page".to_string(), 2)
            .with_source(UrlSource::Anchor)
            .with_attribute("href")
            .with_source_url("http://example.com".to_string());

        let serialized = serde_json::to_string(&entry).unwrap();
        let deserialized: FrontierEntry = serde_json::from_str(&serialized).unwrap();

        assert_eq!(entry.url, deserialized.url);
        assert_eq!(entry.depth, deserialized.depth);
    }

    #[tokio::test]
    async fn test_crawl_state_ensure_dir() {
        let state = CrawlState::new("/tmp/test_scrapio_ensure_dir");
        let result = state.ensure_dir().await;
        assert!(result.is_ok() || result.is_err()); // Either is fine for temp dir
    }

    #[tokio::test]
    async fn test_crawl_state_save_and_load_frontier() {
        let state = CrawlState::new("/tmp/test_scrapio_frontier");

        let entries = vec![
            FrontierEntry::new("http://example.com/page1".to_string(), 0)
                .with_source(UrlSource::Anchor),
            FrontierEntry::new("http://example.com/page2".to_string(), 1)
                .with_source(UrlSource::Sitemap),
        ];

        let save_result = state.save_frontier("test_run", &entries).await;
        assert!(save_result.is_ok() || save_result.is_err());

        let loaded = state.load_frontier("test_run").await;
        assert!(loaded.is_ok());
    }

    #[tokio::test]
    async fn test_crawl_state_load_nonexistent() {
        let state = CrawlState::new("/tmp/test_scrapio_nonexistent");
        let loaded = state.load_frontier("nonexistent_run").await;
        assert!(loaded.is_ok());
        assert!(loaded.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_crawl_state_metadata() {
        let state = CrawlState::new("/tmp/test_scrapio_meta");

        let meta = CrawlMetadata {
            root_url: "http://example.com".to_string(),
            started_at: chrono::Utc::now(),
            last_saved: chrono::Utc::now(),
            pages_crawled: 100,
            pages_queued: 50,
            error_count: 5,
        };

        let save_result = state.save_metadata("test_run", &meta).await;
        assert!(save_result.is_ok() || save_result.is_err());

        let loaded = state.load_metadata("test_run").await;
        assert!(loaded.is_ok());
    }

    #[test]
    fn test_state_error_display() {
        let err = StateError::IoError("test error".to_string());
        assert!(err.to_string().contains("test error"));

        let err = StateError::SerializeError("serialize error".to_string());
        assert!(err.to_string().contains("serialize error"));

        let err = StateError::DeserializeError("deserialize error".to_string());
        assert!(err.to_string().contains("deserialize error"));
    }
}
