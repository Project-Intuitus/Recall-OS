use crate::error::RecallError;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

/// Maximum documents allowed in trial mode
pub const TRIAL_DOCUMENT_LIMIT: usize = 25;

/// LemonSqueezy Product ID for RECALL.OS
const LEMONSQUEEZY_PRODUCT_ID: u64 = 805257;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub is_valid: bool,
    pub license_key: Option<String>, // Masked for display
    pub activated_at: Option<String>,
    pub tier: LicenseTier,
    pub documents_used: Option<usize>,
    pub documents_limit: Option<usize>,
    pub customer_name: Option<String>,
    pub customer_email: Option<String>,
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

/// LemonSqueezy API response structures
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LemonSqueezyValidateResponse {
    valid: bool,
    error: Option<String>,
    license_key: Option<LemonSqueezyLicenseKey>,
    meta: Option<LemonSqueezyMeta>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LemonSqueezyLicenseKey {
    id: u64,
    status: String,
    key: String,
    activation_limit: Option<u32>,
    activation_usage: u32,
    expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LemonSqueezyMeta {
    product_id: u64,
    product_name: String,
    customer_id: u64,
    customer_name: String,
    customer_email: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LemonSqueezyActivateResponse {
    activated: bool,
    error: Option<String>,
    license_key: Option<LemonSqueezyLicenseKey>,
    instance: Option<LemonSqueezyInstance>,
    meta: Option<LemonSqueezyMeta>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LemonSqueezyDeactivateResponse {
    deactivated: bool,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LemonSqueezyInstance {
    id: String,
    name: String,
}

/// Get the current license status
#[tauri::command]
pub async fn get_license_status(state: State<'_, Arc<AppState>>) -> Result<LicenseStatus, RecallError> {
    let settings = state.settings.read();

    // Get document count for trial info
    let stats = state.database.get_ingestion_stats()?;
    let docs_used = stats.total_documents as usize;

    if let Some(ref license_key) = settings.license_key {
        // We have a stored license, check if it's still valid
        // For performance, we don't re-validate on every call
        // The key was validated on activation
        Ok(LicenseStatus {
            is_valid: true,
            license_key: Some(mask_license_key(license_key)),
            activated_at: settings.license_activated_at.clone(),
            tier: LicenseTier::Licensed,
            documents_used: None, // Licensed users don't have limits
            documents_limit: None,
            customer_name: settings.license_customer_name.clone(),
            customer_email: settings.license_customer_email.clone(),
        })
    } else {
        Ok(LicenseStatus {
            is_valid: false,
            license_key: None,
            activated_at: None,
            tier: LicenseTier::Trial,
            documents_used: Some(docs_used),
            documents_limit: Some(TRIAL_DOCUMENT_LIMIT),
            customer_name: None,
            customer_email: None,
        })
    }
}

/// Validate and activate a license key via LemonSqueezy API
#[tauri::command]
pub async fn activate_license(
    state: State<'_, Arc<AppState>>,
    license_key: String,
) -> Result<LicenseStatus, RecallError> {
    let key = license_key.trim().to_string();

    if key.is_empty() {
        return Err(RecallError::Other("License key cannot be empty".to_string()));
    }

    // Get a unique instance identifier for this machine
    let instance_id = get_instance_id();

    // Call LemonSqueezy API to activate the license
    let client = reqwest::Client::new();

    let response = client
        .post("https://api.lemonsqueezy.com/v1/licenses/activate")
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "license_key": key,
            "instance_name": instance_id,
        }))
        .send()
        .await
        .map_err(|e| RecallError::Other(format!("Failed to connect to license server: {}", e)))?;

    let _status = response.status();
    let body: LemonSqueezyActivateResponse = response
        .json()
        .await
        .map_err(|e| RecallError::Other(format!("Invalid response from license server: {}", e)))?;

    // Check for errors
    if let Some(error) = body.error {
        return Err(RecallError::Other(format!("License activation failed: {}", error)));
    }

    if !body.activated {
        return Err(RecallError::Other("License activation failed. Please check your key and try again.".to_string()));
    }

    // Verify this is for our product
    if let Some(ref meta) = body.meta {
        if meta.product_id != LEMONSQUEEZY_PRODUCT_ID {
            return Err(RecallError::Other("This license key is not valid for RECALL.OS".to_string()));
        }
    }

    // Get customer info
    let customer_name = body.meta.as_ref().map(|m| m.customer_name.clone());
    let customer_email = body.meta.as_ref().map(|m| m.customer_email.clone());
    let instance_id_stored = body.instance.as_ref().map(|i| i.id.clone());

    // Store the validated key
    let activated_at = chrono::Utc::now().to_rfc3339();
    {
        let mut settings = state.settings.write();
        settings.license_key = Some(key.clone());
        settings.license_activated_at = Some(activated_at.clone());
        settings.license_customer_name = customer_name.clone();
        settings.license_customer_email = customer_email.clone();
        settings.license_instance_id = instance_id_stored;
    }
    state.save_settings()?;

    tracing::info!("License activated successfully via LemonSqueezy");

    Ok(LicenseStatus {
        is_valid: true,
        license_key: Some(mask_license_key(&key)),
        activated_at: Some(activated_at),
        tier: LicenseTier::Licensed,
        documents_used: None,
        documents_limit: None,
        customer_name,
        customer_email,
    })
}

/// Deactivate the current license
#[tauri::command]
pub async fn deactivate_license(state: State<'_, Arc<AppState>>) -> Result<(), RecallError> {
    let (license_key, instance_id) = {
        let settings = state.settings.read();
        (settings.license_key.clone(), settings.license_instance_id.clone())
    };

    // If we have a license key and instance ID, deactivate via API
    if let (Some(key), Some(instance)) = (license_key, instance_id) {
        let client = reqwest::Client::new();

        let response = client
            .post("https://api.lemonsqueezy.com/v1/licenses/deactivate")
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "license_key": key,
                "instance_id": instance,
            }))
            .send()
            .await;

        // Log but don't fail if deactivation fails (user might be offline)
        if let Err(e) = response {
            tracing::warn!("Failed to deactivate license on server: {}", e);
        }
    }

    // Clear local license data regardless
    {
        let mut settings = state.settings.write();
        settings.license_key = None;
        settings.license_activated_at = None;
        settings.license_customer_name = None;
        settings.license_customer_email = None;
        settings.license_instance_id = None;
    }
    state.save_settings()?;

    tracing::info!("License deactivated");
    Ok(())
}

