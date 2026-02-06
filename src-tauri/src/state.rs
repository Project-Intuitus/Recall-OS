use crate::capture::CaptureManager;
use crate::database::Database;
use crate::error::{RecallError, Result};
use crate::ingestion::{IngestionEngine, WatcherManager};
use crate::llm::LlmClient;
use crate::rag::RagEngine;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub gemini_api_key: Option<String>,
    pub embedding_model: String,
    pub ingestion_model: String,
    pub reasoning_model: String,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub max_context_chunks: usize,
    pub video_segment_duration: u64,
    pub keyframe_interval: f64,
    #[serde(default)]
    pub watched_folders: Vec<String>,
    #[serde(default = "default_auto_ingest")]
    pub auto_ingest_enabled: bool,
    // Screen capture settings
    #[serde(default)]
    pub screen_capture_enabled: bool,
    #[serde(default = "default_capture_interval")]
    pub capture_interval_secs: u64,
    #[serde(default = "default_capture_mode")]
    pub capture_mode: String,
    #[serde(default = "default_capture_filter")]
    pub capture_app_filter: String,
    #[serde(default)]
    pub capture_app_list: Vec<String>,
    #[serde(default = "default_capture_retention")]
    pub capture_retention_days: u32,
    #[serde(default = "default_capture_hotkey")]
    pub capture_hotkey: String,
    // License settings
    #[serde(default)]
    pub license_key: Option<String>,
    #[serde(default)]
    pub license_activated_at: Option<String>,
    #[serde(default)]
    pub license_customer_name: Option<String>,
    #[serde(default)]
    pub license_customer_email: Option<String>,
    #[serde(default)]
    pub license_instance_id: Option<String>,
}

fn default_auto_ingest() -> bool {
    false
}

fn default_capture_interval() -> u64 {
    60
}

fn default_capture_mode() -> String {
    "active_window".to_string()
}

fn default_capture_filter() -> String {
    "none".to_string()
}

fn default_capture_retention() -> u32 {
    7
}

fn default_capture_hotkey() -> String {
    "Ctrl+Shift+S".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gemini_api_key: None,
            embedding_model: "gemini-embedding-001".to_string(),
            ingestion_model: "gemini-2.0-flash".to_string(),
            reasoning_model: "gemini-2.0-flash".to_string(),
            chunk_size: 512,
            chunk_overlap: 50,
            max_context_chunks: 20,
            video_segment_duration: 300,
            keyframe_interval: 0.2,
            watched_folders: Vec::new(),
            auto_ingest_enabled: false,
            screen_capture_enabled: false,
            capture_interval_secs: 60,
            capture_mode: "active_window".to_string(),
            capture_app_filter: "none".to_string(),
            capture_app_list: Vec::new(),
            capture_retention_days: 7,
            capture_hotkey: "Ctrl+Shift+S".to_string(),
            license_key: None,
            license_activated_at: None,
            license_customer_name: None,
            license_customer_email: None,
            license_instance_id: None,
        }
    }
}

pub struct AppState {
    pub database: Arc<Database>,
    pub llm_client: Arc<RwLock<Option<LlmClient>>>,
    pub ingestion_engine: Arc<IngestionEngine>,
    pub rag_engine: Arc<RagEngine>,
    pub settings: Arc<RwLock<Settings>>,
    pub watcher_manager: Arc<WatcherManager>,
    pub capture_manager: Arc<CaptureManager>,
    pub app_data_dir: PathBuf,
}

