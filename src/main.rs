mod api;
mod db;
mod litestream;
mod logger;

use std::sync::Arc;

use axum::middleware;

#[tokio::main]
async fn main() {
    let path = std::env::var("DB_PATH").unwrap_or_else(|_| "/data/app.db".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());

    let server = api::Server {
        pool: db::open(&path).await,
        path: path.clone(),
    };
    let app =
        openapi::server::new(Arc::new(server)).layer(middleware::from_fn(logger::log_request));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    println!("listening on :{port}, database {path}");
    axum::serve(listener, app)
        .with_graceful_shutdown(sigterm())
        .await
        .unwrap();

    // Cloud Run sends SIGTERM before stopping the instance; push pending changes to GCS.
    if let Err(e) = litestream::sync(&path).await {
        eprintln!("litestream sync on shutdown: {e}");
    }
}

async fn sigterm() {
    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .unwrap()
        .recv()
        .await;
}
