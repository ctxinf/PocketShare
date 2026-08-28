use crate::{AppState, db, error::AppError, now_ts, storage};
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};

pub async fn get_share(
    State(state): State<AppState>,
    Path(share_key): Path<String>,
) -> Result<Response, AppError> {
    let now = now_ts();
    let row = {
        let conn = state.db.lock().await;
        db::get_share(&conn, &share_key, now)?
    };
    let row = row.ok_or(AppError::NotFound)?;
    let data = storage::read_asset(&state.config.assets_dir, &share_key).await?;

    if row.kind == "file" {
        let file_name = row.raw_file_name.unwrap_or_else(|| share_key.clone());
        let disposition = format!("attachment; filename=\"{}\"", file_name);
        Ok((
            [
                (header::CONTENT_TYPE, "application/octet-stream".to_string()),
                (header::CONTENT_DISPOSITION, disposition),
            ],
            data,
        )
            .into_response())
    } else {
        Ok(([(header::CONTENT_TYPE, "text/plain; charset=utf-8".to_string())], data).into_response())
    }
}
