//! Application filtering for screen capture
//! Provides whitelist/blacklist functionality with default privacy protections

use serde::{Deserialize, Serialize};

/// Default privacy blacklist patterns (matched against window titles, case-insensitive)
const DEFAULT_PRIVACY_BLACKLIST: &[&str] = &[
    "password",
    "private",
    "incognito",
    "bank",
    "1password",
    "lastpass",
    "bitwarden",
    "keychain",
    "credential",
    "secret",
    "vault",
    "authenticator",
    "otp",
    "2fa",
    "login",
    "sign in",
    "signin",
];

/// Filter mode for application capture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AppFilterMode {
    /// No filtering - capture all apps
    #[default]
    None,
    /// Only capture apps in the whitelist
    Whitelist,
    /// Capture all apps except those in the blacklist
    Blacklist,
}

impl std::str::FromStr for AppFilterMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "whitelist" => Ok(Self::Whitelist),
            "blacklist" => Ok(Self::Blacklist),
            "none" | "" => Ok(Self::None),
            _ => Ok(Self::None),
        }
    }
}

impl std::fmt::Display for AppFilterMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Whitelist => write!(f, "whitelist"),
            Self::Blacklist => write!(f, "blacklist"),
        }
    }
}

/// Application filter for screen capture
#[derive(Debug, Clone)]
pub struct AppFilter {
    mode: AppFilterMode,
    app_list: Vec<String>,
    /// Whether to apply default privacy blacklist
    use_privacy_blacklist: bool,
}

impl Default for AppFilter {
    fn default() -> Self {
        Self {
            mode: AppFilterMode::None,
            app_list: Vec::new(),
            use_privacy_blacklist: true,
        }
    }
}

impl AppFilter {
    pub fn new(mode: AppFilterMode, app_list: Vec<String>) -> Self {
        Self {
            mode,
            app_list,
            use_privacy_blacklist: true,
        }
    }

    /// Create filter with custom privacy blacklist setting
    pub fn with_privacy_blacklist(mut self, enabled: bool) -> Self {
        self.use_privacy_blacklist = enabled;
        self
    }

    /// Check if a window/app should be captured based on filter rules
    ///
    /// # Arguments
    /// * `app_name` - The executable name (e.g., "chrome.exe")
    /// * `window_title` - The window title
    ///
    /// # Returns
    /// `true` if the window should be captured, `false` if it should be skipped
    pub fn should_capture(&self, app_name: &str, window_title: &str) -> bool {
        // First check privacy blacklist
        if self.use_privacy_blacklist && self.matches_privacy_blacklist(window_title) {
            tracing::debug!(
                "Skipping capture: window title matches privacy blacklist: {}",
                window_title
            );
            return false;
        }

        // Then apply user-configured filter
        match self.mode {
            AppFilterMode::None => true,
            AppFilterMode::Whitelist => self.is_in_list(app_name),
            AppFilterMode::Blacklist => !self.is_in_list(app_name),
        }
    }

    /// Check if window title matches any privacy blacklist pattern
    fn matches_privacy_blacklist(&self, window_title: &str) -> bool {
        let title_lower = window_title.to_lowercase();
        DEFAULT_PRIVACY_BLACKLIST
            .iter()
            .any(|pattern| title_lower.contains(pattern))
    }

    /// Check if app name is in the user's configured list
    fn is_in_list(&self, app_name: &str) -> bool {
        let app_lower = app_name.to_lowercase();
        self.app_list
            .iter()
            .any(|entry| app_lower.contains(&entry.to_lowercase()))
    }

    /// Get the current filter mode
    pub fn mode(&self) -> AppFilterMode {
        self.mode
    }

    /// Get the app list
    pub fn app_list(&self) -> &[String] {
        &self.app_list
    }

    /// Update the filter configuration
    pub fn update(&mut self, mode: AppFilterMode, app_list: Vec<String>) {
        self.mode = mode;
        self.app_list = app_list;
    }
}

