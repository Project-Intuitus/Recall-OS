//! Toast notification builders with RECALL.OS branding.

use std::path::PathBuf;

use super::AUMID;
use tauri_winrt_notification::{Duration, IconCrop, Toast};

pub struct NotificationBuilder {
    icon_path: Option<PathBuf>,
}

impl NotificationBuilder {
    pub fn new(icon_path: Option<PathBuf>) -> Self {
        Self { icon_path }
    }

    fn normalize_path(path: &PathBuf) -> Option<PathBuf> {
        let path_str = path.to_string_lossy();
        let normalized = if path_str.starts_with(r"\\?\") {
            PathBuf::from(&path_str[4..])
        } else {
            path.clone()
        };
        if normalized.exists() { Some(normalized) } else { None }
    }

    fn get_icon(&self) -> Option<PathBuf> {
        self.icon_path.as_ref().and_then(Self::normalize_path)
    }

    fn truncate(text: &str, max_len: usize) -> String {
        if text.len() > max_len {
            format!("{}...", &text[..max_len.saturating_sub(3)])
        } else {
            text.to_string()
        }
    }

    /// Related content notification
    pub fn related_content(
        &self,
        doc_title: &str,
        related_titles: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let count = related_titles.len();

        // Short title
        let title = format!("{} Match{}", count, if count != 1 { "es" } else { "" });

        // Document name - shorter truncation
        let line1 = Self::truncate(doc_title, 35);

        // Related docs - shorter names
        let line2 = related_titles
            .iter()
            .take(3)
            .map(|t| Self::truncate(t, 15))
            .collect::<Vec<_>>()
            .join(" · ");

        let mut toast = Toast::new(AUMID)
            .title(&title)
            .text1(&line1)
            .text2(&line2)
            .duration(Duration::Short);

        if let Some(icon) = self.get_icon() {
            toast = toast.icon(&icon, IconCrop::Circular, "RECALL.OS");
        }

        toast.show()?;
        tracing::info!("Notification: {} → {} related", doc_title, count);
        Ok(())
    }

    /// Document indexed notification
    pub fn ingestion_complete(
        &self,
        doc_title: &str,
        chunk_count: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut toast = Toast::new(AUMID)
            .title("Indexed")
            .text1(&Self::truncate(doc_title, 40))
            .text2(&format!("{} segments", chunk_count))
            .duration(Duration::Short);

        if let Some(icon) = self.get_icon() {
            toast = toast.icon(&icon, IconCrop::Circular, "RECALL.OS");
        }

        toast.show()?;
        tracing::info!("Notification: indexed {}", doc_title);
        Ok(())
    }

    /// Error notification
    pub fn error(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut toast = Toast::new(AUMID)
            .title("Error")
            .text1(&Self::truncate(message, 50))
            .duration(Duration::Short);

        if let Some(icon) = self.get_icon() {
            toast = toast.icon(&icon, IconCrop::Circular, "RECALL.OS");
        }

        toast.show()?;
        tracing::warn!("Notification error: {}", message);
        Ok(())
    }

    /// Info notification
    pub fn info(&self, title: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut toast = Toast::new(AUMID)
            .title(title)
            .text1(&Self::truncate(message, 50))
            .duration(Duration::Short);

        if let Some(icon) = self.get_icon() {
            toast = toast.icon(&icon, IconCrop::Circular, "RECALL.OS");
        }

        toast.show()?;
        tracing::info!("Notification: {} - {}", title, message);
        Ok(())
    }
}
