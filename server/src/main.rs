mod cleanup;
mod config;
mod crypto;
mod db;
mod error;
mod handlers;
mod keygen;
mod storage;

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::response::Html;
use axum::routing::{delete, get, post};
use axum::Router;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

const INDEX_HTML: &str = include_str!("../../web/index.html");

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<config::Config>,
    pub db: Arc<Mutex<Connection>>,
}

pub fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = config::Config::load()?;
    tracing::info!("api_key stored at {}", config.args.data_dir.join("api_key").display());

    let conn = db::open(&config.db_path)?;
    let state = AppState {
        config: Arc::new(config),
        db: Arc::new(Mutex::new(conn)),
    };

    cleanup::spawn(state.clone());

    let max_body = state.config.args.max_file_bytes as usize;

    let protected = Router::new()
        .route("/put", post(handlers::put::put_new))
        .route("/put/:share_key", post(handlers::put::put_with_key))
        .route("/get/:share_key", get(handlers::get::get_share))
        .route("/list", get(handlers::list::list_shares))
        .route("/delete/:share_key", delete(handlers::delete::delete_share))
        .layer(DefaultBodyLimit::max(max_body))
        .route_layer(middleware::from_fn_with_state(state.clone(), handlers::auth::require_api_key));

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/", get(|| async { Html(INDEX_HTML) }))
        .merge(protected)
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(&state.config.args.listen_addr).await?;
    tracing::info!("listening on {}", state.config.args.listen_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
