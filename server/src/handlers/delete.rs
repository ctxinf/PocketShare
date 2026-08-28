use crate::{db, error::AppError, storage, AppState};
use axum::extract::{Path, State};
use axum::http::StatusCode;

pub async fn delete_share(
    State(state): State<AppState>,
    Path(share_key): Path<String>,
) -> Result<StatusCode, AppError> {
    let deleted = {
        let conn = state.db.lock().await;
        let deleted = db::delete_share(&conn, &share_key)?;
        db::delete_tokens_for_share(&conn, &share_key)?;
        deleted
    };
    if !deleted {
        return Err(AppError::NotFound);
    }
    storage::delete_asset(&state.config.assets_dir, &share_key).await?;
    Ok(StatusCode::NO_CONTENT)
}
