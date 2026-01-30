//! Recap - Work tracking and reporting tool
//!
//! A Tauri application for work item management.

mod commands;
mod services;

// Re-export from recap-core for backwards compatibility
pub use recap_core::models;
pub use recap_core::services as core_services;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
    Emitter, Manager, RunEvent, WindowEvent,
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
            commands::sources::commands::get_sources,
            commands::sources::commands::add_git_repo,
            commands::sources::commands::remove_git_repo,
            commands::sources::commands::set_source_mode,
            // Claude
            commands::claude::list_claude_sessions,
            commands::claude::import_claude_sessions,
            commands::claude::summarize_claude_session,
            commands::claude::sync_claude_projects,
            // Antigravity (Gemini Code)
            commands::antigravity::check_antigravity_installed,
            commands::antigravity::check_antigravity_api_status,
            commands::antigravity::list_antigravity_sessions,
            commands::antigravity::sync_antigravity_projects,
            // Reports - queries
            commands::reports::queries::get_personal_report,
            commands::reports::queries::get_summary_report,
            commands::reports::queries::get_category_report,
            commands::reports::queries::get_source_report,
            commands::reports::queries::analyze_work_items,
            // Reports - export
            commands::reports::export::export_excel_report,
            commands::reports::export::generate_tempo_report,
            // Sync
            commands::sync::get_sync_status,
            commands::sync::auto_sync,
            commands::sync::list_available_projects,
            // GitLab - config
            commands::gitlab::config::get_gitlab_status,
            commands::gitlab::config::configure_gitlab,
            commands::gitlab::config::remove_gitlab_config,
            // GitLab - projects
            commands::gitlab::projects::list_gitlab_projects,
            commands::gitlab::projects::add_gitlab_project,
            commands::gitlab::projects::remove_gitlab_project,
            commands::gitlab::projects::search_gitlab_projects,
            // GitLab - sync
            commands::gitlab::sync::sync_gitlab,
            // Tempo
            commands::tempo::test_tempo_connection,
            commands::tempo::validate_jira_issue,
            commands::tempo::sync_worklogs_to_tempo,
            commands::tempo::get_tempo_worklogs,
            commands::tempo::search_jira_issues,
            commands::tempo::batch_get_jira_issues,
            commands::tempo::summarize_tempo_description,
            // Users
            commands::users::get_profile,
            commands::users::update_profile,
            // Tray
            commands::tray::update_tray_sync_status,
            commands::tray::set_tray_syncing,
            // Background Sync
            commands::background_sync::get_background_sync_config,
            commands::background_sync::update_background_sync_config,
            commands::background_sync::get_background_sync_status,
            commands::background_sync::start_background_sync,
            commands::background_sync::stop_background_sync,
            commands::background_sync::trigger_background_sync,
            commands::background_sync::trigger_sync_with_progress,
            // Notifications
            commands::notification::send_sync_notification,
            commands::notification::send_auth_notification,
            commands::notification::send_source_error_notification,
            // Snapshots & Compaction
            commands::snapshots::get_work_summaries,
            commands::snapshots::get_snapshot_detail,
            commands::snapshots::trigger_compaction,
            commands::snapshots::force_recompact,
            // Worklog
            commands::snapshots::get_worklog_overview,
            commands::snapshots::get_hourly_breakdown,
            // Worklog Sync
            commands::worklog_sync::get_project_issue_mappings,
            commands::worklog_sync::save_project_issue_mapping,
            commands::worklog_sync::get_worklog_sync_records,
            commands::worklog_sync::save_worklog_sync_record,
            // LLM Usage
            commands::llm_usage::get_llm_usage_stats,
            commands::llm_usage::get_llm_usage_daily,
            commands::llm_usage::get_llm_usage_by_model,
            commands::llm_usage::get_llm_usage_logs,
            // Projects
            commands::projects::queries::list_projects,
            commands::projects::queries::get_project_detail,
            commands::projects::queries::set_project_visibility,
            commands::projects::queries::get_hidden_projects,
            commands::projects::queries::get_project_directories,
            commands::projects::queries::get_claude_session_path,
            commands::projects::queries::update_claude_session_path,
            commands::projects::queries::get_antigravity_session_path,
            commands::projects::queries::update_antigravity_session_path,
            commands::projects::queries::add_manual_project,
            commands::projects::queries::remove_manual_project,
            // Projects - descriptions
            commands::projects::descriptions::get_project_description,
            commands::projects::descriptions::update_project_description,
            commands::projects::descriptions::delete_project_description,
            // Projects - timeline
            commands::projects::timeline::get_project_timeline,
            // Projects - summaries (unified)
            commands::projects::summaries::get_cached_summary,
            commands::projects::summaries::get_cached_summaries_batch,
            commands::projects::summaries::trigger_summaries_generation,
            commands::projects::summaries::generate_completed_summaries,
            // Projects - summaries (legacy)
            commands::projects::summaries::get_project_summary,
            commands::projects::summaries::generate_project_summary,
            commands::projects::summaries::check_summary_freshness,
            // Projects - git diff
            commands::projects::git_diff::get_commit_diff,
            // Danger Zone
            commands::danger_zone::clear_synced_data,
            commands::danger_zone::factory_reset,
            commands::danger_zone::force_recompact_with_progress,
            // Batch Compaction (OpenAI Batch API)
            commands::batch_compaction::check_batch_availability,
            commands::batch_compaction::get_pending_hourly_compactions,
            commands::batch_compaction::get_batch_job_status,
            commands::batch_compaction::submit_batch_compaction,
            commands::batch_compaction::refresh_batch_status,
            commands::batch_compaction::process_completed_batch_job,
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
            let show_item = MenuItem::with_id(app, "show", "開啟 Recap", true, None::<&str>)?;
            let sync_item = MenuItem::with_id(app, "sync_now", "立即同步", true, None::<&str>)?;
            let separator = MenuItem::with_id(app, "sep1", "─────────────", false, None::<&str>)?;
            let status_item = MenuItem::with_id(app, "status", "上次同步: -", false, None::<&str>)?;
            let separator2 = MenuItem::with_id(app, "sep2", "─────────────", false, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "結束 Recap", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &sync_item, &separator, &status_item, &separator2, &quit_item])?;

            // Get the tray icon created by tauri.conf.json and attach menu + events
            let tray = app.tray_by_id("main-tray").expect("tray icon not found");
            tray.set_menu(Some(menu))?;
            tray.set_show_menu_on_left_click(false)?;
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "sync_now" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.emit("tray-sync-now", ());
                    }
                    log::info!("Tray: Sync now triggered");
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            });
            tray.on_tray_icon_event(|tray, event| {
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
            });

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
