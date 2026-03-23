//! Generic WebDriver manager for browser automation
//!
//! This module provides a generic driver management system that supports
//! multiple browser drivers: ChromeDriver, GeckoDriver (Firefox), and EdgeDriver.

use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Child;
use std::time::{Duration, Instant};

/// Driver type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DriverType {
    /// ChromeDriver (default)
    #[default]
    Chrome,
    /// GeckoDriver (Firefox)
    Firefox,
    /// EdgeDriver
    Edge,
}

impl DriverType {
    /// Get the default port for this driver type
    pub fn default_port(&self) -> u16 {
        match self {
            DriverType::Chrome => 9515,
            DriverType::Firefox => 4444,
            DriverType::Edge => 9516,
        }
    }

    /// Get the cache directory name for this driver
    pub fn cache_dir_name(&self) -> &str {
        match self {
            DriverType::Chrome => "chromedriver",
            DriverType::Firefox => "geckodriver",
            DriverType::Edge => "edgedriver",
        }
    }

    /// Get the driver filename (without extension)
    pub fn driver_filename(&self) -> &str {
        match self {
            DriverType::Chrome => "chromedriver",
            DriverType::Firefox => "geckodriver",
            DriverType::Edge => "msedgedriver",
        }
    }

    /// Get the browser name this driver controls
    pub fn browser_name(&self) -> &str {
        match self {
            DriverType::Chrome => "chrome",
            DriverType::Firefox => "firefox",
            DriverType::Edge => "msedge",
        }
    }

    /// Parse from string (case-insensitive)
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "chrome" | "chromedriver" => Some(DriverType::Chrome),
            "firefox" | "gecko" | "geckodriver" => Some(DriverType::Firefox),
            "edge" | "msedge" | "edgedriver" => Some(DriverType::Edge),
            _ => None,
        }
    }
}

impl std::fmt::Display for DriverType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriverType::Chrome => write!(f, "chromedriver"),
            DriverType::Firefox => write!(f, "geckodriver"),
            DriverType::Edge => write!(f, "edgedriver"),
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
            Os::Macos => "macos",
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

/// Driver release channel (for ChromeDriver)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DriverChannel {
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

impl DriverChannel {
    pub fn as_str(&self) -> &str {
        match self {
            DriverChannel::Stable => "stable",
            DriverChannel::Beta => "beta",
            DriverChannel::Dev => "dev",
            DriverChannel::Canary => "canary",
        }
    }
}

/// Generic WebDriver manager
#[derive(Debug, Clone)]
pub struct DriverManager {
    driver_type: DriverType,
    channel: DriverChannel,
    os: Os,
    arch: Arch,
    cache_dir: PathBuf,
    version: Option<String>,
    /// Whether to apply stealth patches to the binary (Chrome only)
    patch_stealth: bool,
    /// Custom driver path (skip download if set)
    custom_path: Option<PathBuf>,
}

impl DriverManager {
    /// Create a new DriverManager with default settings for Chrome
    pub fn new() -> Self {
        Self::with_driver_type(DriverType::Chrome)
    }

    /// Create a new DriverManager with explicit driver type
    pub fn with_driver_type(driver_type: DriverType) -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("scrapio")
            .join(driver_type.cache_dir_name());

