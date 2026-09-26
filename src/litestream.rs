use std::time::Duration;

use tokio::process::{Child, Command};

// Restores the database from GCS, if a replica exists.
pub async fn restore(path: &str) {
    if std::env::var("LITESTREAM_SOCKET").is_err() {
        return; // Not running under Litestream (e.g. `make run`).
    }
    let status = Command::new("litestream")
        .args(["restore", "-if-db-not-exists", "-if-replica-exists", path])
        .status()
        .await
        .expect("run litestream restore");
    assert!(status.success(), "litestream restore: {status}");
}

// Starts replicating the database to GCS. Kill the returned process to stop replication.
pub fn replicate() -> Option<Child> {
    std::env::var("LITESTREAM_SOCKET").ok()?;
    Some(
        Command::new("litestream")
            .arg("replicate")
            .kill_on_drop(true)
            .spawn()
            .expect("start litestream replicate"),
    )
}

// Queries the Litestream control socket with a three-second deadline.
pub async fn healthy() -> bool {
    let Ok(socket) = std::env::var("LITESTREAM_SOCKET") else {
        return false;
    };

    let output = tokio::time::timeout(
        Duration::from_secs(3),
        Command::new("litestream")
            .args(["info", "-socket", &socket, "-timeout", "2"])
            .kill_on_drop(true)
            .output(),
    )
    .await;

    matches!(output, Ok(Ok(output)) if output.status.success())
}

// An uptime check calls /sync every five minutes to push pending changes to GCS.
pub async fn sync(path: &str) -> Result<(), String> {
    let Ok(socket) = std::env::var("LITESTREAM_SOCKET") else {
        return Ok(()); // Not running under Litestream (e.g. `make run`).
    };
    let output = Command::new("litestream")
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
