use crate::{AppState, error::AppError};
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

pub async fn require_api_key(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let provided = req
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if provided != state.config.api_key {
        return Err(AppError::Unauthorized);
    }
    Ok(next.run(req).await)
}