        Self {
            driver_type,
            channel: DriverChannel::Stable,
            os: Os::current(),
            arch: Arch::current(),
            cache_dir,
            version: None,
            patch_stealth: false,
            custom_path: None,
        }
    }

    /// Create a DriverManager for Chrome (backward compatibility)
    pub fn chrome() -> Self {
        Self::with_driver_type(DriverType::Chrome)
    }

    /// Create a DriverManager for Firefox
    pub fn firefox() -> Self {
        Self::with_driver_type(DriverType::Firefox)
    }

    /// Create a DriverManager for Edge
    pub fn edge() -> Self {
        Self::with_driver_type(DriverType::Edge)
    }

    /// Get the driver type
    pub fn driver_type(&self) -> DriverType {
        self.driver_type
    }

    /// Use a custom driver path instead of downloading
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.custom_path = Some(path);
        self
    }

    /// Set the driver channel (stable, beta, dev, canary) - Chrome only
    pub fn with_channel(mut self, channel: DriverChannel) -> Self {
        self.channel = channel;
        self
    }

    /// Set a specific driver version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set the cache directory
    pub fn with_cache_dir(mut self, path: PathBuf) -> Self {
        self.cache_dir = path;
        self
    }

    /// Enable or disable stealth patching (Chrome only, default: false)
    pub fn with_stealth_patching(mut self, enabled: bool) -> Self {
        self.patch_stealth = enabled;
        self
    }

    /// Get the driver path
    pub fn driver_path(&self) -> PathBuf {
        if let Some(ref path) = self.custom_path {
            return path.clone();
        }

        let driver_dir = self.cache_dir.join(self.get_platform_dir());
        let filename = self.driver_type.driver_filename();

        if self.os == Os::Windows {
            driver_dir.join(format!("{}.exe", filename))
        } else {
            driver_dir.join(filename)
        }
    }

    /// Get the cache directory path
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Get the platform directory name
    fn get_platform_dir(&self) -> String {
        let platform = match self.os {
            Os::Macos => {
                if self.arch == Arch::Arm64 {
                    match self.driver_type {
                        DriverType::Chrome => "chromedriver-mac-arm64",
                        DriverType::Firefox => "geckodriver-macos-aarch64",
                        DriverType::Edge => "edgedriver-mac-arm64",
                    }
                } else {
                    match self.driver_type {
                        DriverType::Chrome => "chromedriver-mac-x64",
                        DriverType::Firefox => "geckodriver-macos",
                        DriverType::Edge => "edgedriver-mac-x64",
                    }
                }
            }
            Os::Windows => match self.driver_type {
                DriverType::Chrome => "chromedriver-win32",
                DriverType::Firefox => "geckodriver-win64",
                DriverType::Edge => "edgedriver-win64",
            },
            Os::Linux => match self.driver_type {
                DriverType::Chrome => "chromedriver-linux64",
                DriverType::Firefox => "geckodriver-linux64",
                DriverType::Edge => "edgedriver-linux64",
            },
        };

        platform.to_string()
    }

    /// Get the download URL for the driver
    pub fn get_download_url(&self, version: &str) -> String {
        let platform = match self.os {
            Os::Macos => {
                if self.arch == Arch::Arm64 {
                    "mac-arm64"
                } else {
                    "mac-x64"
                }
            }
            Os::Windows => "win64",
            Os::Linux => "linux64",
        };

        match self.driver_type {
            DriverType::Chrome => {
                format!(
                    "https://storage.googleapis.com/chrome-for-testing-public/{}/{}/chromedriver-{}.zip",
                    version, platform, platform
                )
            }
            DriverType::Firefox => {
                let filename = if self.os == Os::Windows {
                    "geckodriver-{}-win64.zip"
                } else {
                    "geckodriver-{}-{}.tar.gz"
                };
                let archive = if self.os == Os::Windows {
                    filename.replace("{}", version)
                } else {
                    filename.replace("{}", version).replace("{}", platform)
                };
                format!(
                    "https://github.com/mozilla/geckodriver/releases/download/v{}/{}",
                    version, archive
                )
            }
            DriverType::Edge => {
                let platform = if self.os == Os::Windows {
                    "win64"
                } else {
                    platform
                };
                format!(
                    "https://msedgedriver.azureedge.net/{}/edgedriver_{}.zip",
                    version, platform
                )
            }
        }
    }

    /// Get the WebDriver URL (default)
    pub fn webdriver_url(&self) -> String {
        format!("http://localhost:{}", self.driver_type.default_port())
    }

    /// Download and extract the driver
    pub async fn download(&mut self) -> Result<PathBuf, DriverError> {
        // Use custom path if provided
        if let Some(ref path) = self.custom_path {
            if path.exists() {
                return Ok(path.clone());
            }
            return Err(DriverError::NotFound(format!(
                "Custom driver path does not exist: {:?}",
                path
            )));
        }

        // Create cache directory
        std::fs::create_dir_all(&self.cache_dir).map_err(|e| DriverError::Io(e.to_string()))?;

        // Determine version and download URL
        let (version, download_url) = if let Some(ref ver) = self.version {
            (ver.clone(), self.get_download_url(ver))
        } else {
            let ver = self.fetch_latest_version().await?;
            (ver.clone(), self.get_download_url(&ver))
        };

        self.version = Some(version.clone());

        // Check if already downloaded
        if self.driver_path().exists() {
            println!(
                "{} already exists at {:?}",
                self.driver_type,
                self.driver_path()
            );
            return Ok(self.driver_path());
        }

        println!(
            "Downloading {} v{} from {}",
            self.driver_type, version, download_url
        );

        // Download based on driver type
        match self.driver_type {
            DriverType::Chrome => self.download_chromedriver(&download_url).await,
            DriverType::Firefox => self.download_geckodriver(&download_url).await,
            DriverType::Edge => self.download_edgedriver(&download_url).await,
        }
    }

    async fn download_chromedriver(&mut self, url: &str) -> Result<PathBuf, DriverError> {
        let bytes = download_with_retry(url).await?;

        // Extract the zip
        let cursor = std::io::Cursor::new(bytes);
        let mut archive =
            zip::ZipArchive::new(cursor).map_err(|e| DriverError::Extraction(e.to_string()))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| DriverError::Extraction(e.to_string()))?;

            let outpath = match file.enclosed_name() {
                Some(path) => self.cache_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath).map_err(|e| DriverError::Io(e.to_string()))?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| DriverError::Io(e.to_string()))?;
                }
                let mut outfile =
                    std::fs::File::create(&outpath).map_err(|e| DriverError::Io(e.to_string()))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| DriverError::Io(e.to_string()))?;
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode)).ok();
                }
            }
        }

        // Apply stealth patches if enabled
        if self.patch_stealth {
            let _ = apply_chromedriver_patches(&self.driver_path());
        }

        println!("{} extracted to {:?}", self.driver_type, self.driver_path());
        Ok(self.driver_path())
    }

    async fn download_geckodriver(&mut self, url: &str) -> Result<PathBuf, DriverError> {
        // For Firefox, we download from GitHub releases
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| DriverError::Network(e.to_string()))?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| DriverError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DriverError::DownloadFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| DriverError::Network(e.to_string()))?;

        // Determine if it's a zip or tar.gz
        let cursor = std::io::Cursor::new(&bytes);

        // Try to extract as zip first
        if let Ok(mut archive) = zip::ZipArchive::new(cursor) {
            for i in 0..archive.len() {
                let mut file = archive
                    .by_index(i)
                    .map_err(|e| DriverError::Extraction(e.to_string()))?;

                let outpath = match file.enclosed_name() {
                    Some(path) => {
                        // Extract only the driver binary
                        if path
                            .file_name()
                            .is_some_and(|f| f.to_string_lossy().starts_with("geckodriver"))
                        {
                            self.cache_dir.join(path.file_name().unwrap())
                        } else {
                            continue;
                        }
                    }
                    None => continue,
                };

                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| DriverError::Io(e.to_string()))?;
                }

                let mut outfile =
                    std::fs::File::create(&outpath).map_err(|e| DriverError::Io(e.to_string()))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| DriverError::Io(e.to_string()))?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(0o755)).ok();
                }
            }
        }

        println!("{} extracted to {:?}", self.driver_type, self.driver_path());
        Ok(self.driver_path())
    }

    async fn download_edgedriver(&mut self, url: &str) -> Result<PathBuf, DriverError> {
        // EdgeDriver download similar to ChromeDriver
        let bytes = download_with_retry(url).await?;

        let cursor = std::io::Cursor::new(bytes);
        let mut archive =
            zip::ZipArchive::new(cursor).map_err(|e| DriverError::Extraction(e.to_string()))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| DriverError::Extraction(e.to_string()))?;

            let outpath = match file.enclosed_name() {
                Some(path) => self.cache_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath).map_err(|e| DriverError::Io(e.to_string()))?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| DriverError::Io(e.to_string()))?;
                }
                let mut outfile =
                    std::fs::File::create(&outpath).map_err(|e| DriverError::Io(e.to_string()))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| DriverError::Io(e.to_string()))?;
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode)).ok();
                }
            }
        }

        println!("{} extracted to {:?}", self.driver_type, self.driver_path());
        Ok(self.driver_path())
    }

    /// Fetch the latest version from the appropriate API
    async fn fetch_latest_version(&self) -> Result<String, DriverError> {
        match self.driver_type {
            DriverType::Chrome => self.fetch_chrome_version().await,
            DriverType::Firefox => self.fetch_firefox_version().await,
            DriverType::Edge => self.fetch_edge_version().await,
        }
    }

    async fn fetch_chrome_version(&self) -> Result<String, DriverError> {
        let url =
            "https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions.json";

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| DriverError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DriverError::VersionNotFound(
                self.channel.as_str().to_string(),
            ));
        }

        #[derive(Deserialize)]
        struct LastKnownGoodVersions {
            channels: std::collections::HashMap<String, ChannelInfo>,
        }

        #[derive(Deserialize, Clone)]
        #[allow(dead_code)]
        struct ChannelInfo {
            channel: String,
            version: String,
            revision: String,
        }

        let versions: LastKnownGoodVersions = response
            .json()
            .await
            .map_err(|e| DriverError::Network(e.to_string()))?;

        let channel_name = match self.channel {
            DriverChannel::Stable => "Stable",
            DriverChannel::Beta => "Beta",
            DriverChannel::Dev => "Dev",
            DriverChannel::Canary => "Canary",
        };

        versions
            .channels
            .get(channel_name)
            .cloned()
            .ok_or_else(|| DriverError::VersionNotFound(channel_name.to_string()))
            .map(|v| v.version)
    }

    async fn fetch_firefox_version(&self) -> Result<String, DriverError> {
        // For Firefox, we query GitHub releases API for the latest version
        let url = "https://api.github.com/repos/mozilla/geckodriver/releases/latest";

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("User-Agent", "scrapio")
            .send()
            .await
            .map_err(|e| DriverError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DriverError::VersionNotFound("firefox".to_string()));
        }

        #[derive(Deserialize)]
        struct GitHubRelease {
            tag_name: String,
        }

        let release: GitHubRelease = response
            .json()
            .await
            .map_err(|e| DriverError::Network(e.to_string()))?;

        // Remove 'v' prefix from tag_name
        Ok(release.tag_name.trim_start_matches('v').to_string())
    }

    async fn fetch_edge_version(&self) -> Result<String, DriverError> {
        // Edge version can be fetched from the Edge JSON API
        let url = "https://msedgedriver.azureedge.com/EDGEDRIVER.json";

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| DriverError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DriverError::VersionNotFound("edge".to_string()));
        }

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct EdgeDriverInfo {
            #[serde(rename = "OS")]
            os: String,
            #[serde(rename = "SKU")]
            sku: String,
            #[serde(rename = "BrowserVersion")]
            browser_version: String,
            #[serde(rename = "WebDriverVersion")]
            webdriver_version: String,
        }

        let drivers: Vec<EdgeDriverInfo> = response
            .json()
            .await
            .map_err(|e| DriverError::Network(e.to_string()))?;

        // Find matching driver for current OS
        let os_suffix = match self.os {
            Os::Windows => "win64",
            Os::Macos => "macos",
            Os::Linux => "linux64",
        };

        drivers
            .into_iter()
            .find(|d| d.os.contains(os_suffix))
            .map(|d| d.webdriver_version)
            .ok_or_else(|| DriverError::VersionNotFound("edge".to_string()))
    }

    /// Download if not exists and return the path
    pub async fn ensure(&mut self) -> Result<PathBuf, DriverError> {
        if self.driver_path().exists() {
            Ok(self.driver_path())
        } else {
            self.download().await
        }
    }

    /// Force re-download the driver
    pub async fn force_download(&mut self) -> Result<PathBuf, DriverError> {
        let path = self.driver_path();
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| DriverError::Io(e.to_string()))?;
        }
        self.download().await
    }

    /// Start the driver as a child process
    pub fn start(&self, port: u16) -> Result<Child, DriverError> {
        let driver_path = self.driver_path();

        if !driver_path.exists() {
            return Err(DriverError::NotFound(
                "Driver not found. Call download() first.".to_string(),
            ));
        }

        println!("Starting {} on port {}...", self.driver_type, port);

        let mut cmd = std::process::Command::new(driver_path);
        cmd.arg(format!("--port={}", port));

        // Add browser-specific args
        if self.driver_type == DriverType::Firefox {
            // Firefox driver doesn't accept --port in same format
            cmd.arg("--port").arg(port.to_string());
        }

        let child = cmd.spawn().map_err(|e| DriverError::Io(e.to_string()))?;

        Ok(child)
    }

    /// Stop a running driver process
    pub fn stop(mut child: Child) {
        println!("Stopping {}...", child.id());
        let _ = child.kill();
    }

    /// Kill any existing driver on the given port
    pub fn kill_existing(port: u16) {
        use std::process::Command;

        let driver_names = ["chromedriver", "geckodriver", "msedgedriver"];

        #[cfg(target_os = "macos")]
        {
            for name in &driver_names {
                let _ = Command::new("pkill")
                    .arg("-f")
                    .arg(format!("{}.*port={}", name, port))
                    .output();
            }
        }
        #[cfg(target_os = "linux")]
        {
            for name in &driver_names {
                let _ = Command::new("pkill")
                    .arg("-f")
                    .arg(format!("{}.*port={}", name, port))
                    .output();
            }
        }
        #[cfg(target_os = "windows")]
        {
            for name in &driver_names {
                let _ = Command::new("taskkill")
                    .arg("/F")
                    .arg("/IM")
                    .arg(format!("{}.exe", name))
                    .output();
            }
        }
    }

    /// Download and start the driver
    pub async fn download_and_start(&mut self, port: u16) -> Result<Child, DriverError> {
        self.ensure().await?;
        self.start(port)
    }

    /// Get installed driver version
    pub fn installed_version(&self) -> Option<String> {
        if self.driver_path().exists() {
            self.version.clone()
        } else {
            None
        }
    }
}

