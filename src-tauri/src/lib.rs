pub mod capture;
pub mod commands;
pub mod database;
pub mod error;
pub mod ingestion;
pub mod llm;
pub mod notifications;
pub mod rag;
pub mod state;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load environment variables from .env file (for development)
    let _ = dotenvy::dotenv();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "recall_os=debug,tauri=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting RECALL.OS");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Register AUMID for proper Windows notification branding
            #[cfg(windows)]
            {
                use tauri::Manager;

                // Use small 32x32 icon for notifications
                let icon_path = app.path()
                    .resource_dir()
                    .ok()
                    .map(|p| p.join("icons").join("32x32.png"))
                    .filter(|p| p.exists())
                    .or_else(|| {
                        let exe_dir = std::env::current_exe().ok()?;
                        let project_dir = exe_dir.parent()?.parent()?.parent()?;
                        let path = project_dir.join("icons").join("32x32.png");
                        if path.exists() { Some(path) } else { None }
                    });

                if let Some(ref path) = icon_path {
                    let normalized = {
                        let s = path.to_string_lossy();
                        if s.starts_with(r"\\?\") {
                            std::path::PathBuf::from(&s[4..])
                        } else {
                            path.clone()
                        }
                    };
                    if let Err(e) = notifications::ensure_aumid_registered(&normalized) {
                        tracing::warn!("AUMID registration failed: {}", e);
                    }
                }
            }

            // Initialize application state
            let state = Arc::new(AppState::new(&app_handle)?);

            // Start file watcher if auto-ingest is enabled
            if let Err(e) = state.start_watcher(app_handle.clone()) {
                tracing::warn!("Failed to start file watcher: {}", e);
            }

            // Start screen capture scheduler if enabled
            {
                let settings = state.settings.read();
                if settings.screen_capture_enabled {
                    tracing::info!("Starting screen capture scheduler");
                    state.capture_manager.clone().start_scheduler(app_handle.clone());
                }
            }

            // Register global hotkey for screen capture
            let hotkey_str = state.settings.read().capture_hotkey.clone();
            if let Ok(shortcut) = hotkey_str.parse::<Shortcut>() {
                let state_for_hotkey = state.clone();
                let app_handle_for_hotkey = app_handle.clone();

                if let Err(e) = app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        tracing::info!("Global hotkey triggered for screen capture");
                        let state = state_for_hotkey.clone();
                        let app_handle = app_handle_for_hotkey.clone();

                        // Spawn async task for capture
                        tauri::async_runtime::spawn(async move {
                            match state.capture_manager.capture_now(&app_handle).await {
                                Ok(result) => {
                                    tracing::info!("Hotkey capture successful: {:?}", result.file_path);
                                }
                                Err(e) => {
                                    tracing::warn!("Hotkey capture failed: {}", e);
                                }
                            }
                        });
                    }
                }) {
                    tracing::warn!("Failed to register global shortcut '{}': {}", hotkey_str, e);
                } else {
                    tracing::info!("Registered global shortcut: {}", hotkey_str);
                }
            } else {
                tracing::warn!("Invalid hotkey format: {}", hotkey_str);
            }

            // Set up system tray
            let show_item = MenuItem::with_id(app, "show", "Show RECALL.OS", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .tooltip("RECALL.OS")
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            tracing::info!("System tray initialized");

            // Set high-resolution window icon for crisp taskbar display
            #[cfg(windows)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    // Try to load the high-res icon.png (512x512)
                    let icon_path = app.path()
                        .resource_dir()
                        .ok()
                        .map(|p| p.join("icons").join("icon.png"))
                        .filter(|p| p.exists())
                        .or_else(|| {
                            let exe_dir = std::env::current_exe().ok()?;
                            let project_dir = exe_dir.parent()?.parent()?.parent()?;
                            let path = project_dir.join("src-tauri").join("icons").join("icon.png");
                            if path.exists() { Some(path) } else { None }
                        });

                    if let Some(path) = icon_path {
                        match image::open(&path) {
                            Ok(img) => {
                                let rgba = img.to_rgba8();
                                let (width, height) = rgba.dimensions();
                                let raw_data = rgba.into_raw();
                                let icon = tauri::image::Image::new_owned(raw_data, width, height);
                                if let Err(e) = window.set_icon(icon) {
                                    tracing::warn!("Failed to set window icon: {}", e);
                                } else {
                                    tracing::info!("Set high-resolution window icon ({}x{})", width, height);
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to load icon image: {}", e);
                            }
                        }
                    }
                }
            }

            app.manage(state);

            tracing::info!("RECALL.OS initialized successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Database commands
            commands::database::get_documents,
            commands::database::get_document,
            commands::database::delete_document,
            commands::database::get_chunks_for_document,
            commands::database::get_ingestion_stats,
            commands::database::open_file_in_default_app,
            commands::database::reset_database,
            commands::database::categorize_document,
            commands::database::categorize_all_documents,
            commands::database::get_content_categories,
            // Ingestion commands
            commands::ingestion::ingest_file,
            commands::ingestion::ingest_directory,
            commands::ingestion::cancel_ingestion,
            commands::ingestion::get_ingestion_progress,
            commands::ingestion::reingest_document,
            commands::ingestion::get_ingestion_queue,
            // Search commands
            commands::search::search_documents,
            commands::search::hybrid_search,
            // RAG commands
            commands::rag::query,
            commands::rag::query_with_sources,
            // Conversation commands
            commands::conversations::get_conversations,
            commands::conversations::get_conversation,
            commands::conversations::get_conversation_messages,
            commands::conversations::create_conversation,
            commands::conversations::delete_conversation,
            commands::conversations::rename_conversation,
            // Settings commands
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::validate_api_key,
            commands::settings::get_api_key_unmasked,
            commands::settings::clear_api_key,
            // Watcher commands
            commands::watcher::get_watcher_status,
            commands::watcher::start_watcher,
            commands::watcher::stop_watcher,
            commands::watcher::add_watched_folder,
            commands::watcher::remove_watched_folder,
            commands::watcher::toggle_auto_ingest,
            // Notification commands
            commands::notification::notification_window_ready,
            commands::notification::focus_main_window,
            commands::notification::focus_main_window_with_highlights,
            commands::notification::test_notification,
            // Capture commands
            commands::capture::start_screen_capture,
            commands::capture::stop_screen_capture,
            commands::capture::capture_now,
            commands::capture::get_capture_status,
            commands::capture::get_running_applications,
            commands::capture::update_capture_settings,
            commands::capture::pause_screen_capture,
            commands::capture::resume_screen_capture,
            commands::capture::cleanup_old_captures,
            // License commands
            commands::license::get_license_status,
            commands::license::activate_license,
            commands::license::deactivate_license,
            #[cfg(debug_assertions)]
            commands::license::generate_test_license,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
