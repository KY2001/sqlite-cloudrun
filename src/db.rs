use std::time::{Duration, Instant};

use deadpool_sqlite::{Config, Hook, HookError, Pool, PoolError, Runtime};
use openapi::{models::Result as SqlResult, types::Object};
use rusqlite::{fallible_iterator::FallibleIterator, types::ValueRef, Batch, Connection};
use serde_json::Value;

const POOL_SIZE: usize = 30;
const QUERY_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn open(path: &str) -> Pool {
    let pool = Config::new(path)
        .builder(Runtime::Tokio1)
        .unwrap()
        .max_size(POOL_SIZE)
        .post_create(Hook::sync_fn(|conn, _| {
            // Litestream requires WAL mode.
            conn.lock()
                .unwrap()
                .execute_batch(
                    "PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000; PRAGMA synchronous = NORMAL;",
                )
                .map_err(HookError::Backend)
        }))
        .build()
        .unwrap();
    let mut conns = Vec::with_capacity(POOL_SIZE);
    for _ in 0..POOL_SIZE {
        conns.push(pool.get().await.expect("open database"));
    }
    drop(conns);
    pool
}

pub async fn execute(pool: &Pool, sql: String) -> Result<rusqlite::Result<SqlResult>, PoolError> {
    let conn = pool.get().await?;
    let task = tokio::spawn(async move {
        conn.interact(move |conn| {
            let deadline = Instant::now() + QUERY_TIMEOUT;
            conn.progress_handler(1000, Some(move || Instant::now() > deadline))?;
            let result = run(conn, &sql);
            conn.progress_handler(0, None::<fn() -> bool>)?;
            // Don't hand a connection with an open transaction to the next request.
            if !conn.is_autocommit() {
                let _ = conn.execute_batch("ROLLBACK");
            }
            result
        })
        .await
    });
    Ok(task.await.unwrap().unwrap())
}

fn run(conn: &Connection, sql: &str) -> rusqlite::Result<SqlResult> {
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

    Ok(SqlResult::new(
        columns,
        rows,
        (conn.total_changes() - before) as i64,
    ))
}

fn to_json(value: ValueRef) -> Object {
    let value = match value {
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
    };
    serde_json::from_value(value).unwrap()
}
