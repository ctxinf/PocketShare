use crate::config::Config;
use crate::progress::Progress;
use crate::retry::{with_retry, NonRetryable};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Chunk size the upload body is sliced into. This is what bounds progress
/// granularity: the counter can only advance once per chunk, so a huge chunk
/// would make the bar jump. 64 KiB keeps updates smooth without adding
/// meaningful per-chunk overhead.
const UPLOAD_CHUNK: usize = 64 * 1024;

/// Wraps an in-memory body as a stream that reports each chunk as it is handed
/// to the socket, so the caller can draw a progress bar.
///
/// Note what this measures: bytes accepted by the local socket buffer, not
/// bytes durably stored by the server. The two diverge on a slow link, which
/// is why the caller switches to a "waiting for server" message at 100% rather
/// than claiming the upload is done.
fn counting_body(bytes: Vec<u8>, progress: Arc<Mutex<Progress>>) -> reqwest::Body {
    let total_chunks = bytes.len().div_ceil(UPLOAD_CHUNK).max(1);
    let stream = futures_util::stream::iter((0..total_chunks).map(move |i| {
        let start = i * UPLOAD_CHUNK;
        let end = ((i + 1) * UPLOAD_CHUNK).min(bytes.len());
        let chunk = bytes::Bytes::copy_from_slice(&bytes[start..end]);
        if let Ok(mut p) = progress.lock() {
            p.add(chunk.len() as u64);
        }
        Ok::<_, std::io::Error>(chunk)
    }));
    reqwest::Body::wrap_stream(stream)
}

#[derive(Deserialize)]
struct PutResponse {
    share_key: String,
}

#[derive(Deserialize)]
pub struct ListItem {
    pub share_key: String,
    pub kind: String,
    pub raw_file_name: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
}

pub enum GetResult {
    Text(String),
    File { file_name: String, bytes: Vec<u8> },
}

#[derive(Deserialize)]
pub struct LinkResponse {
    pub token: String,
    pub expires_at: i64,
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn put_url(cfg: &Config, key: Option<&str>, ttl: Option<i64>, public: Option<bool>) -> String {
    let base = match key {
        Some(k) => format!("{}/put/{}", cfg.server_url, k),
        None => format!("{}/put", cfg.server_url),
    };
    let mut query = vec![];
    if let Some(t) = ttl {
        query.push(format!("ttl_secs={t}"));
    }
    if let Some(p) = public {
        query.push(format!("public={p}"));
    }
    if query.is_empty() {
        base
    } else {
        format!("{base}?{}", query.join("&"))
    }
}

pub async fn put_content(
    cfg: &Config,
    key: Option<&str>,
    content: &[u8],
    ttl: Option<i64>,
    public: Option<bool>,
) -> Result<String> {
    let url = put_url(cfg, key, ttl, public);
    let progress = Arc::new(Mutex::new(Progress::new(content.len() as u64)));
    let resp: Result<PutResponse> = with_retry(|| async {
        // Each attempt re-sends from byte 0, so the bar has to start over too.
        if let Ok(mut p) = progress.lock() {
            p.reset();
        }
        let body = counting_body(content.to_vec(), progress.clone());
        let resp = client()
            .post(&url)
            .header("X-API-Key", &cfg.api_key)
            .header("Content-Type", "text/plain")
            .header(reqwest::header::CONTENT_LENGTH, content.len())
            .body(body)
            .send()
            .await
            .context("network error")?;
        if let Ok(mut p) = progress.lock() {
            p.finish_upload();
        }
        check_status(&resp)?;
        Ok(resp.json::<PutResponse>().await.context("bad response body")?)
    })
    .await;
    if let Ok(p) = progress.lock() {
        p.clear();
    }
    Ok(resp?.share_key)
}

pub async fn put_file(
    cfg: &Config,
    key: Option<&str>,
    path: &Path,
    ttl: Option<i64>,
    public: Option<bool>,
) -> Result<String> {
    let url = put_url(cfg, key, ttl, public);
    let file_name = path
        .file_name()
        .context("invalid file path")?
        .to_string_lossy()
        .to_string();
    let bytes = std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;

    // The multipart envelope is assembled by hand rather than with
    // `reqwest::multipart`, because progress needs an exact total up front and
    // needs the file bytes to flow through the counting stream. The framing is
    // a few hundred bytes, so counting the whole body keeps the bar honest --
    // it reaches 100% exactly when the last byte is out.
    const BOUNDARY: &str = "----pocketshare-boundary-7Ph3nQx2Kd";
    let head = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n\
         Content-Type: application/octet-stream\r\n\r\n",
        // A quote or newline in the filename would break out of the header;
        // strip rather than reject, since the name is cosmetic here.
        file_name.replace(['"', '\r', '\n'], "")
    );
    let tail = format!("\r\n--{BOUNDARY}--\r\n");

