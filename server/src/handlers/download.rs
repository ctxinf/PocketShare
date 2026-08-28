use crate::{db, error::AppError, handlers::serve::serve_share, now_ts, AppState};
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use rand::Rng;
use serde::Deserialize;

const RENEW_PROBABILITY: f64 = 0.10;
const RENEW_TTL_SECS: i64 = 3600;

#[derive(Deserialize)]
pub struct DlQuery {
    pub token: String,
}

pub async fn download(
    State(state): State<AppState>,
    Path(share_key): Path<String>,
    Query(query): Query<DlQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let now = now_ts();

    let row = {
        let conn = state.db.lock().await;
        let token_row = db::get_download_token(&conn, &query.token, now)?.ok_or(AppError::Unauthorized)?;
        if token_row.share_key != share_key {
            return Err(AppError::Unauthorized);
        }
        let row = db::get_share(&conn, &share_key, now)?.ok_or(AppError::NotFound)?;

        // Randomly-sampled trigger: only a fraction of accesses pay the cost
        // of a write, instead of renewing on every single (possibly Range-
        // split, many-requests-per-download) access.
        if rand::thread_rng().gen_bool(RENEW_PROBABILITY) {
            let new_expiry = now + RENEW_TTL_SECS;
            db::renew_download_token(&conn, &query.token, new_expiry)?;
            db::extend_share_expiry(&conn, &share_key, new_expiry)?;
        }

        row
    };

    let range_header = headers.get(axum::http::header::RANGE).and_then(|v| v.to_str().ok());
    serve_share(&state, &row, range_header).await
}
