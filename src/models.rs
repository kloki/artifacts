use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Whether an artifact stores raw HTML or raw Markdown.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    #[default]
    Html,
    Markdown,
}

/// Persisted as `meta.json` next to the artifact's `index.html`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meta {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub version: u32,
    pub size_bytes: u64,
    pub sha256: String,
    #[serde(default)]
    pub content_type: ContentType,
}

/// `view_uri` is derived from PUBLIC_BASE_URL at response time rather than
/// stored, so the base URL can change without rewriting every artifact.
#[derive(Debug, Serialize)]
pub struct ArtifactResponse {
    #[serde(flatten)]
    pub meta: Meta,
    pub view_uri: String,
}

#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub items: Vec<ArtifactResponse>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Default, Deserialize)]
pub struct MetaParams {
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ListParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ViewParams {
    pub version: Option<u32>,
}
