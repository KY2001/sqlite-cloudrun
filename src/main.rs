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
    axum::serve(listener, app).await.unwrap();
}
