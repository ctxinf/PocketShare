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
use axum::response::{Html, IntoResponse};
use axum::routing::{delete, get, post};
use axum::Router;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

const INDEX_HTML: &str = include_str!("../../web/index.html");
/// Replaced at startup with the configured `--path-prefix` before serving.
const PATH_PREFIX_PLACEHOLDER: &str = "__MYSHARE_PATH_PREFIX__";

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
    tracing::info!("config file: {}", config.config_path.display());
    tracing::info!("api_key stored at {}", config.settings.data_dir.join("api_key").display());

    let conn = db::open(&config.db_path)?;
    let state = AppState {
        config: Arc::new(config),
        db: Arc::new(Mutex::new(conn)),
    };

    cleanup::spawn(state.clone());

    let max_body = state.config.settings.max_file_bytes as usize;

    let protected = Router::new()
        .route("/put", post(handlers::put::put_new))
        .route("/put/:share_key", post(handlers::put::put_with_key))
        .route("/get/:share_key", get(handlers::get::get_share))
        .route("/list", get(handlers::list::list_shares))
        .route("/delete/:share_key", delete(handlers::delete::delete_share))
        .route("/link/:share_key", post(handlers::link::create_link))
        .layer(DefaultBodyLimit::max(max_body))
        .route_layer(middleware::from_fn_with_state(state.clone(), handlers::auth::require_api_key));

    // The web UI resolves its API base from this injected value, so it keeps
    // working when the server is mounted under a path prefix on a shared domain.
    let index_html: &'static str = Box::leak(
        INDEX_HTML
            .replace(PATH_PREFIX_PLACEHOLDER, &state.config.settings.path_prefix)
            .into_boxed_str(),
    );

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/", get(move || async move { Html(index_html) }))
        .route("/dl/:share_key", get(handlers::download::download))
        .merge(protected)
        .with_state(state.clone());

    let prefix = state.config.settings.path_prefix.clone();
    // `nest` maps the inner `/` route onto the bare `/myshare` and claims the
    // whole `/myshare/*` subtree, which leaves the trailing-slash form people
    // actually paste into a browser unrouted. Catch it in the nested router's
    // fallback and serve the UI there.
    let app = if prefix.is_empty() {
        app
    } else {
        let slash_path = format!("{prefix}/");
        Router::new().nest(&prefix, app).fallback(move |uri: axum::http::Uri| {
            let slash_path = slash_path.clone();
            async move {
                if uri.path() == slash_path {
                    Html(index_html).into_response()
                } else {
                    (axum::http::StatusCode::NOT_FOUND, "not found").into_response()
                }
            }
        })
    };

    let listener = tokio::net::TcpListener::bind(&state.config.settings.listen_addr).await?;
    tracing::info!(
        "listening on {}{}",
        state.config.settings.listen_addr,
        state.config.settings.path_prefix
    );
    axum::serve(listener, app).await?;

    Ok(())
}
