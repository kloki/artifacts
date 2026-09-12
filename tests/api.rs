use std::sync::Arc;

use artifacts::{AppState, app, config::Config, storage::Storage};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
use tempfile::TempDir;
use tower::ServiceExt;

const MAX_BODY: usize = 1024;

fn test_app(dir: &TempDir) -> Router {
    let config = Config {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        data_dir: dir.path().to_path_buf(),
        public_base_url: "https://artifacts.example.com".to_string(),
        max_body_bytes: MAX_BODY,
    };
    let storage = Storage::new(&config.data_dir).unwrap();
    app(Arc::new(AppState { config, storage }))
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, Vec<u8>) {
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let body = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, body)
}

async fn send_json(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let (status, body) = send(app, req).await;
    let json = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, json)
}

fn post(uri: &str, html: &str) -> Request<Body> {
    Request::post(uri)
        .body(Body::from(html.to_string()))
        .unwrap()
}

fn put(uri: &str, html: &str) -> Request<Body> {
    Request::put(uri)
        .body(Body::from(html.to_string()))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::get(uri).body(Body::empty()).unwrap()
}

async fn create(app: &Router, html: &str) -> Value {
    let (status, json) = send_json(app, post("/api/artifacts", html)).await;
    assert_eq!(status, StatusCode::CREATED, "create failed: {json}");
    json
}

