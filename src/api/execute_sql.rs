use deadpool_sqlite::Pool;
use openapi::{apis::default::ExecuteSqlResponse, models::ErrorResponse};

use crate::db;

// POST /sql
pub async fn execute_sql(pool: &Pool, body: String) -> ExecuteSqlResponse {
    match db::execute(pool, body).await {
        Ok(Ok(result)) => ExecuteSqlResponse::Status200_StatementExecuted(result),
        Ok(Err(e)) => {
            ExecuteSqlResponse::Status400_SQLError(ErrorResponse::new(400, e.to_string()))
        }
        Err(e) => ExecuteSqlResponse::Status503_NoDatabaseConnectionAvailable(ErrorResponse::new(
            503,
            e.to_string(),
        )),
    }
}