    let mut body_bytes = Vec::with_capacity(head.len() + bytes.len() + tail.len());
    body_bytes.extend_from_slice(head.as_bytes());
    body_bytes.extend_from_slice(&bytes);
    body_bytes.extend_from_slice(tail.as_bytes());
    drop(bytes);

    let total = body_bytes.len();
    let progress = Arc::new(Mutex::new(Progress::new(total as u64)));

    let resp: Result<PutResponse> = with_retry(|| async {
        if let Ok(mut p) = progress.lock() {
            p.reset();
        }
        let body = counting_body(body_bytes.clone(), progress.clone());
        let resp = client()
            .post(&url)
            .header("X-API-Key", &cfg.api_key)
            .header(
                reqwest::header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .header(reqwest::header::CONTENT_LENGTH, total)
            .body(body)
            .send()
            .await
            .context("network error")?;
        if let Ok(mut p) = progress.lock() {
            p.finish_upload();
        }
        check_status(&resp)?;
        Ok(resp.json::<PutResponse>().await.context("bad response body")?)
    })
    .await;
    if let Ok(p) = progress.lock() {
        p.clear();
    }
    Ok(resp?.share_key)
}

pub async fn get(cfg: &Config, key: &str) -> Result<GetResult> {
    let url = format!("{}/get/{}", cfg.server_url, key);
    with_retry(|| async {
        let resp = client()
            .get(&url)
            .header("X-API-Key", &cfg.api_key)
            .send()
            .await
            .context("network error")?;
        check_status(&resp)?;

        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if content_type.starts_with("text/plain") {
            let text = resp.text().await.context("failed reading text body")?;
            Ok(GetResult::Text(text))
        } else {
            let file_name = resp
                .headers()
                .get(reqwest::header::CONTENT_DISPOSITION)
                .and_then(|v| v.to_str().ok())
                .and_then(extract_filename)
                .unwrap_or_else(|| key.to_string());
            let bytes = resp.bytes().await.context("failed reading file body")?.to_vec();
            Ok(GetResult::File { file_name, bytes })
        }
    })
    .await
}

pub async fn list(cfg: &Config) -> Result<Vec<ListItem>> {
    let url = format!("{}/list", cfg.server_url);
    with_retry(|| async {
        let resp = client()
            .get(&url)
            .header("X-API-Key", &cfg.api_key)
            .send()
            .await
            .context("network error")?;
        check_status(&resp)?;
        Ok(resp.json::<Vec<ListItem>>().await.context("bad response body")?)
    })
    .await
}

pub async fn create_link(cfg: &Config, key: &str) -> Result<LinkResponse> {
    let url = format!("{}/link/{}", cfg.server_url, key);
    with_retry(|| async {
        let resp = client()
            .post(&url)
            .header("X-API-Key", &cfg.api_key)
            .send()
            .await
            .context("network error")?;
        check_status(&resp)?;
        Ok(resp.json::<LinkResponse>().await.context("bad response body")?)
    })
    .await
}

pub async fn delete(cfg: &Config, key: &str) -> Result<()> {
    let url = format!("{}/delete/{}", cfg.server_url, key);
    with_retry(|| async {
        let resp = client()
            .delete(&url)
            .header("X-API-Key", &cfg.api_key)
            .send()
            .await
            .context("network error")?;
        check_status(&resp)?;
        Ok(())
    })
    .await
}

fn check_status(resp: &reqwest::Response) -> Result<()> {
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    if status.is_client_error() {
        return Err(NonRetryable(format!("server returned {status}")).into());
    }
    anyhow::bail!("server returned {status}");
}

fn extract_filename(disposition: &str) -> Option<String> {
    disposition
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix("filename=\"").and_then(|s| s.strip_suffix('"')))
        .map(str::to_string)
}
