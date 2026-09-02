use std::net::SocketAddr;
use axum::{
    extract::{Path as AxPath, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use serde_json::json;
use crate::state::SharedState;
use qenlo::{Filter, Mutation, NewRecord, TimestampRange};

const INDEX_HTML: &str = include_str!("web/index.html");

#[derive(Debug, Deserialize)]
pub struct OpenRequest {
    pub path: String,
    pub dimension: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    pub path: String,
    pub dimension: usize,
}

#[derive(Debug, Deserialize)]
pub struct RecordsQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    pub user_id: Option<u64>,
    pub min_ts: Option<i64>,
    pub max_ts: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchFilterPayload {
    pub user_id: Option<u64>,
    pub lower_ts: Option<i64>,
    pub upper_ts: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: Vec<f32>,
    pub filter: Option<SearchFilterPayload>,
    pub k: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum MutationPayload {
    #[serde(rename = "add")]
    Add { record: NewRecordPayload },
    #[serde(rename = "delete")]
    Delete { id: u64 },
}

#[derive(Debug, Deserialize)]
pub struct NewRecordPayload {
    pub id: u64,
    pub user_id: u64,
    pub timestamp: i64,
    pub vector: Vec<f32>,
}

#[derive(Debug, Deserialize)]
pub struct MutateRequest {
    pub mutations: Vec<MutationPayload>,
}

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub path: String,
}

pub fn app_router(state: SharedState) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/api/status", get(get_status))
        .route("/api/open", post(open_collection))
        .route("/api/create", post(create_collection))
        .route("/api/records", get(get_records))
        .route("/api/records/{id}", get(get_record_by_id))
        .route("/api/search", post(run_search))
        .route("/api/mutate", post(mutate_records))
        .route("/api/flush", post(flush_collection))
        .route("/api/export", post(export_collection))
        .route("/api/storage", get(get_storage))
        .route("/api/diagnostics", get(get_diagnostics))
        .with_state(state)
}

pub async fn run_server(state: SharedState, host: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let app = app_router(state);
    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    println!("⬡ QenloDB Web Browser running on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn get_status(State(state): State<SharedState>) -> impl IntoResponse {
    let session = state.read().await;
    Json(session.get_status())
}

async fn open_collection(
    State(state): State<SharedState>,
    Json(payload): Json<OpenRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let mut session = state.write().await;
    match session.open_collection(&payload.path, payload.dimension).await {
        Ok(stats) => Ok(Json(json!({
            "status": "ok",
            "dimension": stats.dimension,
            "rows": stats.rows,
            "live_rows": stats.live_rows,
            "generation": stats.generation,
        }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({ "error": e })))),
    }
}

async fn create_collection(
    State(state): State<SharedState>,
    Json(payload): Json<CreateRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let mut session = state.write().await;
    match session.create_collection(&payload.path, payload.dimension).await {
        Ok(stats) => Ok(Json(json!({
            "status": "ok",
            "dimension": stats.dimension,
            "rows": stats.rows,
            "generation": stats.generation,
        }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({ "error": e })))),
    }
}

async fn get_records(
    State(state): State<SharedState>,
    Query(query): Query<RecordsQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let session = state.read().await;
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(50).min(500);

    let filter = if query.user_id.is_some() || query.min_ts.is_some() || query.max_ts.is_some() {
        Some(Filter::new(
            query.user_id,
            TimestampRange::new(query.min_ts, query.max_ts),
        ))
    } else {
        None
    };

    match session.scan_records(offset, limit, filter.as_ref()) {
        Ok(paginated) => Ok(Json(paginated)),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({ "error": e })))),
    }
}

async fn get_record_by_id(
    State(state): State<SharedState>,
    AxPath(id): AxPath<u64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let session = state.read().await;
    match session.get_record(id) {
        Ok(Some(record)) => Ok(Json(record)),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(json!({ "error": format!("Record #{id} not found") })))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({ "error": e })))),
    }
}

async fn run_search(
    State(state): State<SharedState>,
    Json(payload): Json<SearchRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let session = state.read().await;
    let k = payload.k.unwrap_or(10).clamp(1, qenlo::MAX_K);

    let filter = if let Some(f) = payload.filter {
        Filter::new(f.user_id, TimestampRange::new(f.lower_ts, f.upper_ts))
    } else {
        Filter::ALL
    };

    match session.search(&payload.query, &filter, k).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({ "error": e })))),
    }
}

async fn mutate_records(
    State(state): State<SharedState>,
    Json(payload): Json<MutateRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let session = state.read().await;
    let mut mutations = Vec::with_capacity(payload.mutations.len());

    for m in payload.mutations {
        match m {
            MutationPayload::Add { record } => {
                mutations.push(Mutation::Add(NewRecord {
                    id: record.id,
                    user_id: record.user_id,
                    timestamp: record.timestamp,
                    vector: record.vector,
                }));
            }
            MutationPayload::Delete { id } => {
                mutations.push(Mutation::Delete(id));
            }
        }
    }

    match session.commit_mutations(&mutations) {
        Ok(report) => Ok(Json(json!({
            "status": "ok",
            "generation": report.generation,
            "durable_generation": report.durable_generation,
            "mutations": report.mutations,
        }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({ "error": e })))),
    }
}

async fn flush_collection(
    State(state): State<SharedState>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let session = state.read().await;
    match session.flush() {
        Ok(()) => Ok(Json(json!({ "status": "ok", "message": "Collection flushed and synced" }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({ "error": e })))),
    }
}

async fn export_collection(
    State(state): State<SharedState>,
    Json(payload): Json<ExportRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let session = state.read().await;
    match session.export_qn(&payload.path) {
        Ok(()) => Ok(Json(json!({ "status": "ok", "path": payload.path }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({ "error": e })))),
    }
}

async fn get_storage(State(state): State<SharedState>) -> impl IntoResponse {
    let session = state.read().await;
    Json(session.get_storage_details())
}

async fn get_diagnostics(State(state): State<SharedState>) -> impl IntoResponse {
    let session = state.read().await;
    Json(session.get_diagnostics())
}
