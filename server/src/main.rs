mod cleanup;
mod config;
mod crypto;
mod db;
mod error;
mod handlers;
mod keygen;
mod storage;

use axum::body::Body;
use axum::extract::DefaultBodyLimit;
use axum::http::header;
use axum::middleware;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::Router;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

const INDEX_HTML: &str = include_str!("../../web/index.html");
/// PWA assets. The manifest and the service worker both carry URLs, so they
/// go through the same prefix substitution as the page itself.
const MANIFEST_JSON: &str = include_str!("../../web/manifest.webmanifest");
const SERVICE_WORKER_JS: &str = include_str!("../../web/sw.js");
const ICON_SVG: &str = include_str!("../../web/icon.svg");
const ICON_192_PNG: &[u8] = include_bytes!("../../web/icon-192.png");
const ICON_512_PNG: &[u8] = include_bytes!("../../web/icon-512.png");
const ICON_MASKABLE_512_PNG: &[u8] = include_bytes!("../../web/icon-maskable-512.png");
const APPLE_TOUCH_ICON_PNG: &[u8] = include_bytes!("../../web/apple-touch-icon.png");
/// Replaced at startup with the configured `--path-prefix` before serving.
const PATH_PREFIX_PLACEHOLDER: &str = "__MYSHARE_PATH_PREFIX__";

/// Icons are immutable for a given build; the shell documents are not, so they
/// must be revalidated or a redeploy can be pinned behind a stale HTTP cache.
const ICON_CACHE_CONTROL: &str = "public, max-age=604800";
const NO_CACHE: &str = "no-cache";

fn static_response(
    content_type: &'static str,
    cache_control: &'static str,
    body: impl Into<Body>,
) -> Response {
    (
        [(header::CONTENT_TYPE, content_type), (header::CACHE_CONTROL, cache_control)],
        body.into(),
    )
        .into_response()
}

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

/// The PWA asset routes: manifest, service worker and icons. Unauthenticated
/// on purpose -- the browser fetches the manifest and registers the worker
/// before the user has entered an api_key, and none of these expose data.
fn pwa_routes(manifest_json: &'static str, service_worker_js: &'static str) -> Router<AppState> {
    Router::new()
        .route(
            "/manifest.webmanifest",
            get(move || async move {
                static_response("application/manifest+json", NO_CACHE, manifest_json)
            }),
        )
        // Must be served from the app root: a worker's default scope is its own
        // directory, so a nested path would not control the page.
        .route(
            "/sw.js",
            get(move || async move {
                static_response("text/javascript; charset=utf-8", NO_CACHE, service_worker_js)
            }),
        )
        .route(
            "/icon.svg",
            get(|| async { static_response("image/svg+xml", ICON_CACHE_CONTROL, ICON_SVG) }),
        )
        .route(
            "/icon-192.png",
            get(|| async { static_response("image/png", ICON_CACHE_CONTROL, ICON_192_PNG) }),
        )
        .route(
            "/icon-512.png",
            get(|| async { static_response("image/png", ICON_CACHE_CONTROL, ICON_512_PNG) }),
        )
        .route(
            "/icon-maskable-512.png",
            get(|| async {
                static_response("image/png", ICON_CACHE_CONTROL, ICON_MASKABLE_512_PNG)
            }),
        )
        .route(
            "/apple-touch-icon.png",
            get(|| async { static_response("image/png", ICON_CACHE_CONTROL, APPLE_TOUCH_ICON_PNG) }),
        )
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
    let inject = |s: &str| -> &'static str {
        Box::leak(
            s.replace(PATH_PREFIX_PLACEHOLDER, &state.config.settings.path_prefix)
                .into_boxed_str(),
        )
    };
    let index_html: &'static str = inject(INDEX_HTML);
    let manifest_json: &'static str = inject(MANIFEST_JSON);
    let service_worker_js: &'static str = inject(SERVICE_WORKER_JS);

    // The shell must be revalidated on every load, otherwise a redeployed UI
    // stays invisible behind the browser's HTTP cache until it expires.
    let index_response = move || async move {
        (
            [(header::CACHE_CONTROL, NO_CACHE)],
            Html(index_html),
        )
            .into_response()
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/", get(index_response))
        .route("/dl/:share_key", get(handlers::download::download))
        .merge(pwa_routes(manifest_json, service_worker_js))
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
                    ([(header::CACHE_CONTROL, NO_CACHE)], Html(index_html)).into_response()
                } else {
                    (axum::http::StatusCode::NOT_FOUND, "not found").into_response()
                }
            }
        })
    };

    let listener = tokio::net::TcpListener::bind(&state.config.settings.listen_addr).await?;
    tracing::info!(
        "listening on http://{}{}",
        state.config.settings.listen_addr,
        state.config.settings.path_prefix
    );
    axum::serve(listener, app).await?;

    Ok(())
}
