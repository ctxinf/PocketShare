use crate::{AppState, db, error::AppError, keygen, now_ts, storage};
use axum::extract::{FromRequest, Path as AxPath, Query, Request, State};
use axum::Json;
use axum::body::Bytes;
use axum::http::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct PutQuery {
    pub ttl_secs: Option<i64>,
    pub public: Option<bool>,
}

#[derive(Serialize)]
pub struct PutResponse {
    pub share_key: String,
}

pub async fn put_new(
    state: State<AppState>,
    query: Query<PutQuery>,
    req: Request,
) -> Result<Json<PutResponse>, AppError> {
    handle_put(state, None, query, req).await
}

pub async fn put_with_key(
    state: State<AppState>,
    AxPath(share_key): AxPath<String>,
    query: Query<PutQuery>,
    req: Request,
) -> Result<Json<PutResponse>, AppError> {
    handle_put(state, Some(share_key), query, req).await
}

async fn handle_put(
    State(state): State<AppState>,
    given_key: Option<String>,
    Query(query): Query<PutQuery>,
    req: Request,
) -> Result<Json<PutResponse>, AppError> {
    let content_type = req
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let max_bytes = state.config.args.max_file_bytes;

    let (kind, raw_file_name, data): (&str, Option<String>, Vec<u8>) =
        if content_type.starts_with("multipart/form-data") {
            let mut multipart = axum::extract::Multipart::from_request(req, &state)
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
            let mut file_name = None;
            let mut bytes: Option<Vec<u8>> = None;
            while let Some(field) = multipart
                .next_field()
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?
            {
                if field.name() == Some("file") {
                    file_name = field.file_name().map(|s| s.to_string());
                    let b = field
                        .bytes()
                        .await
                        .map_err(|e| AppError::BadRequest(e.to_string()))?;
                    if b.len() as u64 > max_bytes {
                        return Err(AppError::PayloadTooLarge);
                    }
                    bytes = Some(b.to_vec());
                }
            }
            let data = bytes.ok_or_else(|| AppError::BadRequest("missing file field".into()))?;
            ("file", file_name, data)
        } else {
            let body = Bytes::from_request(req, &state)
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
            if body.len() as u64 > max_bytes {
                return Err(AppError::PayloadTooLarge);
            }
            ("text", None, body.to_vec())
        };

    let now = now_ts();
    let ttl = query.ttl_secs.unwrap_or(state.config.args.default_ttl_secs);
    let expires_at = now + ttl.max(0);
    let is_public = query.public.unwrap_or(true);

    let conn = state.db.lock().await;
    let share_key = match given_key {
        Some(k) => k,
        None => keygen::generate_share_key(&conn, now)?,
    };
    db::upsert_share(
        &conn,
        &share_key,
        kind,
        raw_file_name.as_deref(),
        is_public,
        now,
        expires_at,
    )?;
    drop(conn);

    storage::write_asset(&state.config.assets_dir, &share_key, data).await?;

    Ok(Json(PutResponse { share_key }))
}
