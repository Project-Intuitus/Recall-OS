//! Notification module with custom window and AUMID branding support.
//!
//! This module provides:
//! - Custom notification windows with full CSS styling control
//! - AUMID registration for Windows toast fallback
//! - Toast notification builders with RECALL.OS styling

#[cfg(windows)]
mod aumid;
#[cfg(windows)]
mod toast;
mod window;

#[cfg(windows)]
pub use aumid::ensure_aumid_registered;
#[cfg(windows)]
pub use toast::NotificationBuilder;
pub use window::{show_notification, show_related_content_notification, show_processing_notification, NotificationData, RelatedDocumentInfo};

/// The Application User Model ID for RECALL.OS
/// This must match the identifier in tauri.conf.json
pub const AUMID: &str = "com.recallos.app";
