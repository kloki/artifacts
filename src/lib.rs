pub mod config;
pub mod error;
pub mod handlers;
pub mod models;
pub mod storage;

use std::sync::Arc;

use axum::{extract::DefaultBodyLimit, routing::get, Router};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use config::Config;
use storage::Storage;

pub struct AppState {
    pub config: Config,
    pub storage: Storage,
}

pub fn app(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route(
            "/artifacts",
            get(handlers::list_artifacts).post(handlers::create_artifact),
        )
        .route(
            "/artifacts/{id}",
            get(handlers::get_artifact)
                .put(handlers::update_artifact)
                .delete(handlers::delete_artifact),
        )
        .layer(DefaultBodyLimit::max(state.config.max_body_bytes))
        .layer(CorsLayer::permissive());

    Router::new()
        .nest("/api", api)
        .route("/", get(handlers::dashboard))
        .route("/a/{id}", get(handlers::view_artifact))
        .route("/healthz", get(|| async { "ok" }))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
