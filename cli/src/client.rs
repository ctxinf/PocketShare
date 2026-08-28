use crate::config::Config;
use crate::retry::{with_retry, NonRetryable};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

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
    let resp: PutResponse = with_retry(|| async {
        let resp = client()
            .post(&url)
            .header("X-API-Key", &cfg.api_key)
            .header("Content-Type", "text/plain")
            .body(content.to_vec())
            .send()
            .await
            .context("network error")?;
        check_status(&resp)?;
        Ok(resp.json::<PutResponse>().await.context("bad response body")?)
    })
    .await?;
    Ok(resp.share_key)
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

    let resp: PutResponse = with_retry(|| async {
        let part = reqwest::multipart::Part::bytes(bytes.clone()).file_name(file_name.clone());
        let form = reqwest::multipart::Form::new().part("file", part);
        let resp = client()
            .post(&url)
            .header("X-API-Key", &cfg.api_key)
            .multipart(form)
            .send()
            .await
            .context("network error")?;
        check_status(&resp)?;
        Ok(resp.json::<PutResponse>().await.context("bad response body")?)
    })
    .await?;
    Ok(resp.share_key)
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
