//! Periodic capture scheduler using tokio
//! Manages automatic screenshot capture at configurable intervals

use super::CaptureManager;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Runtime};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Message types for the scheduler
#[allow(dead_code)]
pub enum SchedulerMessage {
    /// Stop the scheduler
    Stop,
    /// Update the capture interval
    UpdateInterval(u64),
    /// Pause capturing temporarily
    Pause,
    /// Resume capturing
    Resume,
}

/// Periodic capture scheduler
pub struct CaptureScheduler {
    /// Whether the scheduler is currently running
    is_running: Arc<AtomicBool>,
    /// Whether capturing is paused (but scheduler still running)
    is_paused: Arc<AtomicBool>,
    /// Channel to send control messages
    tx: Option<mpsc::Sender<SchedulerMessage>>,
    /// Join handle for the scheduler task
    task_handle: Option<JoinHandle<()>>,
}

impl Default for CaptureScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureScheduler {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            is_paused: Arc::new(AtomicBool::new(false)),
            tx: None,
            task_handle: None,
        }
    }

    /// Start the periodic capture scheduler
    ///
    /// # Arguments
    /// * `capture_manager` - The capture manager to use for taking screenshots
    /// * `interval_secs` - Interval between captures in seconds
    /// * `app_handle` - Tauri app handle for emitting events
    pub fn start<R: Runtime + 'static>(
        &mut self,
        capture_manager: Arc<CaptureManager>,
        interval_secs: u64,
        app_handle: AppHandle<R>,
    ) {
        if self.is_running.load(Ordering::SeqCst) {
            tracing::warn!("Capture scheduler already running");
            return;
        }

        let (tx, rx) = mpsc::channel(16);
        self.tx = Some(tx);
        self.is_running.store(true, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);

        let is_running = self.is_running.clone();
        let is_paused = self.is_paused.clone();

        let handle = tokio::spawn(async move {
            Self::run_scheduler(
                capture_manager,
                interval_secs,
                app_handle,
                rx,
                is_running,
                is_paused,
            )
            .await;
        });

        self.task_handle = Some(handle);
        tracing::info!("Capture scheduler started with {}s interval", interval_secs);
    }

    /// Stop the scheduler (async version that waits for cleanup)
    pub async fn stop(&mut self) {
        if !self.is_running.load(Ordering::SeqCst) {
            return;
        }

        self.is_running.store(false, Ordering::SeqCst);

        if let Some(tx) = self.tx.take() {
            let _ = tx.send(SchedulerMessage::Stop).await;
        }

        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }

        tracing::info!("Capture scheduler stopped");
    }

    /// Signal the scheduler to stop (synchronous, non-blocking)
    /// The scheduler will stop on its next iteration
    pub fn signal_stop(&mut self) {
        if !self.is_running.load(Ordering::SeqCst) {
            return;
        }

        self.is_running.store(false, Ordering::SeqCst);

        // Try to send stop message without blocking
        if let Some(tx) = self.tx.take() {
            let _ = tx.try_send(SchedulerMessage::Stop);
        }

        // Don't wait for task handle - it will stop on its own
        self.task_handle = None;

        tracing::info!("Capture scheduler stop signaled");
    }

    /// Pause capturing (scheduler keeps running but doesn't capture)
    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
        tracing::info!("Capture scheduler paused");
    }

    /// Resume capturing
    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
        tracing::info!("Capture scheduler resumed");
    }

    /// Update the capture interval
    pub async fn update_interval(&self, interval_secs: u64) {
        if let Some(ref tx) = self.tx {
            let _ = tx.send(SchedulerMessage::UpdateInterval(interval_secs)).await;
        }
    }

    /// Check if the scheduler is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Check if capturing is paused
    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    /// The main scheduler loop
    async fn run_scheduler<R: Runtime>(
        capture_manager: Arc<CaptureManager>,
        initial_interval_secs: u64,
        app_handle: AppHandle<R>,
        mut rx: mpsc::Receiver<SchedulerMessage>,
        is_running: Arc<AtomicBool>,
        is_paused: Arc<AtomicBool>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(initial_interval_secs));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // Skip the first immediate tick
        interval.tick().await;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !is_running.load(Ordering::SeqCst) {
                        break;
                    }

                    if is_paused.load(Ordering::SeqCst) {
                        continue;
                    }

                    // Perform capture
                    match capture_manager.capture_and_ingest(&app_handle).await {
                        Ok(result) => {
                            tracing::debug!(
                                "Scheduled capture completed: {:?}",
                                result.file_path
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Scheduled capture failed: {}", e);
                        }
                    }
                }
                msg = rx.recv() => {
                    match msg {
                        Some(SchedulerMessage::Stop) | None => {
                            break;
                        }
                        Some(SchedulerMessage::UpdateInterval(new_interval)) => {
                            tracing::info!("Updating capture interval to {}s", new_interval);
                            interval = tokio::time::interval(Duration::from_secs(new_interval));
                            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                            interval.tick().await; // Skip immediate tick
                        }
                        Some(SchedulerMessage::Pause) => {
                            is_paused.store(true, Ordering::SeqCst);
                        }
                        Some(SchedulerMessage::Resume) => {
                            is_paused.store(false, Ordering::SeqCst);
                        }
                    }
                }
            }
        }

        is_running.store(false, Ordering::SeqCst);
        tracing::info!("Capture scheduler loop ended");
    }
}

impl Drop for CaptureScheduler {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
    }
}
