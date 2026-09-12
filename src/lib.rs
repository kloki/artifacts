pub mod config;
pub mod error;
pub mod events;
pub mod handlers;
pub mod models;
pub mod storage;

use std::sync::Arc;

use axum::{Router, extract::DefaultBodyLimit, routing::get};
use config::Config;
use events::EventBus;
use storage::Storage;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub struct AppState {
    pub config: Config,
    pub storage: Storage,
    pub events: EventBus,
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
        .route("/artifacts/{id}/events", get(handlers::artifact_events))
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