impl Default for DriverManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Managed WebDriver session that starts on an ephemeral port and stops on drop.
#[derive(Debug)]
pub struct WebDriverSession {
    child: Option<Child>,
    port: u16,
    driver_type: DriverType,
}

impl WebDriverSession {
    /// Start a managed session with default Chrome driver settings
    pub async fn start() -> Result<Self, DriverError> {
        Self::start_with(DriverManager::new()).await
    }

    /// Start a managed session with Firefox driver
    pub async fn start_firefox() -> Result<Self, DriverError> {
        Self::start_with(DriverManager::firefox()).await
    }

    /// Start a managed session with Edge driver
    pub async fn start_edge() -> Result<Self, DriverError> {
        Self::start_with(DriverManager::edge()).await
    }

    /// Start a managed session with explicit driver manager
    pub async fn start_with(mut manager: DriverManager) -> Result<Self, DriverError> {
        let port = find_available_port()?;
        let driver_type = manager.driver_type();
        let mut child = manager.download_and_start(port).await?;

        if let Err(err) = wait_for_driver(port, &mut child, driver_type).await {
            let _ = child.kill();
            return Err(err);
        }

        Ok(Self {
            child: Some(child),
            port,
            driver_type,
        })
    }

    /// Get the WebDriver URL for this session
    pub fn webdriver_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    /// Get the port this session is running on
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get the driver type
    pub fn driver_type(&self) -> DriverType {
        self.driver_type
    }

