mod execute_sql;
mod health;
mod sync_replica;

use async_trait::async_trait;
use axum::http::Method;
use axum_extra::extract::{CookieJar, Host};
use deadpool_sqlite::Pool;
use openapi::apis::{
    default::{Default, ExecuteSqlResponse, HealthResponse, SyncReplicaResponse},
    ErrorHandler,
};

pub struct Server {
    pub path: String,
    pub pool: Pool,
}

impl AsRef<Server> for Server {
    fn as_ref(&self) -> &Server {
        self
    }
}

impl ErrorHandler for Server {}

#[async_trait]
impl Default for Server {
    async fn execute_sql(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
        body: &String,
    ) -> Result<ExecuteSqlResponse, ()> {
        Ok(execute_sql::execute_sql(&self.pool, body.clone()).await)
    }

    async fn health(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
    ) -> Result<HealthResponse, ()> {
        Ok(health::health().await)
    }

    async fn sync_replica(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
    ) -> Result<SyncReplicaResponse, ()> {
        Ok(sync_replica::sync_replica(&self.path).await)
    }
}
