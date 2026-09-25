use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use qenlo_browser::server::app_router;
use qenlo_browser::state::{BrowserSession, SharedState};
use qenlo_browser::tui::functions::FUNCTION_CATALOG;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

#[tokio::test]
async fn test_rest_api_full_flow() {
    let temp_dir = tempfile::tempdir().unwrap();
    let col_path = temp_dir.path().join("api_test.qenlo");
    let col_path_str = col_path.to_str().unwrap().to_string();

    let session = BrowserSession::new();
    let shared_state: SharedState = Arc::new(RwLock::new(session));
    let app = app_router(shared_state);

    // 1. GET /
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. POST /api/create
    let create_body = json!({
        "path": col_path_str,
        "dimension": 4
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/create")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&create_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. POST /api/mutate (Add 2 records)
    let mutate_body = json!({
        "mutations": [
            {
                "type": "add",
                "record": {
                    "id": 101,
                    "user_id": 5,
                    "timestamp": 1700000000,
                    "vector": [1.0, 0.0, 0.0, 0.0]
                }
            },
            {
                "type": "add",
                "record": {
                    "id": 102,
                    "user_id": 5,
                    "timestamp": 1700001000,
                    "vector": [0.0, 1.0, 0.0, 0.0]
                }
            }
        ]
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/mutate")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&mutate_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. GET /api/records
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/records?offset=0&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["total"], 2);
    assert_eq!(body["records"].as_array().unwrap().len(), 2);

    // 5. POST /api/search
    let search_body = json!({
        "query": [1.0, 0.0, 0.0, 0.0],
        "k": 5
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/search")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&search_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["results"][0]["id"], 101);
    assert!(body["results"][0]["distance"].as_f64().unwrap() < 1e-4);

    // 6. POST /api/flush
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/flush")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 7. GET /api/storage
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/storage")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(!body["files"].as_array().unwrap().is_empty());

    // 8. GET /api/diagnostics
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/diagnostics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_functions_matches_catalog() {
    let session = BrowserSession::new();
    let shared_state: SharedState = Arc::new(RwLock::new(session));
    let app = app_router(shared_state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/functions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    let functions = body.as_array().unwrap();

    assert_eq!(functions.len(), FUNCTION_CATALOG.len());
    assert_eq!(functions[0]["name"], "new");

    for (entry, doc) in functions.iter().zip(FUNCTION_CATALOG.iter()) {
        assert_eq!(entry["name"], doc.name);
        assert!(!entry["name"].as_str().unwrap().is_empty());
        assert!(!entry["group"].as_str().unwrap().is_empty());
        assert!(!entry["signature"].as_str().unwrap().is_empty());
        assert!(!entry["summary"].as_str().unwrap().is_empty());
        assert!(!entry["example"].as_str().unwrap().is_empty());
    }
}
