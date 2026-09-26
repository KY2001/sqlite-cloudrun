use openapi::{apis::default::SyncReplicaResponse, models::ErrorResponse};

use crate::litestream;

// POST /sync
pub async fn sync_replica(path: &str, open: bool) -> SyncReplicaResponse {
    if !open {
        return SyncReplicaResponse::Status503_TheDatabaseWasHandedOffToANewRevision(
            ErrorResponse::new(503, "the database was handed off to a new revision".into()),
        );
    }
    match litestream::sync(path).await {
        Ok(()) => SyncReplicaResponse::Status204_AllChangesAreReplicated,
        Err(e) => SyncReplicaResponse::Status500_LitestreamSyncFailed(ErrorResponse::new(
            500,
            format!("litestream sync: {e}"),
        )),
    }
}
