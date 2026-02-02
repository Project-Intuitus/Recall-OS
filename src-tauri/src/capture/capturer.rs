//! Screenshot capture using xcap crate
//! Supports full screen and active window capture modes

use crate::error::{RecallError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use xcap::{Monitor, Window};

/// Capture mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    /// Capture the entire screen (primary monitor)
    FullScreen,
    /// Capture only the active/foreground window
    #[default]
    ActiveWindow,
}

impl std::str::FromStr for CaptureMode {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "full_screen" | "fullscreen" => Ok(Self::FullScreen),
            "active_window" | "activewindow" => Ok(Self::ActiveWindow),
            _ => Ok(Self::ActiveWindow),
        }
    }
}

impl std::fmt::Display for CaptureMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FullScreen => write!(f, "full_screen"),
            Self::ActiveWindow => write!(f, "active_window"),
        }
    }
}

/// Result of a screen capture
#[derive(Debug, Clone)]
pub struct CaptureResult {
    /// Path to the saved screenshot file
    pub file_path: PathBuf,
    /// Timestamp when the capture was taken
    pub captured_at: DateTime<Utc>,
    /// Capture mode used
    pub mode: CaptureMode,
    /// Source application name (for active window captures)
    pub source_app: Option<String>,
    /// Window title (for active window captures)
    pub window_title: Option<String>,
    /// Resolution of the captured image
    pub resolution: (u32, u32),
    /// Size of the saved file in bytes
    pub file_size: u64,
}

/// Screen capturer using xcap
pub struct Capturer {
    captures_dir: PathBuf,
}

impl Capturer {
    pub fn new(captures_dir: PathBuf) -> Result<Self> {
        // Create captures directory if it doesn't exist
        std::fs::create_dir_all(&captures_dir)?;
        Ok(Self { captures_dir })
    }

    /// Capture a screenshot based on the specified mode
    pub fn capture(&self, mode: CaptureMode) -> Result<CaptureResult> {
        match mode {
            CaptureMode::FullScreen => self.capture_full_screen(),
            CaptureMode::ActiveWindow => self.capture_active_window(),
        }
    }

    /// Capture the primary monitor
    fn capture_full_screen(&self) -> Result<CaptureResult> {
        let monitors = Monitor::all().map_err(|e| {
            RecallError::Capture(format!("Failed to enumerate monitors: {}", e))
        })?;

        let primary = monitors.into_iter().next().ok_or_else(|| {
            RecallError::Capture("No monitors found".to_string())
        })?;

        let image = primary.capture_image().map_err(|e| {
            RecallError::Capture(format!("Failed to capture screen: {}", e))
        })?;

        let resolution = (image.width(), image.height());
        let captured_at = Utc::now();
        let file_path = self.generate_file_path(&captured_at);

        // Save the image
        image.save(&file_path).map_err(|e| {
            RecallError::Capture(format!("Failed to save screenshot: {}", e))
        })?;

        let file_size = std::fs::metadata(&file_path)?.len();

        Ok(CaptureResult {
            file_path,
            captured_at,
            mode: CaptureMode::FullScreen,
            source_app: None,
            window_title: None,
            resolution,
            file_size,
        })
    }

    /// Capture the currently active/foreground window
    fn capture_active_window(&self) -> Result<CaptureResult> {
        let windows = Window::all().map_err(|e| {
            RecallError::Capture(format!("Failed to enumerate windows: {}", e))
        })?;

        // Find the foreground window
        let foreground_window = self.find_foreground_window(&windows)?;

        let image = foreground_window.capture_image().map_err(|e| {
            RecallError::Capture(format!("Failed to capture window: {}", e))
        })?;

        let resolution = (image.width(), image.height());
        let captured_at = Utc::now();
        let file_path = self.generate_file_path(&captured_at);

        // Get window metadata
        let source_app = Some(foreground_window.app_name().to_string());
        let window_title = Some(foreground_window.title().to_string());

        // Save the image
        image.save(&file_path).map_err(|e| {
            RecallError::Capture(format!("Failed to save screenshot: {}", e))
        })?;

        let file_size = std::fs::metadata(&file_path)?.len();

        tracing::info!(
            "Captured active window: {} - {} ({}x{})",
            source_app.as_deref().unwrap_or("unknown"),
            window_title.as_deref().unwrap_or(""),
            resolution.0,
            resolution.1
        );

        Ok(CaptureResult {
            file_path,
            captured_at,
            mode: CaptureMode::ActiveWindow,
            source_app,
            window_title,
            resolution,
            file_size,
        })
    }

    /// Find the foreground window from a list of windows
    fn find_foreground_window<'a>(&self, windows: &'a [Window]) -> Result<&'a Window> {
        #[cfg(windows)]
        {
            use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

            let foreground_hwnd = unsafe { GetForegroundWindow() };
            if foreground_hwnd.is_invalid() {
                return Err(RecallError::Capture("No foreground window found".to_string()));
            }

            let hwnd_isize = foreground_hwnd.0 as isize;

            // Find the matching window in xcap's list
            windows
                .iter()
                .find(|w| w.id() as isize == hwnd_isize)
                .ok_or_else(|| {
                    // Fallback: find a visible window with a title
                    windows
                        .iter()
                        .find(|w| !w.title().is_empty() && w.is_minimized() == false)
                        .ok_or_else(|| {
                            RecallError::Capture("Could not find capturable foreground window".to_string())
                        })
                })
                .or_else(|e| e)
        }

        #[cfg(not(windows))]
        {
            // On non-Windows, just try to find the first visible window
            windows
                .iter()
                .find(|w| !w.title().is_empty())
                .ok_or_else(|| {
                    RecallError::Capture("No visible window found".to_string())
                })
        }
    }

    /// Generate a unique file path for the screenshot
    fn generate_file_path(&self, timestamp: &DateTime<Utc>) -> PathBuf {
        let filename = format!(
            "capture_{}.png",
            timestamp.format("%Y-%m-%d_%H-%M-%S")
        );
        self.captures_dir.join(filename)
    }

    /// Get the captures directory
    pub fn captures_dir(&self) -> &PathBuf {
        &self.captures_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_capture_mode_parsing() {
        assert_eq!(
            "full_screen".parse::<CaptureMode>().unwrap(),
            CaptureMode::FullScreen
        );
        assert_eq!(
            "active_window".parse::<CaptureMode>().unwrap(),
            CaptureMode::ActiveWindow
        );
        assert_eq!(
            "invalid".parse::<CaptureMode>().unwrap(),
            CaptureMode::ActiveWindow
        );
    }

    #[test]
    fn test_capturer_creation() {
        let temp_dir = tempdir().unwrap();
        let capturer = Capturer::new(temp_dir.path().to_path_buf()).unwrap();
        assert!(capturer.captures_dir().exists());
    }
}
