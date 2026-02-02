use crate::database::FileType;
use crate::error::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use tokio::sync::mpsc;

pub struct FileWatcher {
    watcher: RecommendedWatcher,
    #[allow(dead_code)]
    tx: mpsc::Sender<WatchEvent>, // Kept alive to prevent channel closure
}

#[derive(Debug, Clone)]
pub enum WatchEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}

impl FileWatcher {
    pub fn new() -> Result<(Self, mpsc::Receiver<WatchEvent>)> {
        // Increased capacity to reduce event loss during burst activity
        let (tx, rx) = mpsc::channel(1000);
        let tx_clone = tx.clone();

        let watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            match res {
                Ok(event) => {
                    tracing::debug!("File event: {:?}", event.kind);

                    for path in event.paths {
                        // Only watch supported file types
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        let file_type = FileType::from_extension(ext);

                        tracing::debug!("File: {:?}, ext: {}, type: {:?}", path, ext, file_type);

                        if matches!(file_type, FileType::Unknown) {
                            tracing::debug!("Skipping unknown file type: {:?}", path);
                            continue;
                        }

                        let watch_event = match event.kind {
                            notify::EventKind::Create(_) => {
                                tracing::info!("File created: {:?}", path);
                                Some(WatchEvent::Created(path))
                            }
                            notify::EventKind::Modify(_) => {
                                tracing::info!("File modified: {:?}", path);
                                Some(WatchEvent::Modified(path))
                            }
                            notify::EventKind::Remove(_) => {
                                tracing::info!("File removed: {:?}", path);
                                Some(WatchEvent::Deleted(path))
                            }
                            _ => None,
                        };

                        if let Some(evt) = watch_event {
                            // Use try_send to avoid blocking - it's non-async safe
                            match tx_clone.try_send(evt) {
                                Ok(_) => tracing::debug!("Event sent to processor"),
                                Err(tokio::sync::mpsc::error::TrySendError::Full(evt)) => {
                                    tracing::error!(
                                        "Watcher event queue full, dropping event: {:?}. Consider increasing channel capacity.",
                                        evt
                                    );
                                }
                                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                    tracing::error!("Watcher event channel closed unexpectedly");
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Watch error: {}", e);
                }
            }
        })?;

        Ok((Self { watcher, tx }, rx))
    }

    pub fn watch(&mut self, path: &PathBuf) -> Result<()> {
        self.watcher.watch(path.as_path(), RecursiveMode::Recursive)?;
        tracing::info!("Watching directory: {:?}", path);
        Ok(())
    }

    pub fn unwatch(&mut self, path: &PathBuf) -> Result<()> {
        self.watcher.unwatch(path.as_path())?;
        tracing::info!("Stopped watching directory: {:?}", path);
        Ok(())
    }
}
