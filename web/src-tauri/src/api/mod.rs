//! API module - Axum routes

pub mod auth;
pub mod claude;
pub mod config;
pub mod gitlab;
pub mod reports;
pub mod sync;
pub mod tempo;
pub mod users;
pub mod work_items;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::db::Database;

/// Create the API router with all routes
pub fn create_router(db: Database) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .nest("/api/auth", auth::routes())
        .nest("/api/users", users::routes())
        .nest("/api/config", config::routes())
        .nest("/api/work-items", work_items::routes())
        .nest("/api/gitlab", gitlab::routes())
        .nest("/api/claude", claude::routes())
        .nest("/api/reports", reports::routes())
        .nest("/api/sync", sync::routes())
        .nest("/api/tempo", tempo::routes())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(db)
}
