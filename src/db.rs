 nmbmfgn vcuse std::time::{Duration, Instant};

use deadpool_sqlite::{Config, Hook, HookError, Pool, PoolError, Runtime};
use openapi::{
    models::{Result as SqlResult, Statement},
    types::Object,
};
use rusqlite::{
    hooks::{AuthAction, AuthContext, Authorization},
    params_from_iter,
    types::{Value as SqlValue, ValueRef},
    Connection,
};
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
                    "PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;",
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

pub async fn execute(
    pool: &Pool,
    statements: Vec<Statement>,
) -> Result<rusqlite::Result<Vec<SqlResult>>, PoolError> {
    let conn = pool.get().await?;
    let task = tokio::spawn(async move {
        conn.interact(move |conn| {
            let deadline = Instant::now() + QUERY_TIMEOUT;
            conn.progress_handler(1000, Some(move || Instant::now() > deadline))?;
            let result = run_all(conn, &statements);
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

// Two or more statements run in one transaction. IMMEDIATE takes the write lock up front, so a
// later write can't fail with SQLITE_BUSY halfway through.
fn run_all(conn: &Connection, statements: &[Statement]) -> rusqlite::Result<Vec<SqlResult>> {
    if let [statement] = statements {
        return Ok(vec![run(conn, statement)?]);
    }
    conn.execute_batch("BEGIN IMMEDIATE")?;
    // Reject BEGIN/COMMIT/ROLLBACK at prepare time so a statement can't end the transaction early.
    conn.authorizer(Some(|ctx: AuthContext<'_>| match ctx.action {
        AuthAction::Transaction { .. } => Authorization::Deny,
        _ => Authorization::Allow,
    }))?;
    let results = statements
        .iter()
        .map(|statement| run(conn, statement))
        .collect::<rusqlite::Result<Vec<_>>>();
    conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>)?;
    let results = results?;
    conn.execute_batch("COMMIT")?;
    Ok(results)
}

fn run(conn: &Connection, statement: &Statement) -> rusqlite::Result<SqlResult> {
    let before = conn.total_changes();
    // prepare() rejects SQL holding more than one statement.
    let mut stmt = conn.prepare(&statement.sql)?;
    let columns: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
    let types = stmt
        .columns()
        .iter()
        .map(|c| c.decl_type().unwrap_or_default().to_string())
        .collect();
    let params = statement
        .params
        .iter()
        .flatten()
        .map(to_sql)
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut rows = Vec::new();
    let mut cursor = stmt.query(params_from_iter(params))?;
    while let Some(row) = cursor.next()? {
        let values = (0..columns.len())
            .map(|i| row.get_ref(i).map(to_json))
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.push(values);
    }
    drop(cursor);

    Ok(SqlResult::new(
        columns,
        types,
        rows,
        if conn.total_changes() == before {
            0
        } else {
            conn.changes() as i64
        },
    ))
}

fn to_sql(param: &Object) -> rusqlite::Result<SqlValue> {
    Ok(match serde_json::to_value(param).unwrap() {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(b.into()),
        Value::Number(n) => match n.as_i64() {
            Some(i) => SqlValue::Integer(i),
            None => SqlValue::Real(n.as_f64().unwrap_or(f64::NAN)),
        },
        Value::String(s) => SqlValue::Text(s),
        value => {
            return Err(rusqlite::Error::ToSqlConversionFailure(
                format!("unsupported parameter: {value}").into(),
            ))
        }
    })
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