    /// Stop the managed driver process explicitly
    pub fn stop(mut self) {
        if let Some(child) = self.child.take() {
            DriverManager::stop(child);
        }
    }
}

impl Drop for WebDriverSession {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}

/// Alias for backward compatibility
pub type ChromeDriverSession = WebDriverSession;
/// Alias for backward compatibility
pub type ChromeDriverManager = DriverManager;

// Keep old types for backward compatibility
pub use self::chromedriver::{
    ChromeDriverChannel, ChromeDriverError, ChromeDriverManager as OldChromeDriverManager,
    ChromeDriverSession as OldChromeDriverSession,
};

/// Internal module with backward compatible types
pub mod chromedriver {
    use super::*;

    /// ChromeDriver release channel (kept for backward compatibility)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum ChromeDriverChannel {
        #[default]
        Stable,
        Beta,
        Dev,
        Canary,
    }

    impl From<ChromeDriverChannel> for DriverChannel {
        fn from(c: ChromeDriverChannel) -> Self {
            match c {
                ChromeDriverChannel::Stable => DriverChannel::Stable,
                ChromeDriverChannel::Beta => DriverChannel::Beta,
                ChromeDriverChannel::Dev => DriverChannel::Dev,
                ChromeDriverChannel::Canary => DriverChannel::Canary,
            }
        }
    }

    /// Backward compatible ChromeDriverManager
    pub type ChromeDriverManager = DriverManager;

    /// Backward compatible ChromeDriverSession
    pub type ChromeDriverSession = WebDriverSession;

    /// Error type for ChromeDriver operations (kept for backward compatibility)
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

    impl From<DriverError> for ChromeDriverError {
        fn from(e: DriverError) -> Self {
            match e {
                DriverError::Network(s) => ChromeDriverError::Network(s),
                DriverError::VersionNotFound(s) => ChromeDriverError::VersionNotFound(s),
                DriverError::DownloadFailed(s) => ChromeDriverError::DownloadFailed(s),
                DriverError::Extraction(s) => ChromeDriverError::Extraction(s),
                DriverError::Io(s) => ChromeDriverError::Io(s),
                DriverError::NotFound(s) => ChromeDriverError::NotFound(s),
            }
        }
    }
}

