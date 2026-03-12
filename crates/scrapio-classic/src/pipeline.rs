//! Item Pipeline - Process and export scraped data
//!
//! Provides a pipeline system for processing items after they're extracted.

use std::fs::File;
use std::io::Write;

use crate::spider::Item;

/// Pipeline trait for processing scraped items
pub trait Pipeline: Send + Sync {
    /// Process a single item
    fn process_item(&self, item: Item) -> Result<Item, PipelineError>;

    /// Called when spider opens
    fn open_spider(&self) -> Result<(), PipelineError> {
        Ok(())
    }

    /// Called when spider closes
    fn close_spider(&self) -> Result<(), PipelineError> {
        Ok(())
    }
}

/// Pipeline errors
#[derive(Debug)]
pub enum PipelineError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Csv(csv::Error),
    Custom(String),
}

impl From<std::io::Error> for PipelineError {
    fn from(e: std::io::Error) -> Self {
        PipelineError::Io(e)
    }
}

impl From<serde_json::Error> for PipelineError {
    fn from(e: serde_json::Error) -> Self {
        PipelineError::Json(e)
    }
}

impl From<csv::Error> for PipelineError {
    fn from(e: csv::Error) -> Self {
        PipelineError::Csv(e)
    }
}

/// JSON Pipeline - exports items to JSON Lines format
pub struct JsonPipeline {
    path: String,
    file: std::sync::Mutex<Option<File>>,
}

impl JsonPipeline {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            file: std::sync::Mutex::new(None),
        }
    }
}

impl Pipeline for JsonPipeline {
    fn open_spider(&self) -> Result<(), PipelineError> {
        let mut file = File::create(&self.path)?;
        file.write_all(b"")?; // Create empty file
        let mut guard = self.file.lock().unwrap();
        *guard = Some(File::create(&self.path)?);
        Ok(())
    }

    fn process_item(&self, item: Item) -> Result<Item, PipelineError> {
        let mut guard = self.file.lock().unwrap();
        if let Some(ref mut file) = *guard {
            let json = serde_json::to_string(&item)?;
            writeln!(file, "{}", json)?;
        }
        Ok(item)
    }

    fn close_spider(&self) -> Result<(), PipelineError> {
        let mut guard = self.file.lock().unwrap();
        *guard = None;
        Ok(())
    }
}

/// CSV Pipeline - exports items to CSV format
pub struct CsvPipeline {
    path: String,
    writer: std::sync::Mutex<Option<csv::Writer<File>>>,
    headers_written: std::sync::Mutex<bool>,
}

impl CsvPipeline {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            writer: std::sync::Mutex::new(None),
            headers_written: std::sync::Mutex::new(false),
        }
    }
}

impl Pipeline for CsvPipeline {
    fn open_spider(&self) -> Result<(), PipelineError> {
        let writer = csv::Writer::from_path(&self.path)?;
        let mut guard = self.writer.lock().unwrap();
        *guard = Some(writer);
        Ok(())
    }

    fn process_item(&self, item: Item) -> Result<Item, PipelineError> {
        let mut guard = self.writer.lock().unwrap();
        if let Some(ref mut writer) = *guard {
            // Get headers from first item
            let headers_written = *self.headers_written.lock().unwrap();
            if !headers_written && !item.is_empty() {
                let headers: Vec<&str> = item.keys().map(|k| k.as_str()).collect();
                writer.write_record(&headers)?;
                *self.headers_written.lock().unwrap() = true;
            }

            // Write record
            let record: Vec<String> = item
                .values()
                .map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .collect();
            writer.write_record(&record)?;
            writer.flush()?;
        }
        Ok(item)
    }

    fn close_spider(&self) -> Result<(), PipelineError> {
        let mut guard = self.writer.lock().unwrap();
        if let Some(ref mut writer) = *guard {
            writer.flush()?;
        }
        *guard = None;
        Ok(())
    }
}

/// Console Pipeline - prints items to console
pub struct ConsolePipeline;

impl ConsolePipeline {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConsolePipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline for ConsolePipeline {
    fn process_item(&self, item: Item) -> Result<Item, PipelineError> {
        println!("{}", serde_json::to_string_pretty(&item)?);
        Ok(item)
    }
}

/// Pipeline chain - runs multiple pipelines in sequence
pub struct PipelineChain {
    pipelines: Vec<Box<dyn Pipeline>>,
}

impl PipelineChain {
    pub fn new() -> Self {
        Self {
            pipelines: Vec::new(),
        }
    }

    pub fn push<P: Pipeline + 'static>(mut self, pipeline: P) -> Self {
        self.pipelines.push(Box::new(pipeline));
        self
    }

    pub fn process(&self, item: Item) -> Result<Item, PipelineError> {
        let mut result = item;
        for pipeline in &self.pipelines {
            result = pipeline.process_item(result)?;
        }
        Ok(result)
    }

    pub fn open_spiders(&self) -> Result<(), PipelineError> {
        for pipeline in &self.pipelines {
            pipeline.open_spider()?;
        }
        Ok(())
    }

    pub fn close_spiders(&self) -> Result<(), PipelineError> {
        for pipeline in &self.pipelines {
            pipeline.close_spider()?;
        }
        Ok(())
    }
}

impl Default for PipelineChain {
    fn default() -> Self {
        Self::new()
    }
}
