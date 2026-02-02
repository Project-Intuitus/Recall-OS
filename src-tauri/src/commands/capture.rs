//! Tauri commands for screen capture functionality

use crate::capture::{AppFilterMode, AppInfo, CaptureMode, CaptureSettings, CaptureStatus, get_running_apps};
use crate::error::Result;
use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, State};

/// Start periodic screen capture
#[tauri::command]
pub async fn start_screen_capture(
    state: State<'_, Arc<AppState>>,
    app_handle: AppHandle,
) -> Result<()> {
    tracing::info!("Starting screen capture");

    // Update settings to enable capture
    {
        let mut settings = state.settings.write();
        settings.screen_capture_enabled = true;
    }
    state.save_settings()?;

    // Sync settings to capture manager
    let capture_settings = get_capture_settings_from_state(&state);
    state.capture_manager.update_settings(capture_settings);

    // Start the scheduler
    state.capture_manager.clone().start_scheduler(app_handle);

    Ok(())
}

/// Stop periodic screen capture
#[tauri::command]
pub async fn stop_screen_capture(state: State<'_, Arc<AppState>>) -> Result<()> {
    tracing::info!("Stopping screen capture");

    // Update settings to disable capture
    {
        let mut settings = state.settings.write();
        settings.screen_capture_enabled = false;
    }
    state.save_settings()?;

    // Stop the scheduler (synchronous)
    state.capture_manager.stop_scheduler();

    Ok(())
}

/// Capture a screenshot now (manual trigger)
#[tauri::command]
pub async fn capture_now(
    state: State<'_, Arc<AppState>>,
    app_handle: AppHandle,
) -> Result<String> {
    tracing::info!("Manual capture triggered");

    // Sync settings before capture
    let capture_settings = get_capture_settings_from_state(&state);
    state.capture_manager.update_settings(capture_settings);

    let result = state.capture_manager.capture_now(&app_handle).await?;

    Ok(result.file_path.to_string_lossy().to_string())
}

/// Get current capture status
#[tauri::command]
pub async fn get_capture_status(state: State<'_, Arc<AppState>>) -> Result<CaptureStatus> {
    Ok(state.capture_manager.get_status())
}

/// Get list of running applications (for whitelist/blacklist configuration)
#[tauri::command]
pub async fn get_running_applications() -> Result<Vec<AppInfo>> {
    Ok(get_running_apps())
}

/// Update capture settings
#[tauri::command]
pub async fn update_capture_settings(
    state: State<'_, Arc<AppState>>,
    enabled: bool,
    interval_secs: u64,
    mode: String,
    filter_mode: String,
    app_list: Vec<String>,
    retention_days: u32,
    hotkey: String,
    app_handle: AppHandle,
) -> Result<()> {
    tracing::info!(
        "Updating capture settings: enabled={}, interval={}s, mode={}, filter={}",
        enabled,
        interval_secs,
        mode,
        filter_mode
    );

    let was_enabled = state.settings.read().screen_capture_enabled;

    // Update state settings
    {
        let mut settings = state.settings.write();
        settings.screen_capture_enabled = enabled;
        settings.capture_interval_secs = interval_secs.clamp(30, 300);
        settings.capture_mode = mode.clone();
        settings.capture_app_filter = filter_mode.clone();
        settings.capture_app_list = app_list.clone();
        settings.capture_retention_days = retention_days.clamp(1, 90);
        settings.capture_hotkey = hotkey.clone();
    }
    state.save_settings()?;

    // Create capture settings
    let capture_settings = CaptureSettings {
        enabled,
        interval_secs: interval_secs.clamp(30, 300),
        mode: mode.parse().unwrap_or(CaptureMode::ActiveWindow),
        filter_mode: filter_mode.parse().unwrap_or(AppFilterMode::None),
        app_list,
        retention_days: retention_days.clamp(1, 90),
        hotkey,
    };

    // Update capture manager
    state.capture_manager.update_settings(capture_settings);

    // Handle scheduler state changes
    if enabled && !was_enabled {
        // Start scheduler if newly enabled
        state.capture_manager.clone().start_scheduler(app_handle);
    } else if !enabled && was_enabled {
        // Stop scheduler if disabled (synchronous)
        state.capture_manager.stop_scheduler();
    }

    Ok(())
}

/// Pause screen capture temporarily (scheduler keeps running)
#[tauri::command]
pub async fn pause_screen_capture(state: State<'_, Arc<AppState>>) -> Result<()> {
    state.capture_manager.pause_scheduler();
    Ok(())
}

/// Resume screen capture
#[tauri::command]
pub async fn resume_screen_capture(state: State<'_, Arc<AppState>>) -> Result<()> {
    state.capture_manager.resume_scheduler();
    Ok(())
}

/// Clean up old captures based on retention settings
#[tauri::command]
pub async fn cleanup_old_captures(state: State<'_, Arc<AppState>>) -> Result<u64> {
    state.capture_manager.cleanup_old_captures()
}

/// Get capture settings from the state
fn get_capture_settings_from_state(state: &State<'_, Arc<AppState>>) -> CaptureSettings {
    let settings = state.settings.read();
    CaptureSettings {
        enabled: settings.screen_capture_enabled,
        interval_secs: settings.capture_interval_secs,
        mode: settings.capture_mode.parse().unwrap_or(CaptureMode::ActiveWindow),
        filter_mode: settings.capture_app_filter.parse().unwrap_or(AppFilterMode::None),
        app_list: settings.capture_app_list.clone(),
        retention_days: settings.capture_retention_days,
        hotkey: settings.capture_hotkey.clone(),
    }
}
