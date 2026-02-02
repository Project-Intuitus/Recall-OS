//! AUMID (Application User Model ID) registration for Windows notifications.
//!
//! Windows toast notifications require a registered AUMID to display the correct
//! app name and icon. Without registration, notifications show the parent process
//! name (e.g., "Windows PowerShell" in development mode).

use std::path::Path;

use super::AUMID;

/// Ensures the AUMID is registered in the Windows Registry.
///
/// Creates the registry key at:
/// `HKEY_CURRENT_USER\Software\Classes\AppUserModelId\com.recallos.app`
///
/// With values:
/// - DisplayName: "RECALL.OS"
/// - IconUri: Path to the app icon
/// - IconBackgroundColor: Slate-800 color (FF1e293b)
pub fn ensure_aumid_registered(icon_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = format!(r"Software\Classes\AppUserModelId\{}", AUMID);

    // Create or open the key
    let (key, disposition) = hkcu.create_subkey(&path)?;

    let action = match disposition {
        winreg::enums::RegDisposition::REG_CREATED_NEW_KEY => "Created",
        winreg::enums::RegDisposition::REG_OPENED_EXISTING_KEY => "Updated",
    };

    // Set the display name
    key.set_value("DisplayName", &"RECALL.OS")?;

    // Set the icon path (only if the file exists)
    if icon_path.exists() {
        let icon_path_str = icon_path.to_string_lossy().to_string();
        key.set_value("IconUri", &icon_path_str)?;
        tracing::debug!("AUMID icon set to: {}", icon_path_str);
    } else {
        tracing::warn!("Icon file not found at: {:?}", icon_path);
    }

    // Set the background color (slate-800 from Tailwind)
    key.set_value("IconBackgroundColor", &"FF1e293b")?;

    tracing::info!("{} AUMID registry entry: {}", action, path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aumid_constant() {
        assert_eq!(AUMID, "com.recallos.app");
    }
}
