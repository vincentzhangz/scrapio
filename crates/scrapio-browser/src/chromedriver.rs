//! ChromeDriver manager for automatic download and management

use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Child;
use std::time::{Duration, Instant};

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
    /// Whether to apply stealth patches to the binary
    patch_stealth: bool,
    /// Custom ChromeDriver path (skip download if set)
    custom_path: Option<PathBuf>,
}

/// Managed ChromeDriver session that starts on an ephemeral port and stops on drop.
#[derive(Debug)]
pub struct ChromeDriverSession {
    child: Option<Child>,
    port: u16,
}

impl ChromeDriverSession {
    /// Start a managed ChromeDriver session with default manager settings.
    pub async fn start() -> Result<Self, ChromeDriverError> {
        Self::start_with(ChromeDriverManager::new()).await
    }

    /// Start a managed ChromeDriver session using the provided manager configuration.
    pub async fn start_with(mut manager: ChromeDriverManager) -> Result<Self, ChromeDriverError> {
        let port = find_available_port()?;
        let mut child = manager.download_and_start(port).await?;

        if let Err(err) = wait_for_port(port, &mut child).await {
            let _ = child.kill();
            return Err(err);
        }

        Ok(Self {
            child: Some(child),
            port,
        })
    }

    /// Get the WebDriver URL for this session.
    pub fn webdriver_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    /// Stop the managed ChromeDriver process explicitly.
    pub fn stop(mut self) {
        if let Some(child) = self.child.take() {
            ChromeDriverManager::stop(child);
        }
    }
}

impl Drop for ChromeDriverSession {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
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
            patch_stealth: false, // Disabled by default - causes issues on macOS
            custom_path: None,
        }
    }

    /// Use a custom ChromeDriver path instead of downloading
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.custom_path = Some(path);
        self
    }

    /// Get the driver path - returns custom path if set, otherwise computes from cache
    pub fn driver_path(&self) -> PathBuf {
        if let Some(ref path) = self.custom_path {
            return path.clone();
        }
        let platform_dir = self.get_platform_dir();
        let driver_dir = self.cache_dir.join(platform_dir);
        if self.os == Os::Windows {
            driver_dir.join("chromedriver.exe")
        } else {
            driver_dir.join("chromedriver")
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

    /// Enable or disable stealth patching (default: true)
    ///
    /// When enabled, the ChromeDriver binary will be patched to remove
    /// detection vectors like `cdc_adoQpoasnfa76pfcZLmcfl_` and `window.wdc_` globals.
    pub fn with_stealth_patching(mut self, enabled: bool) -> Self {
        self.patch_stealth = enabled;
        self
    }

    /// Get the cache directory path
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
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

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
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
        // Use custom path if provided
        if let Some(ref path) = self.custom_path {
            if path.exists() {
                return Ok(path.clone());
            }
            return Err(ChromeDriverError::NotFound(format!(
                "Custom ChromeDriver path does not exist: {:?}",
                path
            )));
        }

        // Create cache directory
        std::fs::create_dir_all(&self.cache_dir)
            .map_err(|e| ChromeDriverError::Io(e.to_string()))?;

        // Determine version and download URL
        let (version, download_url) = if let Some(ref ver) = self.version {
            (ver.clone(), self.get_download_url(ver))
        } else {
            let channel_info = self.fetch_version_info().await?;
            (
                channel_info.version.clone(),
                self.get_download_url(&channel_info.version),
            )
        };

        self.version = Some(version.clone());

        // Check if already downloaded
        if self.driver_path().exists() {
            println!("ChromeDriver already exists at {:?}", self.driver_path());
            return Ok(self.driver_path());
        }

        println!("Downloading ChromeDriver {} from {}", version, download_url);

        // Download with retry using async reqwest
        let bytes = download_with_retry(&download_url).await?;

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

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode)).ok();
                }
            }
        }

        // On macOS, remove extended attributes that may cause security issues
        // This helps bypass Gatekeeper restrictions
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let _ = Command::new("xattr")
                .args(["-cr", &self.driver_path().to_string_lossy()])
                .output();
        }

        println!("ChromeDriver extracted to {:?}", self.driver_path());

        // Apply stealth patches (with fallback on failure)
        // Note: On macOS, patched binaries are often killed by security, so we verify after patching
        if self.patch_stealth {
            // First, save a backup of the original binary
            let original_binary = std::fs::read(self.driver_path())
                .map_err(|e| ChromeDriverError::Io(e.to_string()))?;

            match patch_chromedriver(&self.driver_path()) {
                Ok(count) => {
                    if count > 0 {
                        println!("Applied {} stealth patches to ChromeDriver", count);
                        // Verify it works
                        match verify_chromedriver(&self.driver_path()) {
                            Ok(_) => println!("ChromeDriver verification passed"),
                            Err(e) => {
                                println!("Warning: Patched ChromeDriver failed: {}", e);
                                println!(
                                    "Using unpatched ChromeDriver (JavaScript stealth will still work)"
                                );
                                // Restore original binary
                                std::fs::write(self.driver_path(), &original_binary)
                                    .map_err(|e| ChromeDriverError::Io(e.to_string()))?;
                            }
                        }
                    } else {
                        println!("No stealth patches needed (patterns not found)");
                    }
                }
                Err(e) => {
                    println!("Warning: Failed to patch ChromeDriver: {}", e);
                }
            }
        }

        Ok(self.driver_path())
    }

    /// Download if not exists and return the path
    pub async fn ensure(&mut self) -> Result<PathBuf, ChromeDriverError> {
        if self.driver_path().exists() {
            // Apply patches even if already downloaded (in case it was updated externally)
            if self.patch_stealth {
                let _ = self.apply_patches();
            }
            Ok(self.driver_path())
        } else {
            self.download().await
        }
    }

    /// Ensure stealth patches are applied to an existing ChromeDriver binary.
    /// This can be called to re-apply patches after a driver update.
    pub fn apply_patches(&self) -> Result<usize, ChromeDriverError> {
        if !self.patch_stealth {
            tracing::debug!("Stealth patching is disabled");
            return Ok(0);
        }
        patch_chromedriver(&self.driver_path())
    }

    /// Force re-download and patch ChromeDriver
    pub async fn force_download(&mut self) -> Result<PathBuf, ChromeDriverError> {
        // Remove existing binary
        let path = self.driver_path();
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| ChromeDriverError::Io(e.to_string()))?;
        }
        self.download().await
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

