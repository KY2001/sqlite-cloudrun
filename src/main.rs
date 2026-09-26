mod health;
mod logger;

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use rusqlite::{fallible_iterator::FallibleIterator, types::ValueRef, Batch, Connection};
use serde_json::{json, Value};

type Db = Arc<String>;

#[tokio::main]
async fn main() {
    let path = std::env::var("DB_PATH").unwrap_or_else(|_| "/data/app.db".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());

    let conn = Connection::open(&path).expect("open database");
    // Litestream requires WAL mode.
    conn.execute_batch(
        "PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000; PRAGMA synchronous = NORMAL;",
    )
    .expect("configure database");

    let app = Router::new()
        .route("/health", get(health::health))
        .route("/sql", post(sql))
        .route("/sync", post(sync))
        .layer(middleware::from_fn(logger::log_request))
        .with_state(Arc::new(path.clone()));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    println!("listening on :{port}, database {path}");
    axum::serve(listener, app).await.unwrap();
}

async fn sql(
    State(path): State<Db>,
    body: String,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    tokio::task::spawn_blocking(move || {
        let conn = Connection::open(path.as_str())?;
        conn.execute_batch("PRAGMA busy_timeout = 5000; PRAGMA synchronous = NORMAL;")?;
        execute(&conn, &body)
    })
    .await
    .unwrap()
    .map(Json)
    .map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
    })
}

// With request-based billing the instance has no CPU between requests, so Litestream's
// background replication may stall. Call this periodically (e.g. from Cloud Scheduler) to
// push pending changes to GCS.
async fn sync(State(path): State<Db>) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let Ok(socket) = std::env::var("LITESTREAM_SOCKET") else {
        return Ok(StatusCode::NO_CONTENT); // Not running under Litestream (e.g. `make run`).
    };
    let output = tokio::process::Command::new("litestream")
        .args(["sync", "-wait", "-socket", &socket, path.as_str()])
        .output()
        .await;
    match output {
        Ok(output) if output.status.success() => Ok(StatusCode::NO_CONTENT),
        Ok(output) => Err(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        Err(e) => Err(e.to_string()),
    }
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("litestream sync: {e}") })),
        )
    })
}

fn execute(conn: &Connection, sql: &str) -> rusqlite::Result<Value> {
    let before = conn.total_changes();
    let mut columns = Vec::new();
    let mut rows = Vec::new();

    // Run each statement in turn; the response holds the last statement's rows.
    let mut batch = Batch::new(conn, sql);
    while let Some(mut stmt) = batch.next()? {
        columns = stmt.column_names().into_iter().map(String::from).collect();
        rows.clear();
        let mut cursor = stmt.query([])?;
        while let Some(row) = cursor.next()? {
            let values = (0..columns.len())
                .map(|i| row.get_ref(i).map(to_json))
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows.push(values);
        }
    }

    Ok(json!({ "columns": columns, "rows": rows, "changes": conn.total_changes() - before }))
}

fn to_json(value: ValueRef) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => i.into(),
        ValueRef::Real(f) => f.into(),
        ValueRef::Text(t) => String::from_utf8_lossy(t).into(),
        // Blobs are returned as lowercase hex strings.
        ValueRef::Blob(b) => b
            .iter()
            .map(|x| format!("{x:02x}"))
            .collect::<String>()
            .into(),
    }
}
