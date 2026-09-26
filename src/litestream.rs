use std::time::Duration;

// Queries the Litestream control socket with a three-second deadline.
pub async fn healthy() -> bool {
    let Ok(socket) = std::env::var("LITESTREAM_SOCKET") else {
        return false;
    };

    let output = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::process::Command::new("litestream")
            .args(["info", "-socket", &socket, "-timeout", "2"])
            .kill_on_drop(true)
            .output(),
    )
    .await;

    matches!(output, Ok(Ok(output)) if output.status.success())
}

// With request-based billing the instance has no CPU between requests, so Litestream's
// background replication may stall. Call this periodically (e.g. from Cloud Scheduler) to
// push pending changes to GCS.
pub async fn sync(path: &str) -> Result<(), String> {
    let Ok(socket) = std::env::var("LITESTREAM_SOCKET") else {
        return Ok(()); // Not running under Litestream (e.g. `make run`).
    };
    let output = tokio::process::Command::new("litestream")
        .args(["sync", "-wait", "-socket", &socket, path])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}
