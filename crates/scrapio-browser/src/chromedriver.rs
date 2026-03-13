//! ChromeDriver manager for automatic download and management

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// ChromeDriver release channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChromeDriverChannel {
    /// Stable channel (default)
    #[default]
    Stable,
    /// Beta channel
    Beta,
    /// Dev channel
    Dev,
    /// Canary channel
    Canary,
}

impl ChromeDriverChannel {
    pub fn as_str(&self) -> &str {
        match self {
            ChromeDriverChannel::Stable => "stable",
            ChromeDriverChannel::Beta => "beta",
            ChromeDriverChannel::Dev => "dev",
            ChromeDriverChannel::Canary => "canary",
        }
    }
}

/// Operating system type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    Macos,
    Linux,
}

impl Os {
    pub fn current() -> Self {
        #[cfg(target_os = "windows")]
        return Os::Windows;
        #[cfg(target_os = "macos")]
        return Os::Macos;
        #[cfg(target_os = "linux")]
        return Os::Linux;
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        panic!("Unsupported operating system")
    }

    pub fn as_str(&self) -> &str {
        match self {
            Os::Windows => "win32",
            Os::Macos => "mac-x64",
            Os::Linux => "linux64",
        }
    }

    pub fn extension(&self) -> &str {
        match self {
            Os::Windows => "exe",
            Os::Macos | Os::Linux => "",
        }
    }
}

/// Architecture type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    Amd64,
    Arm64,
}

impl Arch {
    pub fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        return Arch::Amd64;
        #[cfg(target_arch = "aarch64")]
        return Arch::Arm64;
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        panic!("Unsupported architecture")
    }

    pub fn as_str(&self) -> &str {
        match self {
            Arch::Amd64 => "amd64",
            Arch::Arm64 => "arm64",
        }
    }
}

/// ChromeDriver version response (new API format)
#[derive(Debug, Clone, Deserialize)]
pub struct LastKnownGoodVersions {
    pub channels: std::collections::HashMap<String, ChannelInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChannelInfo {
    pub channel: String,
    pub version: String,
    pub revision: String,
}

/// ChromeDriver manager for automatic download and management
#[derive(Debug, Clone)]
pub struct ChromeDriverManager {
    channel: ChromeDriverChannel,
    os: Os,
    arch: Arch,
    cache_dir: PathBuf,
    version: Option<String>,
}

impl Default for ChromeDriverManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ChromeDriverManager {
    /// Create a new ChromeDriverManager with default settings
    pub fn new() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("scrapio")
            .join("chromedriver");

        Self {
            channel: ChromeDriverChannel::Stable,
            os: Os::current(),
            arch: Arch::current(),
            cache_dir,
            version: None,
        }
    }

    /// Set the ChromeDriver channel (stable, beta, dev, canary)
    pub fn with_channel(mut self, channel: ChromeDriverChannel) -> Self {
        self.channel = channel;
        self
    }

    /// Set a specific ChromeDriver version (e.g., "146.0.7680.72")
    ///
    /// When a specific version is set, it will be used instead of fetching
    /// the latest version from the API.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set the cache directory
    pub fn with_cache_dir(mut self, path: PathBuf) -> Self {
        self.cache_dir = path;
        self
    }

    /// Get the cache directory path
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Get the ChromeDriver path
    pub fn driver_path(&self) -> PathBuf {
        let platform_dir = self.get_platform_dir();
        let driver_dir = self.cache_dir.join(platform_dir);
        if self.os == Os::Windows {
            driver_dir.join("chromedriver.exe")
        } else {
            driver_dir.join("chromedriver")
        }
    }

    /// Get the platform directory name (same as used in download URL)
    fn get_platform_dir(&self) -> String {
        match self.os {
            Os::Macos => {
                if self.arch == Arch::Arm64 {
                    "chromedriver-mac-arm64".to_string()
                } else {
                    "chromedriver-mac-x64".to_string()
                }
            }
            Os::Windows => "chromedriver-win32".to_string(),
            Os::Linux => "chromedriver-linux64".to_string(),
        }
    }

