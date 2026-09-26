use std::time::Duration;

use deadpool_sqlite::Pool;
use openapi::{apis::default::HealthResponse, models::Statement};

use crate::{db, litestream};

// GET /health
pub async fn health(pool: &Pool) -> HealthResponse {
    let (litestream, database) = tokio::join!(litestream::healthy(), database_healthy(pool));
    if litestream && database {
        HealthResponse::Status204_LitestreamAndTheDatabaseAreResponsive
    } else {
        HealthResponse::Status503_LitestreamOrTheDatabaseIsUnavailable
    }
}

async fn database_healthy(pool: &Pool) -> bool {
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        db::execute(pool, vec![Statement::new("SELECT 1".into())]),
    )
    .await;
    matches!(result, Ok(Ok(Ok(_))))
}