/// Verify license is still valid (call periodically or on app start)
#[tauri::command]
pub async fn verify_license(state: State<'_, Arc<AppState>>) -> Result<bool, RecallError> {
    let license_key = {
        let settings = state.settings.read();
        settings.license_key.clone()
    };

    let Some(key) = license_key else {
        return Ok(false);
    };

    let client = reqwest::Client::new();

    let response = client
        .post("https://api.lemonsqueezy.com/v1/licenses/validate")
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "license_key": key,
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if let Ok(body) = resp.json::<LemonSqueezyValidateResponse>().await {
                if !body.valid {
                    // License is no longer valid, clear it
                    tracing::warn!("License validation failed, clearing local license");
                    let mut settings = state.settings.write();
                    settings.license_key = None;
                    settings.license_activated_at = None;
                    drop(settings);
                    let _ = state.save_settings();
                    return Ok(false);
                }

                // Check if license is for our product
                if let Some(meta) = body.meta {
                    if meta.product_id != LEMONSQUEEZY_PRODUCT_ID {
                        return Ok(false);
                    }
                }

                return Ok(true);
            }
        }
        Err(e) => {
            // Network error - assume valid if we have a stored key
            // (allow offline usage)
            tracing::warn!("Could not verify license (offline?): {}", e);
            return Ok(true);
        }
    }

    Ok(false)
}

/// Mask license key for display (show only last 4 characters)
fn mask_license_key(key: &str) -> String {
    if key.len() <= 8 {
        "****-****".to_string()
    } else {
        let visible = &key[key.len().saturating_sub(4)..];
        format!("****-****-{}", visible)
    }
}

/// Activate a test license for development (debug builds only)
#[cfg(debug_assertions)]
#[tauri::command]
pub async fn activate_test_license(state: State<'_, Arc<AppState>>) -> Result<LicenseStatus, RecallError> {
    let activated_at = chrono::Utc::now().to_rfc3339();
    {
        let mut settings = state.settings.write();
        settings.license_key = Some("TEST-LICENSE-KEY".to_string());
        settings.license_activated_at = Some(activated_at.clone());
        settings.license_customer_name = Some("Test User".to_string());
        settings.license_customer_email = Some("test@example.com".to_string());
        settings.license_instance_id = Some("test-instance".to_string());
    }
    state.save_settings()?;

    tracing::info!("Test license activated (debug build only)");

    Ok(LicenseStatus {
        is_valid: true,
        license_key: Some("****-TEST".to_string()),
        activated_at: Some(activated_at),
        tier: LicenseTier::Licensed,
        documents_used: None,
        documents_limit: None,
        customer_name: Some("Test User".to_string()),
        customer_email: Some("test@example.com".to_string()),
    })
}

/// Get a unique identifier for this machine instance
fn get_instance_id() -> String {
    // Try to get machine-specific ID
    if let Ok(hostname) = hostname::get() {
        if let Some(name) = hostname.to_str() {
            return format!("RECALL-{}", name);
        }
    }

    // Fallback to a random ID
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("RECALL-{}", timestamp)
}