    /// Get the WebDriver URL (default)
    pub fn webdriver_url(&self) -> String {
        "http://localhost:9515".to_string()
    }

    /// Fetch version info from the JSON API
    async fn fetch_version_info(&self) -> Result<ChannelInfo, ChromeDriverError> {
        let url =
            "https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions.json";

        let response = reqwest::get(url)
            .await
            .map_err(|e| ChromeDriverError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ChromeDriverError::VersionNotFound(
                self.channel.as_str().to_string(),
            ));
        }

        let versions: LastKnownGoodVersions = response
            .json()
            .await
            .map_err(|e| ChromeDriverError::Network(e.to_string()))?;

        // Get the channel name (capitalized)
        let channel_name = match self.channel {
            ChromeDriverChannel::Stable => "Stable",
            ChromeDriverChannel::Beta => "Beta",
            ChromeDriverChannel::Dev => "Dev",
            ChromeDriverChannel::Canary => "Canary",
        };

        // Find matching channel
        versions
            .channels
            .get(channel_name)
            .cloned()
            .ok_or_else(|| ChromeDriverError::VersionNotFound(channel_name.to_string()))
    }

    /// Fetch the ChromeDriver version from the API
    pub async fn fetch_version(&mut self) -> Result<String, ChromeDriverError> {
        let info = self.fetch_version_info().await?;
        self.version = Some(info.version.clone());
        Ok(info.version)
    }

    /// Get the download URL
    pub fn get_download_url(&self, version: &str) -> String {
        // Chrome for Testing uses specific platform names:
        // - win32 (Windows)
        // - mac-x64 (macOS Intel)
        // - mac-arm64 (macOS Apple Silicon)
        // - linux64 (Linux)
        let platform = match self.os {
            Os::Macos => {
                if self.arch == Arch::Arm64 {
                    "mac-arm64".to_string()
                } else {
                    "mac-x64".to_string()
                }
            }
            Os::Windows => "win32".to_string(),
            Os::Linux => "linux64".to_string(),
        };
        format!(
            "https://storage.googleapis.com/chrome-for-testing-public/{}/{}/chromedriver-{}.zip",
            version, platform, platform
        )
    }

    /// Download and extract ChromeDriver
    pub async fn download(&mut self) -> Result<PathBuf, ChromeDriverError> {
        // Create cache directory
        std::fs::create_dir_all(&self.cache_dir)
            .map_err(|e| ChromeDriverError::Io(e.to_string()))?;

        // Determine version and download URL
        let (version, download_url) = if let Some(ref ver) = self.version {
            // Use manually specified version
            println!("Using manually specified version: {}", ver);
            let url = self.get_download_url(ver);
            (ver.clone(), url)
        } else {
            // Fetch version info from API
            let channel_info = self.fetch_version_info().await?;
            let version = channel_info.version.clone();
            let url = self.get_download_url(&version);
            (version, url)
        };

        self.version = Some(version.clone());

        // Check if already downloaded
        if self.driver_path().exists() {
            println!("ChromeDriver already exists at {:?}", self.driver_path());
            return Ok(self.driver_path());
        }

        println!("Downloading ChromeDriver {} from {}", version, download_url);

        // Download the zip file
        let response = reqwest::get(&download_url)
            .await
            .map_err(|e| ChromeDriverError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ChromeDriverError::DownloadFailed(
                response.status().to_string(),
            ));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| ChromeDriverError::Network(e.to_string()))?;

        // Extract the zip
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| ChromeDriverError::Extraction(e.to_string()))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| ChromeDriverError::Extraction(e.to_string()))?;

            let outpath = match file.enclosed_name() {
                Some(path) => self.cache_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)
                    .map_err(|e| ChromeDriverError::Io(e.to_string()))?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| ChromeDriverError::Io(e.to_string()))?;
                }
                let mut outfile = std::fs::File::create(&outpath)
                    .map_err(|e| ChromeDriverError::Io(e.to_string()))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| ChromeDriverError::Io(e.to_string()))?;
            }

            // Set executable permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode)).ok();
                }
            }
        }

        println!(
            "ChromeDriver downloaded and extracted to {:?}",
            self.driver_path()
        );
        Ok(self.driver_path())
    }

    /// Download if not exists and return the path
    pub async fn ensure(&mut self) -> Result<PathBuf, ChromeDriverError> {
        if self.driver_path().exists() {
            Ok(self.driver_path())
        } else {
            self.download().await
        }
    }

    /// Start ChromeDriver as a child process
    pub fn start(&self, port: u16) -> Result<std::process::Child, ChromeDriverError> {
        let driver_path = self.driver_path();

        if !driver_path.exists() {
            return Err(ChromeDriverError::NotFound(
                "ChromeDriver not found. Call download() first.".to_string(),
            ));
        }

        println!("Starting ChromeDriver on port {}...", port);

        let child = std::process::Command::new(driver_path)
            .arg(format!("--port={}", port))
            .spawn()
            .map_err(|e| ChromeDriverError::Io(e.to_string()))?;

        Ok(child)
    }

    /// Stop a running ChromeDriver process
    pub fn stop(mut child: std::process::Child) {
        println!("Stopping ChromeDriver...");
        let _ = child.kill();
    }

    /// Kill any existing ChromeDriver on the given port
    pub fn kill_existing(port: u16) {
        use std::process::Command;
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("pkill")
                .arg("-f")
                .arg(format!("chromedriver.*port={}", port))
                .output();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("pkill")
                .arg("-f")
                .arg(format!("chromedriver.*port={}", port))
                .output();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("taskkill")
                .arg("/F")
                .arg("/IM")
                .arg("chromedriver.exe")
                .output();
        }
    }

    /// Download and start ChromeDriver
    pub async fn download_and_start(
        &mut self,
        port: u16,
    ) -> Result<std::process::Child, ChromeDriverError> {
        self.ensure().await?;
        self.start(port)
    }

    /// Get installed ChromeDriver version
    pub fn installed_version(&self) -> Option<String> {
        if self.driver_path().exists() {
            self.version.clone()
        } else {
            None
        }
    }
}

