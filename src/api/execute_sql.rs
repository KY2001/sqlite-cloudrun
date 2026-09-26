use deadpool_sqlite::Pool;
use openapi::{
    apis::default::ExecuteSqlResponse,
    models::{ErrorResponse, Statement},
};
use rusqlite::Error;

use crate::db;

// POST /sql
pub async fn execute_sql(pool: &Pool, statements: Vec<Statement>) -> ExecuteSqlResponse {
    match db::execute(pool, statements).await {
        Ok(Ok(result)) => ExecuteSqlResponse::Status200_StatementsExecuted(result),
        Ok(Err(e)) => ExecuteSqlResponse::Status400_SQLError(ErrorResponse {
            code: sqlite_code(&e),
            ..ErrorResponse::new(400, e.to_string())
        }),
        Err(e) => ExecuteSqlResponse::Status503_NoDatabaseConnectionAvailable(ErrorResponse::new(
            503,
            e.to_string(),
        )),
    }
}

// SQLite's extended result code, e.g. 2067 for SQLITE_CONSTRAINT_UNIQUE.
fn sqlite_code(e: &Error) -> Option<i32> {
    match e {
        Error::SqliteFailure(error, _) | Error::SqlInputError { error, .. } => {
            Some(error.extended_code)
        }
        _ => None,
    }
}
