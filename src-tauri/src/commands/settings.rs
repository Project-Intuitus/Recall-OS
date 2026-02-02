use crate::error::RecallError;
use crate::llm::validate_api_key as validate_key;
use crate::state::{AppState, Settings};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_settings(state: State<'_, Arc<AppState>>) -> Result<Settings, RecallError> {
    let settings = state.settings.read().clone();

    // Mask API key for security
    Ok(Settings {
        gemini_api_key: settings.gemini_api_key.map(|k| mask_api_key(&k)),
        ..settings
    })
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, Arc<AppState>>,
    new_settings: Settings,
) -> Result<(), RecallError> {
    tracing::info!("update_settings called");

    // If API key changed, update LLM client
    if let Some(ref api_key) = new_settings.gemini_api_key {
        if !api_key.starts_with("****") {
            // It's a new key, not the masked one
            state.update_llm_client(api_key.clone());
        }
    }

    // Preserve existing values that are managed elsewhere
    let mut settings = state.settings.write();
    let existing_key = settings.gemini_api_key.clone();
    let existing_watched_folders = settings.watched_folders.clone();
    let existing_auto_ingest = settings.auto_ingest_enabled;

    *settings = new_settings;

    // Preserve API key if new one is masked
    if settings
        .gemini_api_key
        .as_ref()
        .map(|k| k.starts_with("****"))
        .unwrap_or(false)
    {
        settings.gemini_api_key = existing_key;
    }

    // Preserve watcher settings (managed by watcher commands)
    settings.watched_folders = existing_watched_folders;
    settings.auto_ingest_enabled = existing_auto_ingest;

    drop(settings);

    state.save_settings()?;
    Ok(())
}

#[tauri::command]
pub async fn validate_api_key(
    state: State<'_, Arc<AppState>>,
    api_key: String,
) -> Result<bool, RecallError> {
    let is_valid = validate_key(&api_key).await?;

    if is_valid {
        // Update the client with the validated key
        state.update_llm_client(api_key.clone());

        // Save to settings
        let mut settings = state.settings.write();
        settings.gemini_api_key = Some(api_key);
        drop(settings);

        state.save_settings()?;
    }

    Ok(is_valid)
}

#[tauri::command]
pub async fn get_api_key_unmasked(
    state: State<'_, Arc<AppState>>,
) -> Result<Option<String>, RecallError> {
    let settings = state.settings.read();
    Ok(settings.gemini_api_key.clone())
}

#[tauri::command]
pub async fn clear_api_key(
    state: State<'_, Arc<AppState>>,
) -> Result<(), RecallError> {
    {
        let mut settings = state.settings.write();
        settings.gemini_api_key = None;
    }
    *state.llm_client.write() = None;
    state.save_settings()?;
    Ok(())
}

fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        "****".to_string()
    } else {
        format!("****{}", &key[key.len() - 4..])
    }
}