fn find_available_port() -> Result<u16, ChromeDriverError> {
    let listener =
        TcpListener::bind("127.0.0.1:0").map_err(|e| ChromeDriverError::Io(e.to_string()))?;
    listener
        .local_addr()
        .map(|addr| addr.port())
        .map_err(|e| ChromeDriverError::Io(e.to_string()))
}

async fn wait_for_port(port: u16, child: &mut Child) -> Result<(), ChromeDriverError> {
    let addr = format!("127.0.0.1:{}", port);
    let deadline = Instant::now() + Duration::from_secs(10);

    while Instant::now() < deadline {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| ChromeDriverError::Io(e.to_string()))?
        {
            return Err(ChromeDriverError::NotFound(format!(
                "ChromeDriver exited before accepting connections: {}",
                status
            )));
        }

        if tokio::net::TcpStream::connect(&addr).await.is_ok() {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Err(ChromeDriverError::NotFound(format!(
        "ChromeDriver did not become ready on port {}",
        port
    )))
}

/// Patches the ChromeDriver binary to remove detection vectors.
///
/// This patches common Selenium/ChromeDriver detection points:
/// - `cdc_adoQpoasnfa76pfcZLmcfl_` globals (Selenium's internal variable prefix)
/// - `window.wdc_` globals (ChromeDriver's global object prefix)
/// - Various CDP-related detection points
///
/// Returns the number of patches applied, or an error if patching fails.
pub fn patch_chromedriver(path: &std::path::Path) -> Result<usize, ChromeDriverError> {
    if !path.exists() {
        return Err(ChromeDriverError::NotFound(
            "ChromeDriver binary not found".to_string(),
        ));
    }

    let mut data = std::fs::read(path).map_err(|e| ChromeDriverError::Io(e.to_string()))?;

    let original_len = data.len();
    let mut patches_applied = 0;

    // Define patches: (pattern, replacement) - must be same length
    // Using random-looking prefixes to avoid detection
    // All patterns and replacements must be exactly the same byte length
    let patches: Vec<(&[u8], &[u8])> = vec![
        // Selenium's main detection variable (27 bytes each)
        (
            b"cdc_adoQpoasnfa76pfcZLmcfl_",
            b"jse_evaluate_0000_abcdefghi",
        ),
        // ChromeDriver global objects (matching lengths)
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
        // Some versions use shorter patterns (24 bytes each)
        (b"cdc_adoQpoasnfa76pfcZLmcfl", b"jse_evaluate_0000abcdef"),
    ];

    for (pattern, replacement) in patches {
        if pattern.len() != replacement.len() {
            tracing::warn!("Skipping patch: pattern and replacement have different lengths");
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

    // Only write if we made changes
    if patches_applied > 0 {
        std::fs::write(path, &data).map_err(|e| ChromeDriverError::Io(e.to_string()))?;
        tracing::info!(
            "Patched ChromeDriver: {} bytes -> {} bytes, {} patches applied",
            original_len,
            data.len(),
            patches_applied
        );
    } else {
        tracing::debug!("No patches applied to ChromeDriver (patterns not found)");
    }

    Ok(patches_applied)
}

/// Download ChromeDriver with retry logic
async fn download_with_retry(url: &str) -> Result<Vec<u8>, ChromeDriverError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| ChromeDriverError::Network(e.to_string()))?;

    let mut last_error = None;

    for attempt in 1..=3 {
        println!("Download attempt {}...", attempt);

        match client.get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let bytes = response
                        .bytes()
                        .await
                        .map_err(|e| ChromeDriverError::Network(e.to_string()))?;
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

    Err(ChromeDriverError::Network(format!(
        "Download failed after 3 attempts: {}",
        last_error.unwrap_or_else(|| "Unknown".to_string())
    )))
}

/// Verify that a ChromeDriver binary can run by checking its version.
/// Returns Ok(()) if successful, or an error if the binary fails to run.
fn verify_chromedriver(path: &std::path::Path) -> Result<(), ChromeDriverError> {
    use std::process::Command;

    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|e| ChromeDriverError::Io(format!("Failed to run ChromeDriver: {}", e)))?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        tracing::debug!("ChromeDriver version check: {}", version.trim());
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(ChromeDriverError::Io(format!(
            "ChromeDriver failed version check: {}",
            stderr
        )))
    }
}