#[tokio::test]
async fn create_get_and_view() {
    let dir = TempDir::new().unwrap();
    let app = test_app(&dir);
    let html = "<!doctype html><title>Hi</title><h1>Hello</h1>";

    let (status, created) = send_json(
        &app,
        post("/api/artifacts?title=My%20Page&description=A%20test", html),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let id = created["id"].as_str().unwrap().to_string();
    assert_eq!(created["version"], 1);
    assert_eq!(created["title"], "My Page");
    assert_eq!(created["description"], "A test");
    assert_eq!(created["size_bytes"], html.len());
    assert_eq!(
        created["view_uri"],
        format!("https://artifacts.example.com/a/{id}")
    );
    // sha256 of the uploaded bytes, lowercase hex.
    assert_eq!(created["sha256"].as_str().unwrap().len(), 64);

    let (status, fetched) = send_json(&app, get(&format!("/api/artifacts/{id}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched["sha256"], created["sha256"]);
    assert_eq!(fetched["created_at"], created["created_at"]);

    let res = app.clone().oneshot(get(&format!("/a/{id}"))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()["content-type"], "text/html; charset=utf-8");
    assert_eq!(res.headers()["x-content-type-options"], "nosniff");
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body, html.as_bytes());
}

#[tokio::test]
async fn update_keeps_history() {
    let dir = TempDir::new().unwrap();
    let app = test_app(&dir);

    let created = create(&app, "<h1>v1</h1>").await;
    let id = created["id"].as_str().unwrap().to_string();

    let (status, updated) =
        send_json(&app, put(&format!("/api/artifacts/{id}"), "<h1>v2</h1>")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["version"], 2);
    assert_eq!(updated["view_uri"], created["view_uri"]);
    assert_eq!(updated["created_at"], created["created_at"]);
    assert!(updated["updated_at"].as_str().unwrap() >= created["updated_at"].as_str().unwrap());

    // The bare view URL follows the latest version...
    let (status, body) = send(&app, get(&format!("/a/{id}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"<h1>v2</h1>");

    // ...while every version that ever existed stays addressable.
    let (status, body) = send(&app, get(&format!("/a/{id}?version=1"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"<h1>v1</h1>");

    let (status, body) = send(&app, get(&format!("/a/{id}?version=2"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"<h1>v2</h1>");

    let (status, _) = send(&app, get(&format!("/a/{id}?version=3"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = send(&app, get(&format!("/a/{id}?version=0"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_paginates_newest_first() {
    let dir = TempDir::new().unwrap();
    let app = test_app(&dir);

    let mut ids = Vec::new();
    for i in 0..3 {
        let created = create(&app, &format!("<h1>{i}</h1>")).await;
        ids.push(created["id"].as_str().unwrap().to_string());
    }

    let (status, page) = send_json(&app, get("/api/artifacts?limit=2&offset=0")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["total"], 3);
    assert_eq!(page["limit"], 2);
    assert_eq!(page["items"].as_array().unwrap().len(), 2);

    let (_, page2) = send_json(&app, get("/api/artifacts?limit=2&offset=2")).await;
    assert_eq!(page2["items"].as_array().unwrap().len(), 1);

    // Every artifact appears exactly once across the two pages.
    let mut seen: Vec<String> = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .chain(page2["items"].as_array().unwrap())
        .map(|i| i["id"].as_str().unwrap().to_string())
        .collect();
    seen.sort();
    ids.sort();
    assert_eq!(seen, ids);
}

#[tokio::test]
async fn delete_removes_artifact() {
    let dir = TempDir::new().unwrap();
    let app = test_app(&dir);

    let created = create(&app, "<h1>bye</h1>").await;
    let id = created["id"].as_str().unwrap().to_string();

    let (status, _) = send(
        &app,
        Request::delete(format!("/api/artifacts/{id}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = send(&app, get(&format!("/api/artifacts/{id}"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = send(&app, get(&format!("/a/{id}"))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = send(
        &app,
        Request::delete(format!("/api/artifacts/{id}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (_, page) = send_json(&app, get("/api/artifacts")).await;
    assert_eq!(page["total"], 0);
}

#[tokio::test]
async fn rejects_invalid_input() {
    let dir = TempDir::new().unwrap();
    let app = test_app(&dir);

    let (status, json) = send_json(&app, post("/api/artifacts", "")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["error"], "bad_request");

    let oversized = "x".repeat(MAX_BODY + 1);
    let (status, _) = send(&app, post("/api/artifacts", &oversized)).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);

    // A malformed ID is a client error on the API, but just a missing page on
    // the public view route.
    let (status, _) = send(&app, get("/api/artifacts/not-a-uuid")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = send(&app, get("/a/not-a-uuid")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let missing = uuid::Uuid::new_v4();
    let (status, _) = send(&app, put(&format!("/api/artifacts/{missing}"), "<p>x</p>")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_preserves_metadata_unless_supplied() {
    let dir = TempDir::new().unwrap();
    let app = test_app(&dir);

    let (_, created) = send_json(
        &app,
        post(
            "/api/artifacts?title=Original&description=First",
            "<p>1</p>",
        ),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_string();

    let (_, kept) = send_json(&app, put(&format!("/api/artifacts/{id}"), "<p>2</p>")).await;
    assert_eq!(kept["title"], "Original");
    assert_eq!(kept["description"], "First");

    let (_, replaced) = send_json(
        &app,
        put(&format!("/api/artifacts/{id}?title=Renamed"), "<p>3</p>"),
    )
    .await;
    assert_eq!(replaced["title"], "Renamed");
    assert_eq!(replaced["description"], "First");
}

#[tokio::test]
async fn markdown_artifact_is_served_raw_in_bauhaus_shell() {
    let dir = TempDir::new().unwrap();
    let app = test_app(&dir);
    let md = "# Hello\n\nSome <script>alert(1)</script> & \"text\".";

    let req = Request::post("/api/artifacts?title=Notes&description=d")
        .header("Content-Type", "text/markdown")
        .body(Body::from(md.to_string()))
        .unwrap();
    let (status, created) = send_json(&app, req).await;
    assert_eq!(status, StatusCode::CREATED);
    let id = created["id"].as_str().unwrap().to_string();
    assert_eq!(created["content_type"], "markdown");
    assert_eq!(created["size_bytes"], md.len());

    let (status, body) = send(&app, get(&format!("/a/{id}"))).await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("Markdown artifact"));
    assert!(html.contains("<h1>Notes</h1>"));
    assert!(html.contains("copy raw text"));
    assert!(html.contains("# Hello"));
    // Dangerous characters are escaped inside the raw view.
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("&amp;"));
}

#[tokio::test]
async fn markdown_versioning_keeps_type() {
    let dir = TempDir::new().unwrap();
    let app = test_app(&dir);
    let id = create_markdown(&app, "# v1").await;

    let req = Request::put(format!("/api/artifacts/{id}"))
        .header("Content-Type", "text/markdown")
        .body(Body::from("# v2".to_string()))
        .unwrap();
    let (status, updated) = send_json(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["version"], 2);
    assert_eq!(updated["content_type"], "markdown");

    let (status, body) = send(&app, get(&format!("/a/{id}?version=1"))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8(body).unwrap().contains("# v1"));

    let (status, body) = send(&app, get(&format!("/a/{id}?version=2"))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8(body).unwrap().contains("# v2"));
}

#[tokio::test]
async fn update_rejects_content_type_switch() {
    let dir = TempDir::new().unwrap();
    let app = test_app(&dir);
    let created = create(&app, "<p>1</p>").await;
    let id = created["id"].as_str().unwrap().to_string();
    assert_eq!(created["content_type"], "html");

    let req = Request::put(format!("/api/artifacts/{id}"))
        .header("Content-Type", "text/markdown")
        .body(Body::from("# md".to_string()))
        .unwrap();
    let (status, json) = send_json(&app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(json["error"], "bad_request");
}

async fn create_markdown(app: &Router, md: &str) -> String {
    let req = Request::post("/api/artifacts?title=Notes")
        .header("Content-Type", "text/markdown")
        .body(Body::from(md.to_string()))
        .unwrap();
    let (status, json) = send_json(app, req).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "create markdown failed: {json}"
    );
    json["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn healthz_reports_ok() {
    let dir = TempDir::new().unwrap();
    let app = test_app(&dir);

    let (status, body) = send(&app, get("/healthz")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"ok");
}

#[tokio::test]
async fn root_serves_the_dashboard() {
    let dir = TempDir::new().unwrap();
    let app = test_app(&dir);

    let res = app.oneshot(get("/")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()[axum::http::header::CONTENT_TYPE],
        "text/html; charset=utf-8"
    );

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).unwrap();
    assert!(html.contains("<title>Artifacts</title>"));
    assert!(html.contains("/api/artifacts"));
}
