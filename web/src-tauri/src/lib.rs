//! Recap - Work tracking and reporting tool
//!
//! A Tauri application with embedded Axum server for work item management.

mod api;
mod auth;
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
        .setup(|app| {
            // Setup logging
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .build(),
            )?;

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

            // Start the Axum API server
            tauri::async_runtime::spawn(async move {
                start_api_server().await;
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

/// Start the Axum API server
async fn start_api_server() {
    if SERVER_STARTED.load(Ordering::SeqCst) {
        return;
    }

    log::info!("Starting Recap API server...");

    // Initialize database
    let db = match db::Database::new().await {
        Ok(db) => {
            log::info!("Database initialized successfully");
            db
        }
        Err(e) => {
            log::error!("Failed to initialize database: {}", e);
            return;
        }
    };

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
