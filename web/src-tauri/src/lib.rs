//! Recap - Work tracking and reporting tool
//!
//! A Tauri application for work item management.

mod api;
mod auth;
mod commands;
mod db;
mod models;
mod services;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent, WindowEvent,
};

static SERVER_STARTED: AtomicBool = AtomicBool::new(false);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        // Register Tauri commands
        .invoke_handler(tauri::generate_handler![
            // Auth
            commands::auth::get_app_status,
            commands::auth::register_user,
            commands::auth::login,
            commands::auth::auto_login,
            commands::auth::get_current_user,
            // Config
            commands::config::get_config,
            commands::config::update_config,
            commands::config::update_llm_config,
            commands::config::update_jira_config,
            // Work Items
            commands::work_items::list_work_items,
            commands::work_items::create_work_item,
            commands::work_items::get_work_item,
            commands::work_items::update_work_item,
            commands::work_items::delete_work_item,
            commands::work_items::get_stats_summary,
            commands::work_items::get_grouped_work_items,
            commands::work_items::get_timeline_data,
            commands::work_items::batch_sync_tempo,
            commands::work_items::aggregate_work_items,
            // Claude
            commands::claude::list_claude_sessions,
            commands::claude::import_claude_sessions,
            commands::claude::summarize_claude_session,
            commands::claude::sync_claude_projects,
            // Reports
            commands::reports::get_personal_report,
            commands::reports::get_summary_report,
            commands::reports::get_category_report,
            commands::reports::get_source_report,
            commands::reports::export_excel_report,
            // Sync
            commands::sync::get_sync_status,
            commands::sync::auto_sync,
            commands::sync::list_available_projects,
            // GitLab
            commands::gitlab::get_gitlab_status,
            commands::gitlab::configure_gitlab,
            commands::gitlab::remove_gitlab_config,
            commands::gitlab::list_gitlab_projects,
            commands::gitlab::add_gitlab_project,
            commands::gitlab::remove_gitlab_project,
            commands::gitlab::sync_gitlab,
            commands::gitlab::search_gitlab_projects,
            // Tempo
            commands::tempo::test_tempo_connection,
            commands::tempo::validate_jira_issue,
            commands::tempo::sync_worklogs_to_tempo,
            commands::tempo::upload_single_worklog,
            commands::tempo::get_tempo_worklogs,
        ])
        .setup(|app| {
            // Setup logging
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .build(),
            )?;

            // Initialize database and app state
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match db::Database::new().await {
                    Ok(database) => {
                        log::info!("Database initialized successfully");
                        let state = commands::AppState::new(database.clone());
                        app_handle.manage(state);

                        // Also start the legacy Axum API server for gradual migration
                        start_api_server_with_db(database).await;
                    }
                    Err(e) => {
                        log::error!("Failed to initialize database: {}", e);
                    }
                }
            });

            // Create tray menu
            let show_item = MenuItem::with_id(app, "show", "Show Recap", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // Create tray icon
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
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

            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide window instead of closing when user clicks close button
            if let WindowEvent::CloseRequested { api, .. } = event {
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = window.hide();
                    api.prevent_close();
                }
                #[cfg(target_os = "macos")]
                {
                    let _ = tauri::AppHandle::hide(&window.app_handle());
                    api.prevent_close();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}

/// Start the Axum API server (legacy, for gradual migration)
async fn start_api_server_with_db(db: db::Database) {
    if SERVER_STARTED.load(Ordering::SeqCst) {
        return;
    }

    log::info!("Starting legacy Axum API server...");

    // Create router
    let app = api::create_router(db);

    // Bind to localhost
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    log::info!("API server listening on http://{}", addr);

    SERVER_STARTED.store(true, Ordering::SeqCst);

    // Start server
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            log::error!("Failed to bind to {}: {}", addr, e);
            SERVER_STARTED.store(false, Ordering::SeqCst);
            return;
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        log::error!("Server error: {}", e);
        SERVER_STARTED.store(false, Ordering::SeqCst);
    }
}
