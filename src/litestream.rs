use std::{
    future::Future,
    time::{Duration, Instant},
};

use tokio::process::{Child, Command};

const POLL_INTERVAL: Duration = Duration::from_millis(100);
const RESTORE_TIMEOUT: Duration = Duration::from_secs(30);
const CATCH_UP_TIMEOUT: Duration = Duration::from_secs(10);

// Restores the database from GCS while the old revision still serves, then calls `stop` and
// applies only the changes made in the meantime. Falls back to a full restore.
pub async fn restore(path: &str, stop: impl Future<Output = ()>) {
    if std::env::var("LITESTREAM_SOCKET").is_err() {
        return stop.await; // Not running under Litestream (e.g. `make run`).
    }
    // Follow mode restores, then keeps applying new changes and writes the last applied TXID to `<path>-txid`.
    let mut follower = Command::new("litestream")
        .args(["restore", "-f", "-follow-interval", "100ms"])
        .args(["-if-replica-exists", path])
        .kill_on_drop(true)
        .spawn()
        .expect("start litestream restore -f");
    catch_up(path, &mut follower, RESTORE_TIMEOUT).await;

    stop.await;

    // The old revision has pushed its last changes, so only those are left to apply.
    let caught_up = catch_up(path, &mut follower, CATCH_UP_TIMEOUT).await;
    let _ = follower.kill().await;
    if !caught_up {
        println!("follow restore did not catch up; restoring from scratch");
        let _ = std::fs::remove_file(path);
    }

    let status = Command::new("litestream")
        .args(["restore", "-if-db-not-exists", "-if-replica-exists", path])
        .status()
        .await
        .expect("run litestream restore");
    assert!(status.success(), "litestream restore: {status}");
}

// Waits until `restore -f` has applied the latest TXID in GCS. Returns whether it has.
async fn catch_up(path: &str, follower: &mut Child, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        let target = replica_txid(path).await;
        if target.is_some() && followed_txid(path) >= target {
            return true;
        }
        if !follower.try_wait().is_ok_and(|status| status.is_none()) || Instant::now() > deadline {
            return false;
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

// The last TXID `restore -f` has applied.
fn followed_txid(path: &str) -> Option<u64> {
    let txid = std::fs::read_to_string(format!("{path}-txid")).ok()?;
    u64::from_str_radix(txid.trim(), 16).ok()
}

// The latest TXID in GCS. Litestream never deletes the newest level-0 file.
async fn replica_txid(path: &str) -> Option<u64> {
    let output = Command::new("litestream")
        .args(["ltx", "-level", "0", "-json", path])
        .output()
        .await
        .ok()?;
    let files: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).ok()?;
    files
        .iter()
        .filter_map(|file| u64::from_str_radix(file["max_txid"].as_str()?, 16).ok())
        .max()
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
