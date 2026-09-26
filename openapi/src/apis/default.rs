use async_trait::async_trait;
use axum::extract::*;
use axum_extra::extract::{CookieJar, Host};
use bytes::Bytes;
use http::Method;
use serde::{Deserialize, Serialize};

use crate::{models, types::*};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteSqlResponse {
    /// Statements executed
    Status200_StatementsExecuted
    (Vec<models::Result>)
    ,
    /// SQL error
    Status400_SQLError
    (models::ErrorResponse)
    ,
    /// No database connection available
    Status503_NoDatabaseConnectionAvailable
    (models::ErrorResponse)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum HealthResponse {
    /// Litestream and the database are responsive
    Status204_LitestreamAndTheDatabaseAreResponsive
    ,
    /// Litestream or the database is unavailable
    Status503_LitestreamOrTheDatabaseIsUnavailable
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum SyncReplicaResponse {
    /// All changes are replicated
    Status204_AllChangesAreReplicated
    ,
    /// Litestream sync failed
    Status500_LitestreamSyncFailed
    (models::ErrorResponse)
}




/// Default
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Default<E: std::fmt::Debug + Send + Sync + 'static = ()>: super::ErrorHandler<E> {
    /// ExecuteSql - POST /sql
    async fn execute_sql(
    &self,
    
    method: &Method,
    host: &Host,
    cookies: &CookieJar,
            body: &Vec<models::Statement>,
    ) -> Result<ExecuteSqlResponse, E>;

    /// Check Litestream and database responsiveness.
    ///
    /// Health - GET /health
    async fn health(
    &self,
    
    method: &Method,
    host: &Host,
    cookies: &CookieJar,
    ) -> Result<HealthResponse, E>;

    /// Replicate to GCS.
    ///
    /// SyncReplica - POST /sync
    async fn sync_replica(
    &self,
    
    method: &Method,
    host: &Host,
    cookies: &CookieJar,
    ) -> Result<SyncReplicaResponse, E>;
}
