use crate::error::RecallError;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

/// Maximum documents allowed in trial mode
pub const TRIAL_DOCUMENT_LIMIT: usize = 25;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub is_valid: bool,
    pub license_key: Option<String>, // Masked for display
    pub activated_at: Option<String>,
    pub tier: LicenseTier,
    pub documents_used: Option<usize>,
    pub documents_limit: Option<usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LicenseTier {
    Trial,     // Limited features
    Licensed,  // Full features with one-time payment
}

impl Default for LicenseTier {
    fn default() -> Self {
        Self::Trial
    }
}

/// Get the current license status
#[tauri::command]
pub async fn get_license_status(state: State<'_, Arc<AppState>>) -> Result<LicenseStatus, RecallError> {
    let settings = state.settings.read();

    // Get document count for trial info
    let stats = state.database.get_ingestion_stats()?;
    let docs_used = stats.total_documents as usize;

    if let Some(ref license_key) = settings.license_key {
        // Validate the stored key
        let is_valid = validate_license_key_format(license_key);

        Ok(LicenseStatus {
            is_valid,
            license_key: Some(mask_license_key(license_key)),
            activated_at: settings.license_activated_at.clone(),
            tier: if is_valid { LicenseTier::Licensed } else { LicenseTier::Trial },
            documents_used: if is_valid { None } else { Some(docs_used) },
            documents_limit: if is_valid { None } else { Some(TRIAL_DOCUMENT_LIMIT) },
        })
    } else {
        Ok(LicenseStatus {
            is_valid: false,
            license_key: None,
            activated_at: None,
            tier: LicenseTier::Trial,
            documents_used: Some(docs_used),
            documents_limit: Some(TRIAL_DOCUMENT_LIMIT),
        })
    }
}

/// Validate and activate a license key
#[tauri::command]
pub async fn activate_license(
    state: State<'_, Arc<AppState>>,
    license_key: String,
) -> Result<LicenseStatus, RecallError> {
    // Basic format validation
    let key = license_key.trim().to_uppercase();

    if !validate_license_key_format(&key) {
        return Err(RecallError::Other("Invalid license key format. Expected format: RO-XXXX-XXXX-XXXX".to_string()));
    }

    // For V1, we'll do local-only validation with a simple checksum
    // In production, this would call a backend API to validate
    if !validate_license_checksum(&key) {
        return Err(RecallError::Other("License key validation failed. Please check your key and try again.".to_string()));
    }

    // Store the validated key
    let activated_at = chrono::Utc::now().to_rfc3339();
    {
        let mut settings = state.settings.write();
        settings.license_key = Some(key.clone());
        settings.license_activated_at = Some(activated_at.clone());
    }
    state.save_settings()?;

    tracing::info!("License activated successfully");

    Ok(LicenseStatus {
        is_valid: true,
        license_key: Some(mask_license_key(&key)),
        activated_at: Some(activated_at),
        tier: LicenseTier::Licensed,
        documents_used: None, // Licensed users don't have limits
        documents_limit: None,
    })
}

/// Deactivate the current license
#[tauri::command]
pub async fn deactivate_license(state: State<'_, Arc<AppState>>) -> Result<(), RecallError> {
    {
        let mut settings = state.settings.write();
        settings.license_key = None;
        settings.license_activated_at = None;
    }
    state.save_settings()?;

    tracing::info!("License deactivated");
    Ok(())
}

/// Validate license key format: RO-XXXX-XXXX-XXXX
fn validate_license_key_format(key: &str) -> bool {
    let parts: Vec<&str> = key.split('-').collect();

    if parts.len() != 4 {
        return false;
    }

    if parts[0] != "RO" {
        return false;
    }

    // Each segment after prefix should be 4 alphanumeric characters
    for segment in &parts[1..] {
        if segment.len() != 4 {
            return false;
        }
        if !segment.chars().all(|c| c.is_ascii_alphanumeric()) {
            return false;
        }
    }

    true
}

/// Simple checksum validation for offline verification
/// In production, this would be replaced with proper cryptographic validation
/// or an API call to your license server
fn validate_license_checksum(key: &str) -> bool {
    // Remove the prefix and dashes
    let clean_key: String = key.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .skip(2) // Skip "RO"
        .collect();

    if clean_key.len() != 12 {
        return false;
    }

    // Simple checksum: sum of ASCII values modulo 97 should equal last two digits interpreted as number
    // This is a placeholder - in production use proper cryptographic signing
    let chars: Vec<char> = clean_key.chars().collect();
    let check_chars: String = chars[10..12].iter().collect();

    // Convert last 2 chars to a number (base 36)
    let check_value = u32::from_str_radix(&check_chars, 36).unwrap_or(0);

    // Calculate checksum from first 10 chars
    let sum: u32 = chars[0..10].iter()
        .map(|c| *c as u32)
        .sum();

    let expected = sum % 1296; // 36^2 = 1296

    check_value == expected
}

/// Mask license key for display (show only last 4 characters)
fn mask_license_key(key: &str) -> String {
    if key.len() <= 4 {
        "****".to_string()
    } else {
        let visible = &key[key.len() - 4..];
        format!("RO-****-****-{}", visible)
    }
}

/// Generate a valid license key (for testing purposes)
/// This should NOT be included in production builds
#[cfg(debug_assertions)]
#[tauri::command]
pub async fn generate_test_license() -> Result<String, RecallError> {
    use rand::Rng;

    let mut rng = rand::thread_rng();

    // Generate 10 random alphanumeric characters
    let chars: String = (0..10)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'A' + idx - 10) as char
            }
        })
        .collect();

    // Calculate checksum
    let sum: u32 = chars.chars().map(|c| c as u32).sum();
    let check = sum % 1296;

    // Convert checksum to 2 base-36 chars
    let check_chars: String = format!("{:02}", check)
        .chars()
        .take(2)
        .map(|c| {
            let d = c.to_digit(10).unwrap_or(0);
            if d < 10 {
                (b'0' + d as u8) as char
            } else {
                (b'A' + (d - 10) as u8) as char
            }
        })
        .collect();

    // Format: RO-XXXX-XXXX-XXCC
    let full = format!("{}{}", chars, check_chars);
    let key = format!(
        "RO-{}-{}-{}",
        &full[0..4],
        &full[4..8],
        &full[8..12]
    );

    Ok(key)
}
