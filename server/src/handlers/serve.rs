use crate::{db::ShareRow, error::AppError, storage, AppState};
use axum::body::Body;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Response;

/// Parses a single-range `Range: bytes=...` header value against a resource
/// of length `total_len`. Returns `None` when there's no usable range (treat
/// as a full-content request). Supports `start-end`, `start-`, and `-suffix`.
fn parse_range(range_header: &str, total_len: u64) -> Option<Result<(u64, u64), AppError>> {
    let spec = range_header.strip_prefix("bytes=")?;
    // Only single-range requests are supported; anything else falls back to full content.
    let spec = spec.split(',').next()?.trim();
    let (start_s, end_s) = spec.split_once('-')?;

    if start_s.is_empty() {
        // suffix range: last N bytes
        let suffix_len: u64 = end_s.parse().ok()?;
        if suffix_len == 0 || total_len == 0 {
            return Some(Err(AppError::RangeNotSatisfiable(total_len)));
        }
        let start = total_len.saturating_sub(suffix_len);
        return Some(Ok((start, total_len - 1)));
    }

    let start: u64 = start_s.parse().ok()?;
    let end: u64 = if end_s.is_empty() {
        total_len.saturating_sub(1)
    } else {
        match end_s.parse() {
            Ok(e) => e,
            Err(_) => return Some(Err(AppError::RangeNotSatisfiable(total_len))),
        }
    };

    if start >= total_len || end < start {
        return Some(Err(AppError::RangeNotSatisfiable(total_len)));
    }
    Some(Ok((start, end.min(total_len.saturating_sub(1)))))
}

pub async fn serve_share(
    state: &AppState,
    row: &ShareRow,
    range_header: Option<&str>,
) -> Result<Response, AppError> {
    let share_key = &row.share_key;
    let total_len = storage::asset_len(&state.config.assets_dir, share_key).await?;

    let range = range_header.and_then(|h| parse_range(h, total_len)).transpose()?;

    let content_type = if row.kind == "file" {
        "application/octet-stream"
    } else {
        "text/plain; charset=utf-8"
    };

    let mut builder = Response::builder().header(header::ACCEPT_RANGES, "bytes");
    if row.kind == "file" {
        let file_name = row.raw_file_name.clone().unwrap_or_else(|| share_key.clone());
        let disposition = format!("attachment; filename=\"{}\"", file_name.replace('"', ""));
        builder = builder.header(header::CONTENT_DISPOSITION, HeaderValue::from_str(&disposition).unwrap());
    }
    builder = builder.header(header::CONTENT_TYPE, content_type);

    match range {
        Some((start, end)) => {
            let data =
                storage::read_asset_range(&state.config.assets_dir, share_key, start, end + 1).await?;
            let resp = builder
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_LENGTH, data.len().to_string())
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {start}-{end}/{total_len}"),
                )
                .body(Body::from(data))
                .map_err(|e| AppError::Internal(e.to_string()))?;
            Ok(resp)
        }
        None => {
            let data = storage::read_asset(&state.config.assets_dir, share_key).await?;
            let resp = builder
                .status(StatusCode::OK)
                .header(header::CONTENT_LENGTH, data.len().to_string())
                .body(Body::from(data))
                .map_err(|e| AppError::Internal(e.to_string()))?;
            Ok(resp)
        }
    }
}
