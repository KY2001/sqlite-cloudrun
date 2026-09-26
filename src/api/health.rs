use openapi::{
    apis::default::HealthResponse,
    models::{Health200Response, Health503Response},
};

use crate::litestream;

// GET /health
pub async fn health() -> HealthResponse {
    if litestream::healthy().await {
        HealthResponse::Status200_LitestreamDaemonIsResponsive(Health200Response {
            litestream: Some("healthy".into()),
        })
    } else {
        HealthResponse::Status503_LitestreamDaemonIsUnavailable(Health503Response {
            litestream: Some("unavailable".into()),
        })
    }
}
