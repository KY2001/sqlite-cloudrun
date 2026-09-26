use std::time::Duration;

use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

pub async fn health() -> (StatusCode, Json<Value>) {
    let Ok(socket) = std::env::var("LITESTREAM_SOCKET") else {
        return unavailable();
    };

    let output = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::process::Command::new("litestream")
            .args(["info", "-socket", &socket, "-timeout", "2"])
            .kill_on_drop(true)
            .output(),
    )
    .await;

    match output {
        Ok(Ok(output)) if output.status.success() => {
            (StatusCode::OK, Json(json!({ "litestream": "healthy" })))
        }
        _ => unavailable(),
    }
}

fn unavailable() -> (StatusCode, Json<Value>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "litestream": "unavailable" })),
    )
}