fn find_available_port() -> Result<u16, DriverError> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| DriverError::Io(e.to_string()))?;
    listener
        .local_addr()
        .map(|addr| addr.port())
        .map_err(|e| DriverError::Io(e.to_string()))
}

async fn wait_for_driver(
    port: u16,
    child: &mut Child,
    driver_type: DriverType,
) -> Result<(), DriverError> {
    let addr = format!("127.0.0.1:{}", port);
    let deadline = Instant::now() + Duration::from_secs(10);

    while Instant::now() < deadline {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| DriverError::Io(e.to_string()))?
        {
            return Err(DriverError::NotFound(format!(
                "{} exited before accepting connections: {}",
                driver_type, status
            )));
        }

        if tokio::net::TcpStream::connect(&addr).await.is_ok() {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Err(DriverError::NotFound(format!(
        "{} did not become ready on port {}",
        driver_type, port
    )))
}

/// Apply stealth patches to ChromeDriver binary
fn apply_chromedriver_patches(path: &std::path::Path) -> Result<usize, DriverError> {
    if !path.exists() {
        return Err(DriverError::NotFound(
            "ChromeDriver binary not found".to_string(),
        ));
    }

    let mut data = std::fs::read(path).map_err(|e| DriverError::Io(e.to_string()))?;

    let patches: Vec<(&[u8], &[u8])> = vec![
        (
            b"cdc_adoQpoasnfa76pfcZLmcfl_",
            b"jse_evaluate_0000_abcdefghi",
        ),
        (
            b"$cdc_adoQpoasnfa76pfcZLmcfl_A",
            b"$jse_evaluat_0000_Abcdefgh",
        ),
        (
            b"$cdc_adoQpoasnfa76pfcZLmcfl_D",
            b"$jse_evaluat_0000_Abcdefghi",
        ),
        (
            b"$cdc_adoQpoasnfa76pfcZLmcfl_E",
            b"$jse_evaluat_0000_Abcdefgh",
        ),
        (b"cdc_adoQpoasnfa76pfcZLmcfl", b"jse_evaluate_0000abcdef"),
    ];

    let mut patches_applied = 0;

    for (pattern, replacement) in patches {
        if pattern.len() != replacement.len() {
            continue;
        }

        let count = replace_all_bytes(&mut data, pattern, replacement);
        if count > 0 {
            tracing::debug!(
                "Applied patch: {} -> {} ({} occurrences)",
                String::from_utf8_lossy(pattern),
                String::from_utf8_lossy(replacement),
                count
            );
            patches_applied += count;
        }
    }

    if patches_applied > 0 {
        std::fs::write(path, &data).map_err(|e| DriverError::Io(e.to_string()))?;
    }

    Ok(patches_applied)
}

fn replace_all_bytes(data: &mut [u8], pattern: &[u8], replacement: &[u8]) -> usize {
    if pattern.is_empty() || replacement.len() != pattern.len() {
        return 0;
    }

    let mut count = 0;
    let mut pos = 0;

    while let Some(idx) = data[pos..]
        .windows(pattern.len())
        .position(|w| w == pattern)
    {
        let actual_idx = pos + idx;
        let end = actual_idx + pattern.len();

        data[actual_idx..end].copy_from_slice(replacement);

        pos = actual_idx + replacement.len();
        count += 1;
    }

    count
}

/// Download with retry logic
async fn download_with_retry(url: &str) -> Result<Vec<u8>, DriverError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| DriverError::Network(e.to_string()))?;

    let mut last_error = None;

    for attempt in 1..=3 {
        println!("Download attempt {}...", attempt);

        match client.get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let bytes = response
                        .bytes()
                        .await
                        .map_err(|e| DriverError::Network(e.to_string()))?;
                    println!("Downloaded {} bytes", bytes.len());
                    return Ok(bytes.to_vec());
                }
                last_error = Some(format!("HTTP {}", response.status()));
            }
            Err(e) => {
                last_error = Some(format!("{:?}", e));
                if attempt < 3 {
                    println!("Failed, retrying in 2 seconds...");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    Err(DriverError::Network(format!(
        "Download failed after 3 attempts: {}",
        last_error.unwrap_or_else(|| "Unknown".to_string())
    )))
}

/// Error type for driver operations
#[derive(Debug)]
pub enum DriverError {
    Network(String),
    VersionNotFound(String),
    DownloadFailed(String),
    Extraction(String),
    Io(String),
    NotFound(String),
}

impl std::fmt::Display for DriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriverError::Network(e) => write!(f, "Network error: {}", e),
            DriverError::VersionNotFound(v) => {
                write!(f, "Version not found for: {}", v)
            }
            DriverError::DownloadFailed(s) => write!(f, "Download failed: {}", s),
            DriverError::Extraction(e) => write!(f, "Extraction error: {}", e),
            DriverError::Io(e) => write!(f, "IO error: {}", e),
            DriverError::NotFound(e) => write!(f, "Not found: {}", e),
        }
    }
}

impl std::error::Error for DriverError {}
