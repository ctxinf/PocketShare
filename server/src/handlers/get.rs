use crate::{db, error::AppError, handlers::serve::serve_share, now_ts, AppState};
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::Response;

pub async fn get_share(
    State(state): State<AppState>,
    Path(share_key): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let now = now_ts();
    let row = {
        let conn = state.db.lock().await;
        db::get_share(&conn, &share_key, now)?
    };
    let row = row.ok_or(AppError::NotFound)?;
    let range_header = headers.get(axum::http::header::RANGE).and_then(|v| v.to_str().ok());
    serve_share(&state, &row, range_header).await
}
