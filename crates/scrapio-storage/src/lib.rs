//! Storage layer for Scrapio using SQLite

use std::path::Path;

use scrapio_core::error::ScrapioError;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

/// Database storage for crawl results
pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    /// Create a new storage instance
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self, ScrapioError> {
        let path = db_path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(ScrapioError::Io)?;
        }

        // Use SQLite with direct file path
        // For absolute path /tmp/scrapio.db -> just /tmp/scrapio.db
        let url = format!("sqlite:{}?mode=rwc", path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .map_err(|e| ScrapioError::Storage(e.to_string()))?;

        // Initialize tables
        Self::init_tables(&pool).await?;

        Ok(Self { pool })
    }

    async fn init_tables(pool: &SqlitePool) -> Result<(), ScrapioError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS crawl_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                status INTEGER NOT NULL,
                title TEXT,
                content TEXT,
                links TEXT,
                crawled_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(url)
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ScrapioError::Storage(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS crawl_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                spider_name TEXT NOT NULL,
                started_at DATETIME NOT NULL,
                finished_at DATETIME,
                items_count INTEGER DEFAULT 0,
                errors_count INTEGER DEFAULT 0,
                status TEXT DEFAULT 'running'
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ScrapioError::Storage(e.to_string()))?;

        Ok(())
    }

    /// Save a crawl result
    pub async fn save_result(
        &self,
        url: &str,
        status: u16,
        title: Option<&str>,
        content: &str,
        links: &[String],
    ) -> Result<i64, ScrapioError> {
        let result = sqlx::query(
            r#"
            INSERT INTO crawl_results (url, status, title, content, links)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(url) DO UPDATE SET
                status = excluded.status,
                title = excluded.title,
                content = excluded.content,
                links = excluded.links,
                crawled_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(url)
        .bind(status)
        .bind(title)
        .bind(content)
        .bind(serde_json::to_string(links).unwrap_or_default())
        .execute(&self.pool)
        .await
        .map_err(|e| ScrapioError::Storage(e.to_string()))?;

        Ok(result.last_insert_rowid())
    }

    /// Get crawl result by URL
    pub async fn get_result(&self, url: &str) -> Result<Option<CrawlResult>, ScrapioError> {
        let row = sqlx::query(
            "SELECT id, url, status, title, content, links, crawled_at FROM crawl_results WHERE url = ?"
        )
        .bind(url)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ScrapioError::Storage(e.to_string()))?;

        Ok(row.map(|r| row_to_crawl_result(&r)))
    }

    /// Get crawl result by ID
    pub async fn get_result_by_id(&self, id: i64) -> Result<Option<CrawlResult>, ScrapioError> {
        let row = sqlx::query(
            "SELECT id, url, status, title, content, links, crawled_at FROM crawl_results WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ScrapioError::Storage(e.to_string()))?;

        Ok(row.map(|r| row_to_crawl_result(&r)))
    }

    /// Get all crawl results
    pub async fn get_all_results(&self, limit: usize) -> Result<Vec<CrawlResult>, ScrapioError> {
        let rows = sqlx::query(
            "SELECT id, url, status, title, content, links, crawled_at FROM crawl_results ORDER BY crawled_at DESC LIMIT ?"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ScrapioError::Storage(e.to_string()))?;

        Ok(rows.iter().map(row_to_crawl_result).collect())
    }

    /// Record a crawl start
    pub async fn record_crawl_start(&self, spider_name: &str) -> Result<i64, ScrapioError> {
        let result = sqlx::query(
            "INSERT INTO crawl_history (spider_name, started_at, status) VALUES (?, datetime('now'), 'running')"
        )
        .bind(spider_name)
        .execute(&self.pool)
        .await
        .map_err(|e| ScrapioError::Storage(e.to_string()))?;

        Ok(result.last_insert_rowid())
    }

    /// Record a crawl end
    pub async fn record_crawl_end(
        &self,
        id: i64,
        items_count: i64,
        errors_count: i64,
        status: &str,
    ) -> Result<(), ScrapioError> {
        sqlx::query(
            "UPDATE crawl_history SET finished_at = datetime('now'), items_count = ?, errors_count = ?, status = ? WHERE id = ?"
        )
        .bind(items_count)
        .bind(errors_count)
        .bind(status)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| ScrapioError::Storage(e.to_string()))?;

        Ok(())
    }
}

fn row_to_crawl_result(row: &sqlx::sqlite::SqliteRow) -> CrawlResult {
    CrawlResult {
        id: row.get("id"),
        url: row.get("url"),
        status: row.get("status"),
        title: row.get("title"),
        content: row.get("content"),
        links: serde_json::from_str(&row.get::<String, _>("links")).unwrap_or_default(),
        crawled_at: row.get("crawled_at"),
    }
}

/// A stored crawl result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlResult {
    pub id: i64,
    pub url: String,
    pub status: u16,
    pub title: Option<String>,
    pub content: String,
    pub links: Vec<String>,
    pub crawled_at: String,
}
