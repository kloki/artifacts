use std::sync::Arc;

use askama::Template;
use axum::{
    Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event, Sse},
    },
};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use uuid::Uuid;

use crate::{
    AppState,
    error::AppError,
    events::EventKind,
    models::{
        ArtifactResponse, ContentType, ListParams, ListResponse, Meta, MetaParams, ViewParams,
    },
};

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;

type ApiResult<T> = Result<T, AppError>;

fn respond(state: &AppState, meta: Meta) -> ArtifactResponse {
    let view_uri = state.config.view_uri(meta.id, meta.title.as_deref());
    ArtifactResponse { meta, view_uri }
}

/// Public view IDs may be a bare UUID (`/a/{uuid}`) or a UUID followed by a
/// cosmetic slug (`/a/{uuid}-{slug}`). Only the leading UUID is used for
/// routing, so the human-readable part can change without breaking links.
fn parse_view_id(raw: &str) -> Result<uuid::Uuid, AppError> {
    if let Ok(id) = uuid::Uuid::parse_str(raw) {
        return Ok(id);
    }
    if raw.len() > 36 && raw.as_bytes().get(36) == Some(&b'-') {
        if let Ok(id) = uuid::Uuid::parse_str(&raw[..36]) {
            return Ok(id);
        }
    }
    Err(AppError::NotFound)
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

fn content_type_from_headers(headers: &HeaderMap) -> ContentType {
    match headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    {
        Some(ct) if ct.starts_with("text/markdown") || ct.starts_with("text/x-markdown") => {
            ContentType::Markdown
        }
        _ => ContentType::Html,
    }
}

pub async fn create_artifact(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MetaParams>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Response> {
    require_html(&body)?;
    let content_type = content_type_from_headers(&headers);
    let meta = state
        .storage
        .create(&body, content_type, params.title, params.description)
        .await?;
    tracing::info!(id = %meta.id, bytes = meta.size_bytes, "artifact created");
    Ok((StatusCode::CREATED, Json(respond(&state, meta))).into_response())
}

pub async fn update_artifact(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<MetaParams>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<ArtifactResponse>> {
    let id = parse_id(&id)?;
    require_html(&body)?;
    let content_type = content_type_from_headers(&headers);
    let meta = state
        .storage
        .update(id, &body, content_type, params.title, params.description)
        .await?;
    state.events.publish(id, EventKind::Update(meta.version));
    tracing::info!(id = %meta.id, version = meta.version, "artifact updated");
    Ok(Json(respond(&state, meta)))
}

pub async fn patch_artifact(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<MetaParams>,
) -> ApiResult<Json<ArtifactResponse>> {
    let id = parse_id(&id)?;
    if params.title.is_none() && params.description.is_none() {
        return Err(AppError::BadRequest(
            "at least one of title or description must be supplied".to_string(),
        ));
    }
    let title = params
        .title
        .map(|t| if t.is_empty() { None } else { Some(t) });
    let description = params
        .description
        .map(|d| if d.is_empty() { None } else { Some(d) });
    let meta = state
        .storage
        .update_metadata(id, title, description)
        .await?;
    tracing::info!(id = %meta.id, "artifact metadata updated");
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
    state.events.publish(id, EventKind::Deleted);
    state.events.remove(id);
    tracing::info!(%id, "artifact deleted");
    Ok(StatusCode::NO_CONTENT)
}

/// Server-sent events for an artifact. Subscribers receive `update` when a new
/// version is published and `deleted` when the artifact is removed. The stream
/// sends periodic keepalive comments so proxies do not close idle connections.
pub async fn artifact_events(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let id = parse_id(&id)?;
    // Confirm the artifact exists; unknown IDs should be 404, not an open stream.
    state.storage.read_meta(id).await?;

    let rx = state.events.subscribe(id);
    let stream = BroadcastStream::new(rx).map(|result| {
        let event = match result {
            Ok(EventKind::Update(v)) => Event::default()
                .event("update")
                .data(format!(r#"{{"version":{v}}}"#)),
            Ok(EventKind::Deleted) => Event::default().event("deleted").data("{}"),
            Err(_) => Event::default().event("error").data("event stream lagged"),
        };
        Ok::<_, std::convert::Infallible>(event)
    });

    Ok(Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
        .into_response())
}

/// Public view route. Serves artifact HTML verbatim — no CSP, since artifacts
/// are intentionally arbitrary self-contained pages with inline scripts.
pub async fn view_artifact(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<ViewParams>,
) -> ApiResult<Response> {
    let id = parse_view_id(&id)?;
    let (content, meta) = state.storage.read_content(id, params.version).await?;

    let body = match meta.content_type {
        ContentType::Html => content,
        ContentType::Markdown => {
            wrap_markdown(meta.title.as_deref(), meta.description.as_deref(), &content)
        }
    };

    Ok((
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::REFERRER_POLICY, "no-referrer"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        body,
    )
        .into_response())
}

/// Askama template for the management dashboard.
#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate;

/// Askama template for the Markdown artifact view.
#[derive(Template)]
#[template(path = "markdown.html")]
struct MarkdownTemplate<'a> {
    title: &'a str,
    description: Option<&'a str>,
    raw: &'a str,
}

/// Renders a raw Markdown artifact as a self-contained Bauhaus-styled page
/// that shows the original text and offers a copy-to-clipboard button.
fn wrap_markdown(title: Option<&str>, description: Option<&str>, raw: &[u8]) -> Vec<u8> {
    let template = MarkdownTemplate {
        title: title.unwrap_or("untitled"),
        description,
        raw: std::str::from_utf8(raw).unwrap_or_default(),
    };
    template.render().unwrap_or_default().into_bytes()
}

/// The management dashboard. Rendered from an Askama template and embedded at
/// compile time so the binary stays self-contained.
pub async fn dashboard() -> Response {
    let page = DashboardTemplate.render().unwrap_or_default();
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        page,
    )
        .into_response()
}
