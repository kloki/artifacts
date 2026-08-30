use std::sync::Arc;

use axum::{
    Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{
    AppState,
    error::AppError,
    models::{ArtifactResponse, ListParams, ListResponse, Meta, MetaParams, ViewParams},
};

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;

type ApiResult<T> = Result<T, AppError>;

fn respond(state: &AppState, meta: Meta) -> ArtifactResponse {
    let view_uri = state.config.view_uri(meta.id);
    ArtifactResponse { meta, view_uri }
}

/// Path IDs are parsed by hand so the API can answer 400 while the public view
/// route answers 404 for the same malformed input.
fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::BadRequest(format!("`{raw}` is not a valid UUID")))
}

fn require_html(body: &Bytes) -> Result<(), AppError> {
    if body.is_empty() {
        return Err(AppError::BadRequest(
            "request body is empty; send the artifact HTML as the raw body".to_string(),
        ));
    }
    Ok(())
}

pub async fn create_artifact(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MetaParams>,
    body: Bytes,
) -> ApiResult<Response> {
    require_html(&body)?;
    let meta = state
        .storage
        .create(&body, params.title, params.description)
        .await?;
    tracing::info!(id = %meta.id, bytes = meta.size_bytes, "artifact created");
    Ok((StatusCode::CREATED, Json(respond(&state, meta))).into_response())
}

pub async fn update_artifact(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<MetaParams>,
    body: Bytes,
) -> ApiResult<Json<ArtifactResponse>> {
    let id = parse_id(&id)?;
    require_html(&body)?;
    let meta = state
        .storage
        .update(id, &body, params.title, params.description)
        .await?;
    tracing::info!(id = %meta.id, version = meta.version, "artifact updated");
    Ok(Json(respond(&state, meta)))
}

pub async fn get_artifact(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<ArtifactResponse>> {
    let meta = state.storage.read_meta(parse_id(&id)?).await?;
    Ok(Json(respond(&state, meta)))
}

pub async fn list_artifacts(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<ListResponse>> {
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = params.offset.unwrap_or(0);

    let (metas, total) = state.storage.list(limit, offset).await?;
    let items = metas.into_iter().map(|m| respond(&state, m)).collect();

    Ok(Json(ListResponse {
        items,
        total,
        limit,
        offset,
    }))
}

pub async fn delete_artifact(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let id = parse_id(&id)?;
    state.storage.delete(id).await?;
    tracing::info!(%id, "artifact deleted");
    Ok(StatusCode::NO_CONTENT)
}

/// Public view route. Serves artifact HTML verbatim — no CSP, since artifacts
/// are intentionally arbitrary self-contained pages with inline scripts.
pub async fn view_artifact(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<ViewParams>,
) -> ApiResult<Response> {
    let id = Uuid::parse_str(&id).map_err(|_| AppError::NotFound)?;
    let html = state.storage.read_html(id, params.version).await?;

    Ok((
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::REFERRER_POLICY, "no-referrer"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        html,
    )
        .into_response())
}
