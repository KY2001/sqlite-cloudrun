use openapi::{apis::default::SyncReplicaResponse, models::ErrorResponse};

use crate::litestream;

// POST /sync
pub async fn sync_replica(path: &str) -> SyncReplicaResponse {
    match litestream::sync(path).await {
        Ok(()) => SyncReplicaResponse::Status204_AllChangesAreReplicated,
        Err(e) => SyncReplicaResponse::Status500_LitestreamSyncFailed(ErrorResponse::new(
            500,
            format!("litestream sync: {e}"),
        )),
    }
}
