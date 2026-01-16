//! Recap - Work tracking and reporting tool
//!
//! A Tauri application for work item management.

mod commands;

// Re-export from recap-core for backwards compatibility
pub use recap_core::models;
pub use recap_core::services;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent, WindowEvent,
};

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
            commands::auth::commands::get_app_status,
            commands::auth::commands::register_user,
            commands::auth::commands::login,
            commands::auth::commands::auto_login,
            commands::auth::commands::get_current_user,
            // Config
            commands::config::get_config,
            commands::config::update_config,
            commands::config::update_llm_config,
            commands::config::update_jira_config,
            // Work Items - queries
            commands::work_items::queries::list_work_items,
            commands::work_items::queries::get_stats_summary,
            commands::work_items::queries::get_timeline_data,
            // Work Items - mutations
            commands::work_items::mutations::create_work_item,
            commands::work_items::mutations::get_work_item,
            commands::work_items::mutations::update_work_item,
            commands::work_items::mutations::delete_work_item,
            commands::work_items::mutations::map_work_item_jira,
            // Work Items - grouped
            commands::work_items::grouped::get_grouped_work_items,
            // Work Items - sync
            commands::work_items::sync::batch_sync_tempo,
            commands::work_items::sync::aggregate_work_items,
            // Work Items - commit centric
            commands::work_items::commit_centric::get_commit_centric_worklog,
            // Sources
            commands::sources::get_sources,
            commands::sources::add_git_repo,
            commands::sources::remove_git_repo,
            commands::sources::set_source_mode,
            // Claude
            commands::claude::list_claude_sessions,
            commands::claude::import_claude_sessions,
            commands::claude::summarize_claude_session,
            commands::claude::sync_claude_projects,
            // Reports - queries
            commands::reports::queries::get_personal_report,
            commands::reports::queries::get_summary_report,
            commands::reports::queries::get_category_report,
            commands::reports::queries::get_source_report,
            // Reports - export
            commands::reports::export::export_excel_report,
            commands::reports::export::generate_tempo_report,
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
            // Users
            commands::users::get_profile,
            commands::users::update_profile,
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
                match recap_core::Database::new().await {
                    Ok(database) => {
                        log::info!("Database initialized successfully");
                        let state = commands::AppState::new(database);
                        app_handle.manage(state);
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
