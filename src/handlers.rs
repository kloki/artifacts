use std::sync::Arc;

use axum::{
    Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{
    AppState,
    error::AppError,
    models::{
        ArtifactResponse, ContentType, ListParams, ListResponse, Meta, MetaParams, ViewParams,
    },
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

/// Escapes a string for safe inclusion in HTML text or attributes.
fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// Renders a raw Markdown artifact as a self-contained Bauhaus-styled page
/// that shows the original text and offers a copy-to-clipboard button.
fn wrap_markdown(title: Option<&str>, description: Option<&str>, raw: &[u8]) -> Vec<u8> {
    let title = title.unwrap_or("untitled");
    let desc = description.unwrap_or_default();
    let text = String::from_utf8_lossy(raw);
    let escaped = html_escape(&text);
    let escaped_title = html_escape(title);
    let escaped_desc = html_escape(desc);

    let desc_html = if desc.is_empty() {
        String::new()
    } else {
        format!(r#"<p class="lede">{}</p>"#, escaped_desc)
    };

    let page = format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{escaped_title}</title>
    <style>
      :root {{
        --paper: #f4f1e8;
        --paper-raised: #fbf9f3;
        --ink: #1a1a1a;
        --ink-muted: #57544a;
        --ink-faint: #8a877b;
        --red: #c2332b;
        --blue: #1f4fb8;
        --rule: #1a1a1a;
        --rule-soft: #d8d4c6;
        --font-sans:
          "Futura", "Avenir Next", "Century Gothic", "Segoe UI", system-ui,
          Roboto, "Helvetica Neue", Arial, sans-serif;
      }}
      @media (prefers-color-scheme: dark) {{
        :root {{
          --paper: #161511;
          --paper-raised: #1e1d17;
          --ink: #f2efe6;
          --ink-muted: #b5b1a4;
          --ink-faint: #7d7a6e;
          --red: #e0524a;
          --blue: #7d9de8;
          --rule: #f2efe6;
          --rule-soft: #3a3931;
        }}
      }}
      * {{ box-sizing: border-box; }}
      body {{
        margin: 0;
        background: var(--paper);
        color: var(--ink);
        font-family: var(--font-sans);
        line-height: 1.6;
        -webkit-font-smoothing: antialiased;
        padding: 2rem 1rem;
      }}
      .wrap {{
        max-width: 68rem;
        margin: 0 auto;
      }}
      header {{ margin-bottom: 1.5rem; }}
      .eyebrow {{
        margin: 0 0 0.5rem;
        font-size: 0.8125rem;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: var(--red);
      }}
      h1 {{
        margin: 0;
        font-size: clamp(2rem, 6vw, 2.75rem);
        font-weight: 700;
        line-height: 1.05;
        text-transform: uppercase;
      }}
      .lede {{
        margin-top: 1rem;
        max-width: 35rem;
        color: var(--ink-muted);
        font-size: 1.0625rem;
      }}
      .toolbar {{
        display: flex;
        gap: 0.75rem;
        align-items: center;
        margin: 1.5rem 0;
      }}
      button {{
        font-family: inherit;
        font-size: 0.875rem;
        cursor: pointer;
        min-height: 2.75rem;
        padding: 0.5rem 1.25rem;
        border: 2px solid var(--rule);
        background: transparent;
        color: var(--ink);
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        transition: border-color 150ms ease, background 150ms ease, color 150ms ease;
      }}
      button:hover {{
        background: var(--ink);
        border-color: var(--ink);
        color: var(--paper);
      }}
      pre {{
        margin: 0;
        padding: 1.5rem;
        border: 2px solid var(--rule);
        background: var(--paper-raised);
        color: var(--ink);
        font-family: ui-monospace, Menlo, Consolas, monospace;
        font-size: 0.9375rem;
        line-height: 1.6;
        overflow-x: auto;
        white-space: pre-wrap;
      }}
      @media (max-width: 40rem) {{
        body {{ padding: 1.5rem 1rem; }}
      }}
    </style>
  </head>
  <body>
    <div class="wrap">
      <header>
        <p class="eyebrow">Markdown artifact</p>
        <h1>{escaped_title}</h1>
        {desc_html}
      </header>
      <div class="toolbar">
        <button id="copy" type="button">copy raw text</button>
      </div>
      <pre id="raw">{escaped}</pre>
    </div>
    <script>
      "use strict";
      (function () {{
        const button = document.getElementById("copy");
        const raw = document.getElementById("raw");
        async function copy() {{
          const text = raw.textContent;
          let ok = false;
          if (navigator.clipboard && window.isSecureContext) {{
            try {{
              await navigator.clipboard.writeText(text);
              ok = true;
            }} catch {{}}
          }}
          if (!ok) {{
            const scratch = document.createElement("textarea");
            scratch.value = text;
            scratch.setAttribute("readonly", "");
            scratch.style.position = "fixed";
            scratch.style.opacity = "0";
            document.body.append(scratch);
            scratch.select();
            try {{
              ok = document.execCommand("copy");
            }} catch {{}}
            scratch.remove();
          }}
          button.textContent = ok ? "copied" : "copy failed";
          setTimeout(() => button.textContent = "copy raw text", 2000);
        }}
        button.addEventListener("click", copy);
      }})();
    </script>
  </body>
</html>"#
    );
    page.into_bytes()
}

/// The management dashboard. Embedded at compile time so the binary stays
/// self-contained — there is no static directory to ship or mount alongside it.
pub async fn dashboard() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        include_str!("dashboard.html"),
    )
        .into_response()
}