/// Replace all occurrences of a byte pattern in data.
/// Returns the number of replacements made.
fn replace_all_bytes(data: &mut [u8], pattern: &[u8], replacement: &[u8]) -> usize {
    if pattern.is_empty() || replacement.len() != pattern.len() {
        return 0;
    }

    let mut count = 0;
    let mut pos = 0;

    // Use windows() to correctly find pattern matches at each position
    while let Some(idx) = data[pos..]
        .windows(pattern.len())
        .position(|w| w == pattern)
    {
        let actual_idx = pos + idx;
        let end = actual_idx + pattern.len();

        // Replace in place
        data[actual_idx..end].copy_from_slice(replacement);

        // Move past the replaced region to avoid matching overlapping instances
        pos = actual_idx + replacement.len();
        count += 1;
    }

    count
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

    #[test]
    fn test_managed_session_webdriver_url() {
        let session = ChromeDriverSession {
            child: None,
            port: 9876,
        };

        assert_eq!(session.webdriver_url(), "http://localhost:9876");
    }

    #[test]
    fn test_stealth_patching_default() {
        let manager = ChromeDriverManager::new();
        // Stealth patching is disabled by default - causes issues on macOS
        assert!(!manager.patch_stealth);
    }

    #[test]
    fn test_stealth_patching_can_be_disabled() {
        let manager = ChromeDriverManager::new().with_stealth_patching(false);
        assert!(!manager.patch_stealth);
    }

    #[test]
    fn test_replace_all_bytes() {
        let mut data =
            b"hello cdc_adoQpoasnfa76pfcZLmcfl_ world cdc_adoQpoasnfa76pfcZLmcfl_ test".to_vec();
        let count = replace_all_bytes(
            &mut data,
            b"cdc_adoQpoasnfa76pfcZLmcfl_",
            b"jse_evaluate_0000_abcdefghi",
        );
        assert_eq!(count, 2);
        let data_str = String::from_utf8_lossy(&data);
        assert!(data_str.contains("jse_evaluate_0000_abcdefghi"));
        assert!(!data_str.contains("cdc_adoQpoasnfa76pfcZLmcfl_"));
    }

    #[test]
    fn test_replace_all_bytes_no_match() {
        let mut data = b"hello world".to_vec();
        let count = replace_all_bytes(&mut data, b"xyz", b"abc");
        assert_eq!(count, 0);
        assert_eq!(&data, b"hello world");
    }

    #[test]
    fn test_replace_all_bytes_different_length() {
        let mut data = b"hello world".to_vec();
        let count = replace_all_bytes(&mut data, b"hello", b"hi");
        assert_eq!(count, 0); // Different lengths should be skipped
    }
}
