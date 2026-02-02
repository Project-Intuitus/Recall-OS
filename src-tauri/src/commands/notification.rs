//! Commands for notification window management.

use tauri::{command, AppHandle, Emitter, Manager, Runtime, Window};
use crate::notifications::{show_notification, NotificationData};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Global storage for pending notification data
static PENDING_NOTIFICATIONS: OnceLock<RwLock<HashMap<String, NotificationData>>> = OnceLock::new();

fn get_pending_notifications() -> &'static RwLock<HashMap<String, NotificationData>> {
    PENDING_NOTIFICATIONS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Store notification data for a window to retrieve
pub fn store_notification_data(window_label: &str, data: NotificationData) {
    get_pending_notifications().write().insert(window_label.to_string(), data);
}

/// Called by notification window when it's ready to receive data
#[command]
pub async fn notification_window_ready<R: Runtime>(window: Window<R>) -> Result<NotificationData, String> {
    let label = window.label();
    tracing::debug!("Notification window '{}' requesting data", label);

    // Try to get the stored notification data
    if let Some(data) = get_pending_notifications().write().remove(label) {
        tracing::debug!("Returning notification data for window '{}'", label);
        Ok(data)
    } else {
        tracing::warn!("No notification data found for window '{}'", label);
        Err("No notification data available".to_string())
    }
}

/// Test notification - shows a sample notification window
#[command]
pub async fn test_notification<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let data = NotificationData {
        title: "Test Notification".to_string(),
        message: "This is a test notification from RECALL.OS!".to_string(),
        document_id: None,
        related_documents: None,
    };

    show_notification(&app, data).map_err(|e| e.to_string())?;
    tracing::info!("Test notification sent");
    Ok(())
}

/// Focus the main application window
#[command]
pub async fn focus_main_window<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        // Show window first (in case it was hidden via close-to-tray)
        window.show().map_err(|e| e.to_string())?;
        // Unminimize in case it was minimized
        window.unminimize().map_err(|e| e.to_string())?;
        // Finally set focus to bring to front
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Focus the main window and highlight specific documents in the sidebar
#[command]
pub async fn focus_main_window_with_highlights<R: Runtime>(
    app: AppHandle<R>,
    document_ids: Vec<String>,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        // Show window first (in case it was hidden via close-to-tray)
        window.show().map_err(|e| e.to_string())?;
        // Unminimize in case it was minimized
        window.unminimize().map_err(|e| e.to_string())?;
        // Finally set focus to bring to front
        window.set_focus().map_err(|e| e.to_string())?;

        // Small delay to ensure window is fully visible and event listeners are ready
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Emit event to highlight documents
        let count = document_ids.len();
        app.emit("highlight-documents", document_ids)
            .map_err(|e| e.to_string())?;

        tracing::debug!("Emitted highlight-documents event with {} IDs", count);
    }
    Ok(())
}
