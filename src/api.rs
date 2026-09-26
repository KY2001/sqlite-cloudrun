mod execute_sql;
mod health;
mod stop;
mod sync_replica;

use async_trait::async_trait;
use axum::http::Method;
use axum_extra::extract::{CookieJar, Host};
use deadpool_sqlite::Pool;
use openapi::apis::{
    default::{Default, ExecuteSqlResponse, HealthResponse, StopResponse, SyncReplicaResponse},
    ErrorHandler,
};
use openapi::models::{Statement, StopQueryParams};
use tokio::{process::Child, sync::RwLock};

use crate::litestream;

pub struct Server {
    pub path: String,
    pub db: RwLock<Option<Database>>,
}

pub struct Database {
    pub pool: Pool,
    pub litestream: Option<Child>,
}

impl Server {
    // Pushes pending changes to GCS, then closes SQLite and stops Litestream.
    pub async fn stop(&self) -> Result<(), String> {
        let mut db = self.db.write().await;
        let Some(Database { pool, litestream }) = db.take() else {
            return Ok(());
        };
        let result = litestream::sync(&self.path).await;
        pool.close();
        if let Some(mut litestream) = litestream {
            let _ = litestream.kill().await;
        }
        println!("database handed off");
        result
    }
}

impl AsRef<Server> for Server {
    fn as_ref(&self) -> &Server {
        self
    }
}

impl ErrorHandler for Server {}

#[async_trait]
impl Default for Server {
    async fn stop(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
        query_params: &StopQueryParams,
    ) -> Result<StopResponse, ()> {
        Ok(stop::stop(self, &query_params.revision).await)
    }

    async fn execute_sql(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
        body: &Vec<Statement>,
    ) -> Result<ExecuteSqlResponse, ()> {
        let db = self.db.read().await;
        Ok(execute_sql::execute_sql(db.as_ref().map(|db| &db.pool), body.clone()).await)
    }

    async fn health(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
    ) -> Result<HealthResponse, ()> {
        let db = self.db.read().await;
        Ok(health::health(db.as_ref().map(|db| &db.pool)).await)
    }

    async fn sync_replica(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
    ) -> Result<SyncReplicaResponse, ()> {
        let db = self.db.read().await;
        Ok(sync_replica::sync_replica(&self.path, db.is_some()).await)
    }
}