/// Error type for ChromeDriver operations
#[derive(Debug)]
pub enum ChromeDriverError {
    Network(String),
    VersionNotFound(String),
    DownloadFailed(String),
    Extraction(String),
    Io(String),
    NotFound(String),
}

impl std::fmt::Display for ChromeDriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChromeDriverError::Network(e) => write!(f, "Network error: {}", e),
            ChromeDriverError::VersionNotFound(v) => {
                write!(f, "Version not found for channel: {}", v)
            }
            ChromeDriverError::DownloadFailed(s) => write!(f, "Download failed: {}", s),
            ChromeDriverError::Extraction(e) => write!(f, "Extraction error: {}", e),
            ChromeDriverError::Io(e) => write!(f, "IO error: {}", e),
            ChromeDriverError::NotFound(e) => write!(f, "Not found: {}", e),
        }
    }
}

impl std::error::Error for ChromeDriverError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_detection() {
        let os = Os::current();
        match os {
            Os::Windows => assert_eq!(os.as_str(), "win32"),
            Os::Macos => assert_eq!(os.as_str(), "mac-x64"),
            Os::Linux => assert_eq!(os.as_str(), "linux64"),
        }
    }

    #[test]
    fn test_arch_detection() {
        let arch = Arch::current();
        match arch {
            Arch::Amd64 => assert_eq!(arch.as_str(), "amd64"),
            Arch::Arm64 => assert_eq!(arch.as_str(), "arm64"),
        }
    }

    #[test]
    fn test_manager_default() {
        let manager = ChromeDriverManager::new();
        assert_eq!(manager.channel, ChromeDriverChannel::Stable);
    }

    #[test]
    fn test_manager_with_channel() {
        let manager = ChromeDriverManager::new().with_channel(ChromeDriverChannel::Beta);
        assert_eq!(manager.channel, ChromeDriverChannel::Beta);
    }
}