impl AppState {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e: tauri::Error| RecallError::Config(e.to_string()))?;

        std::fs::create_dir_all(&app_data_dir)?;

        let db_path = app_data_dir.join("recall.db");
        let resources_dir = app_handle
            .path()
            .resource_dir()
            .map_err(|e: tauri::Error| RecallError::Config(e.to_string()))?;

        let database = Arc::new(Database::new(&db_path, &resources_dir)?);
        let settings = Arc::new(RwLock::new(Self::load_settings(&app_data_dir)));

        let llm_client = Arc::new(RwLock::new(None));

        let ingestion_engine = Arc::new(IngestionEngine::new(
            database.clone(),
            llm_client.clone(),
            settings.clone(),
        ));

        let rag_engine = Arc::new(RagEngine::new(
            database.clone(),
            llm_client.clone(),
            settings.clone(),
        ));

        let watcher_manager = Arc::new(WatcherManager::new());

        // Initialize capture manager
        let capture_manager = Arc::new(CaptureManager::new(
            app_data_dir.clone(),
            database.clone(),
            llm_client.clone(),
            ingestion_engine.clone(),
        )?);

        // Initialize LLM client if API key exists
        {
            let settings_guard = settings.read();
            if let Some(ref api_key) = settings_guard.gemini_api_key {
                let client = LlmClient::new(api_key.clone());
                *llm_client.write() = Some(client);
            }
        }

        Ok(Self {
            database,
            llm_client,
            ingestion_engine,
            rag_engine,
            settings,
            watcher_manager,
            capture_manager,
            app_data_dir,
        })
    }

    fn load_settings(app_data_dir: &PathBuf) -> Settings {
        let settings_path = app_data_dir.join("settings.json");
        let mut settings = if settings_path.exists() {
            std::fs::read_to_string(&settings_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Settings::default()
        };

        // Migrate deprecated embedding model
        if settings.embedding_model == "text-embedding-004" {
            tracing::info!("Migrating embedding model from text-embedding-004 to gemini-embedding-001");
            settings.embedding_model = "gemini-embedding-001".to_string();
        }

        // Check for API key from environment variable (for development)
        if settings.gemini_api_key.is_none() {
            if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
                tracing::info!("Using GEMINI_API_KEY from environment variable");
                settings.gemini_api_key = Some(api_key);
            }
        }

        settings
    }

    pub fn save_settings(&self) -> Result<()> {
        let settings_path = self.app_data_dir.join("settings.json");
        let settings = self.settings.read();
        let json = serde_json::to_string_pretty(&*settings)?;
        std::fs::write(settings_path, json)?;
        Ok(())
    }

    pub fn update_llm_client(&self, api_key: String) {
        let client = LlmClient::new(api_key);
        *self.llm_client.write() = Some(client);
    }

    /// Start the file watcher with configured folders
    pub fn start_watcher<R: Runtime + 'static>(&self, app_handle: AppHandle<R>) -> Result<()> {
        let settings = self.settings.read();

        tracing::info!(
            "start_watcher called: auto_ingest={}, folders={:?}",
            settings.auto_ingest_enabled,
            settings.watched_folders
        );

        if !settings.auto_ingest_enabled {
            tracing::info!("Auto-ingest disabled, not starting watcher");
            return Ok(());
        }

        if settings.watched_folders.is_empty() {
            tracing::info!("No watched folders configured");
            return Ok(());
        }

        let folders = settings.watched_folders.clone();
        drop(settings);

        // Start the watcher
        self.watcher_manager.start()?;

        // Add all configured folders
        for folder in &folders {
            let path = PathBuf::from(folder);
            if path.exists() && path.is_dir() {
                tracing::info!("Adding watch for folder: {:?}", path);
                if let Err(e) = self.watcher_manager.add_folder(path.clone()) {
                    tracing::warn!("Failed to watch folder {:?}: {}", path, e);
                } else {
                    tracing::info!("Successfully watching folder: {:?}", path);
                }
            } else {
                tracing::warn!("Folder does not exist or is not a directory: {:?}", path);
            }
        }

        // Start the event processor
        self.watcher_manager.spawn_event_processor(
            app_handle,
            self.ingestion_engine.clone(),
            self.database.clone(),
        );

        tracing::info!("File watcher setup complete");
        Ok(())
    }

    /// Stop the file watcher
    pub fn stop_watcher(&self) {
        self.watcher_manager.stop();
    }
}
