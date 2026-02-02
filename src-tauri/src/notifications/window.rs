//! Custom notification window management.
//!
//! Creates a frameless, always-on-top window at the bottom-right corner
//! for displaying rich notifications with full CSS styling control.

use serde::Serialize;
use tauri::{AppHandle, Runtime, WebviewUrl, WebviewWindowBuilder};

/// Data sent to the notification window
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationData {
    pub title: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_documents: Option<Vec<RelatedDocumentInfo>>,
}

/// Simplified related document info for notifications
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedDocumentInfo {
    pub id: String,
    pub title: String,
    pub similarity: f64,
}

/// Notification window dimensions (initial size, will be resized by frontend)
const NOTIFICATION_WIDTH: f64 = 356.0;  // 340 content + 16 padding
const NOTIFICATION_HEIGHT: f64 = 120.0; // Small initial height, will auto-resize
const NOTIFICATION_MARGIN: f64 = 16.0;

/// Show a custom notification window
pub fn show_notification<R: Runtime>(
    app: &AppHandle<R>,
    data: NotificationData,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Generate unique label for this notification
    let label = format!("notification-{}", uuid::Uuid::new_v4());

    // Get primary monitor to position the window
    let (screen_width, screen_height) = get_screen_dimensions(app)?;

    // Calculate position (bottom-right corner with margin)
    let x = screen_width - NOTIFICATION_WIDTH - NOTIFICATION_MARGIN;
    let y = screen_height - NOTIFICATION_HEIGHT - NOTIFICATION_MARGIN - 40.0; // Extra 40px for taskbar

    // Create the notification window
    let _window = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::App("index.html?notification=true".into()),
    )
    .title("RECALL.OS Notification")
    .inner_size(NOTIFICATION_WIDTH, NOTIFICATION_HEIGHT)
    .position(x, y)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .focused(false)
    .transparent(true)
    .shadow(false)  // Disable shadow to remove the border outline on Windows
    .visible(true)
    .build()?;

    // Store notification data for the window to retrieve via command
    crate::commands::notification::store_notification_data(&label, data);

    tracing::info!(
        "Created notification window '{}' at ({}, {})",
        label,
        x,
        y
    );

    Ok(())
}

/// Get screen dimensions from the primary monitor
fn get_screen_dimensions<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<(f64, f64), Box<dyn std::error::Error + Send + Sync>> {
    // Try to get the primary monitor
    if let Some(monitor) = app.primary_monitor()? {
        let size = monitor.size();
        return Ok((size.width as f64, size.height as f64));
    }

    // Fallback to available monitors
    let monitors = app.available_monitors()?;
    if let Some(monitor) = monitors.first() {
        let size = monitor.size();
        return Ok((size.width as f64, size.height as f64));
    }

    // Default fallback
    Ok((1920.0, 1080.0))
}

/// Show a notification for related content found
pub fn show_related_content_notification<R: Runtime>(
    app: &AppHandle<R>,
    new_document_id: &str,
    document_title: &str,
    related: &[(String, String, f64)], // (id, title, similarity)
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let related_docs: Vec<RelatedDocumentInfo> = related
        .iter()
        .map(|(id, title, sim)| RelatedDocumentInfo {
            id: id.clone(),
            title: title.clone(),
            similarity: *sim,
        })
        .collect();

    let data = NotificationData {
        title: document_title.to_string(),
        message: format!(
            "{} similar {} found",
            related.len(),
            if related.len() == 1 { "document" } else { "documents" }
        ),
        document_id: Some(new_document_id.to_string()),
        related_documents: Some(related_docs),
    };

    show_notification(app, data)
}

/// Show a notification for a screenshot being processed
pub fn show_processing_notification<R: Runtime>(
    app: &AppHandle<R>,
    source_app: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let title = "Processing Screenshot".to_string();
    let message = match source_app {
        Some(app_name) => format!("From {}", app_name),
        None => "Extracting text...".to_string(),
    };

    let data = NotificationData {
        title,
        message,
        document_id: None,
        related_documents: None,
    };

    show_notification(app, data)
}