/// Information about a running application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    /// Process name (e.g., "chrome.exe")
    pub process_name: String,
    /// Window title
    pub window_title: String,
    /// Whether this is the currently focused window
    pub is_foreground: bool,
}

#[cfg(windows)]
pub fn get_running_apps() -> Vec<AppInfo> {
    use std::collections::HashMap;
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible,
    };

    let mut apps: HashMap<String, AppInfo> = HashMap::new();
    let foreground_hwnd = unsafe { GetForegroundWindow() };

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let apps = &mut *(lparam.0 as *mut HashMap<String, AppInfo>);

        // Skip invisible windows
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }

        // Get window title
        let title_len = GetWindowTextLengthW(hwnd);
        if title_len == 0 {
            return BOOL(1);
        }

        let mut title_buf: Vec<u16> = vec![0; (title_len + 1) as usize];
        GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

        if title.is_empty() {
            return BOOL(1);
        }

        // Get process ID and name
        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        if let Some(process_name) = get_process_name(process_id) {
            // Use process name as key to deduplicate
            if !apps.contains_key(&process_name) {
                apps.insert(
                    process_name.clone(),
                    AppInfo {
                        process_name,
                        window_title: title,
                        is_foreground: false, // Will be updated later
                    },
                );
            }
        }

        BOOL(1)
    }

    unsafe {
        let apps_ptr = &mut apps as *mut HashMap<String, AppInfo>;
        let _ = EnumWindows(Some(enum_callback), LPARAM(apps_ptr as isize));
    }

    // Mark foreground window
    if !foreground_hwnd.is_invalid() {
        let mut process_id: u32 = 0;
        unsafe {
            GetWindowThreadProcessId(foreground_hwnd, Some(&mut process_id));
        }
        if let Some(process_name) = get_process_name(process_id) {
            if let Some(app) = apps.get_mut(&process_name) {
                app.is_foreground = true;
            }
        }
    }

    let mut result: Vec<AppInfo> = apps.into_values().collect();
    result.sort_by(|a, b| {
        // Foreground first, then alphabetically
        match (a.is_foreground, b.is_foreground) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.process_name.cmp(&b.process_name),
        }
    });

    result
}

#[cfg(windows)]
fn get_process_name(process_id: u32) -> Option<String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, process_id).ok()?;

        let mut name_buf: [u16; 260] = [0; 260];
        let len = GetModuleBaseNameW(handle, None, &mut name_buf);

        let _ = CloseHandle(handle);

        if len > 0 {
            let os_string = OsString::from_wide(&name_buf[..len as usize]);
            os_string.into_string().ok()
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
pub fn get_running_apps() -> Vec<AppInfo> {
    // Placeholder for non-Windows platforms
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_blacklist() {
        let filter = AppFilter::default();

        // Should block sensitive windows
        assert!(!filter.should_capture("chrome.exe", "Login - Google Chrome"));
        assert!(!filter.should_capture("firefox.exe", "1Password - Firefox"));
        assert!(!filter.should_capture("edge.exe", "Bank of America"));

        // Should allow normal windows
        assert!(filter.should_capture("chrome.exe", "GitHub - Google Chrome"));
        assert!(filter.should_capture("code.exe", "RECALL.OS - Visual Studio Code"));
    }

    #[test]
    fn test_whitelist_mode() {
        let filter = AppFilter::new(
            AppFilterMode::Whitelist,
            vec!["chrome.exe".to_string(), "code.exe".to_string()],
        );

        assert!(filter.should_capture("chrome.exe", "Normal Window"));
        assert!(filter.should_capture("code.exe", "Editor"));
        assert!(!filter.should_capture("firefox.exe", "Browser"));
    }

    #[test]
    fn test_blacklist_mode() {
        let filter = AppFilter::new(
            AppFilterMode::Blacklist,
            vec!["slack.exe".to_string()],
        );

        assert!(filter.should_capture("chrome.exe", "Normal Window"));
        assert!(!filter.should_capture("slack.exe", "Work Chat"));
    }
}
