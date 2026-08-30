use std::{env, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use qenlo_testkit::{MAX_RUN_BYTES, TestRun};
use rusqlite::{Connection, params};
use serde::Serialize;

#[derive(Clone)]
struct AppState {
    database: Arc<PathBuf>,
    api_key: Arc<str>,
}

#[derive(Debug)]
struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

#[derive(Serialize)]
struct Accepted {
    accepted: bool,
    run_id: String,
}

#[derive(Serialize)]
struct RunSummary {
    run_id: String,
    received_at: String,
    target: String,
    os: String,
    cpu_name: String,
    gpu_name: Option<String>,
    suite: String,
    passed: bool,
    cells: u32,
    failures: u32,
}

#[tokio::main]
async fn main() {
    let bind = env::var("QENLO_TELEMETRY_BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let database = PathBuf::from(
        env::var("QENLO_TELEMETRY_DB").unwrap_or_else(|_| "qenlo-telemetry.sqlite3".into()),
    );
    let api_key = env::var("QENLO_TELEMETRY_API_KEY")
        .expect("QENLO_TELEMETRY_API_KEY is required and must be at least 24 characters");
    assert!(api_key.len() >= 24, "QENLO_TELEMETRY_API_KEY is too short");
    initialize(&database).expect("initialize telemetry database");
    let state = AppState {
        database: Arc::new(database),
        api_key: Arc::from(api_key),
    };
    let app = Router::new()
        .route("/", get(dashboard))
        .route("/healthz", get(|| async { "ok\n" }))
        .route("/api/v1/runs", post(ingest).get(list_runs))
        .route("/api/v1/runs/{run_id}", get(get_run))
        .layer(DefaultBodyLimit::max(MAX_RUN_BYTES))
        .with_state(state);
    let address: SocketAddr = bind.parse().expect("valid QENLO_TELEMETRY_BIND");
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("bind telemetry listener");
    eprintln!("qenlo-telemetry listening on http://{address}");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("serve telemetry API");
}

async fn ingest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(run): Json<TestRun>,
) -> Result<(StatusCode, Json<Accepted>), ApiError> {
    authorize(&state, &headers)?;
    run.validate()
        .map_err(|reason| ApiError(StatusCode::UNPROCESSABLE_ENTITY, reason.into()))?;
    let run_id = run.run_id.clone();
    let database = state.database.clone();
    tokio::task::spawn_blocking(move || insert_run(&database, &run))
        .await
        .map_err(internal)?
        .map_err(internal)?;
    Ok((
        StatusCode::CREATED,
        Json(Accepted {
            accepted: true,
            run_id,
        }),
    ))
}

async fn list_runs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<RunSummary>>, ApiError> {
    authorize(&state, &headers)?;
    let database = state.database.clone();
    let runs = tokio::task::spawn_blocking(move || read_runs(&database))
        .await
        .map_err(internal)?
        .map_err(internal)?;
    Ok(Json(runs))
}

async fn get_run(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(run_id): Path<String>,
) -> Result<Json<TestRun>, ApiError> {
    authorize(&state, &headers)?;
    if run_id.len() > 256 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "run_id too long".into()));
    }
    let database = state.database.clone();
    let found = tokio::task::spawn_blocking(move || read_run(&database, &run_id))
        .await
        .map_err(internal)?
        .map_err(internal)?;
    found
        .map(Json)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "run not found".into()))
}

async fn dashboard() -> Html<&'static str> {
    Html(include_str!("dashboard.html"))
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = format!("Bearer {}", state.api_key);
    let supplied = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if constant_time_eq(supplied.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err(ApiError(
            StatusCode::UNAUTHORIZED,
            "invalid bearer token".into(),
        ))
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        difference |= usize::from(
            left.get(index).copied().unwrap_or_default()
                ^ right.get(index).copied().unwrap_or_default(),
        );
    }
    difference == 0
}

fn internal(error: impl std::fmt::Display) -> ApiError {
    eprintln!("telemetry error: {error}");
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
}

fn connect(path: &PathBuf) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;
    connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")?;
    Ok(connection)
}

fn initialize(path: &PathBuf) -> rusqlite::Result<()> {
    let connection = connect(path)?;
    connection.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE IF NOT EXISTS runs (
           run_id TEXT PRIMARY KEY, received_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
           install_id TEXT NOT NULL, started_ms INTEGER NOT NULL, completed_ms INTEGER NOT NULL,
           app_version TEXT NOT NULL, target TEXT NOT NULL, os TEXT NOT NULL,
           os_version TEXT NOT NULL, cpu_arch TEXT NOT NULL, cpu_name TEXT NOT NULL,
           gpu_name TEXT, gpu_api TEXT, power_source TEXT, thermal_state TEXT,
           suite TEXT NOT NULL, payload TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS runs_received ON runs(received_at DESC);",
    )
}

fn insert_run(path: &PathBuf, run: &TestRun) -> rusqlite::Result<()> {
    let payload = serde_json::to_string(run)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    let connection = connect(path)?;
    connection.execute(
        "INSERT INTO runs(run_id,install_id,started_ms,completed_ms,app_version,target,os,os_version,cpu_arch,cpu_name,gpu_name,gpu_api,power_source,thermal_state,suite,payload)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        params![run.run_id,run.install_id,run.started_at_unix_ms,run.completed_at_unix_ms,run.app_version,run.target,run.os,run.os_version,run.cpu_arch,run.cpu_name,run.gpu_name,run.gpu_api,run.power_source,run.thermal_state,run.suite,payload],
    )?;
    Ok(())
}

fn read_runs(path: &PathBuf) -> rusqlite::Result<Vec<RunSummary>> {
    let connection = connect(path)?;
    let mut statement = connection.prepare(
        "SELECT run_id,received_at,target,os,cpu_name,gpu_name,suite,payload FROM runs ORDER BY received_at DESC LIMIT 500",
    )?;
    statement
        .query_map([], |row| {
            let payload: String = row.get(7)?;
            let run: TestRun = serde_json::from_str(&payload).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    payload.len(),
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(RunSummary {
                run_id: row.get(0)?,
                received_at: row.get(1)?,
                target: row.get(2)?,
                os: row.get(3)?,
                cpu_name: row.get(4)?,
                gpu_name: row.get(5)?,
                suite: row.get(6)?,
                passed: run.failures.is_empty() && run.cells.iter().all(|cell| cell.passed),
                cells: run.cells.len() as u32,
                failures: run.failures.len() as u32,
            })
        })?
        .collect()
}

fn read_run(path: &PathBuf, run_id: &str) -> rusqlite::Result<Option<TestRun>> {
    let connection = connect(path)?;
    let mut statement = connection.prepare("SELECT payload FROM runs WHERE run_id=?1")?;
    let mut rows = statement.query([run_id])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let payload: String = row.get(0)?;
    serde_json::from_str(&payload).map(Some).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            payload.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn bearer_comparison_rejects_length_and_content_changes() {
        assert!(constant_time_eq(b"Bearer same", b"Bearer same"));
        assert!(!constant_time_eq(b"Bearer same", b"Bearer game"));
        assert!(!constant_time_eq(b"Bearer same", b"Bearer same-longer"));
    }
}
