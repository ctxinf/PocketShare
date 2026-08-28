use crate::{AppState, db, error::AppError, now_ts};
use axum::extract::State;
use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct ShareListItem {
    pub share_key: String,
    pub kind: String,
    pub raw_file_name: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
}

pub async fn list_shares(State(state): State<AppState>) -> Result<Json<Vec<ShareListItem>>, AppError> {
    let now = now_ts();
    let conn = state.db.lock().await;
    let rows = db::list_public(&conn, now)?;
    let items = rows
        .into_iter()
        .map(|r| ShareListItem {
            share_key: r.share_key,
            kind: r.kind,
            raw_file_name: r.raw_file_name,
            created_at: r.created_at,
            expires_at: r.expires_at,
        })
        .collect();
    Ok(Json(items))
}
