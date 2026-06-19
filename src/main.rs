use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod models;
mod handlers;

use db::Database;
use handlers::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "device_metrics=info,tower_http=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_path = std::env::var("DB_PATH")
        .unwrap_or_else(|_| "metrics.db".to_string());

    tracing::info!("正在打开数据库: {}", db_path);

    let db = Database::new(&db_path)?;
    let state = Arc::new(db);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/metrics", post(submit_metric))
        .route("/api/metrics", get(get_all_metrics))
        .route("/api/metrics/device/:device_id", get(get_device_metrics))
        .route("/api/metrics/device/:device_id/latest", get(get_latest_metric))
        .with_state(state);

    let addr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("服务启动，监听地址: {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
