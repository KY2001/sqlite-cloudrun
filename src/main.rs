mod api;
mod db;
mod handoff;
mod litestream;
mod logger;

use std::sync::Arc;

use axum::middleware;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let path = std::env::var("DB_PATH").unwrap_or_else(|_| "/data/app.db".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());

    // Take the database over from the old revision during the restore, so no changes are lost and only one instance replicates to GCS.
    litestream::restore(&path, handoff::stop_serving_revision()).await;
    let server = Arc::new(api::Server {
        db: RwLock::new(Some(api::Database {
            pool: db::open(&path).await,
            litestream: litestream::replicate(),
        })),
        path: path.clone(),
    });
    let app = openapi::server::new(server.clone()).layer(middleware::from_fn(logger::log_request));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    println!("listening on :{port}, database {path}");
    axum::serve(listener, app)
        .with_graceful_shutdown(sigterm())
        .await
        .unwrap();

    // Cloud Run sends SIGTERM before stopping the instance; push pending changes to GCS.
    if let Err(e) = server.stop().await {
        eprintln!("litestream sync on shutdown: {e}");
    }
}

async fn sigterm() {
    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .unwrap()
        .recv()
        .await;
}
