use crate::{db, error::AppError, now_ts, AppState};
use axum::extract::{Path, State};
use axum::Json;
use rand::Rng;
use serde::Serialize;

const LINK_TTL_SECS: i64 = 3600;

#[derive(Serialize)]
pub struct LinkResponse {
    pub token: String,
    pub expires_at: i64,
}

fn random_token() -> String {
    let mut rng = rand::thread_rng();
    let choices = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    (0..48).map(|_| choices[rng.gen_range(0..choices.len())] as char).collect()
}

pub async fn create_link(
    State(state): State<AppState>,
    Path(share_key): Path<String>,
) -> Result<Json<LinkResponse>, AppError> {
    let now = now_ts();
    let conn = state.db.lock().await;
    let row = db::get_share(&conn, &share_key, now)?;
    if row.is_none() {
        return Err(AppError::NotFound);
    }

    let token = random_token();
    let expires_at = now + LINK_TTL_SECS;
    db::create_download_token(&conn, &token, &share_key, expires_at)?;
    // Minting a link is itself an access intent: make sure the share lives
    // at least as long as the link we just handed out.
    db::extend_share_expiry(&conn, &share_key, expires_at)?;

    Ok(Json(LinkResponse { token, expires_at }))
}
