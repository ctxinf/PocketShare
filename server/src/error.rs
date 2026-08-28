use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug)]
pub enum AppError {
    Unauthorized,
    NotFound,
    PayloadTooLarge,
    CapacityFull,
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            AppError::PayloadTooLarge => {
                (StatusCode::PAYLOAD_TOO_LARGE, "payload too large".to_string())
            }
            AppError::CapacityFull => {
                (StatusCode::SERVICE_UNAVAILABLE, "share key capacity full".to_string())
            }
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        };
        (status, msg).into_response()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}
