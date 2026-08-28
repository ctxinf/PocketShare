use anyhow::Result;
use std::fmt;
use std::future::Future;
use std::time::Duration;

const MAX_RETRIES: u32 = 4;

/// Wrap an error with this to signal that retrying will never help
/// (e.g. 4xx responses like unauthorized or bad request).
#[derive(Debug)]
pub struct NonRetryable(pub String);

impl fmt::Display for NonRetryable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for NonRetryable {}

pub async fn with_retry<T, F, Fut>(mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 0;
    loop {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) if e.downcast_ref::<NonRetryable>().is_some() => return Err(e),
            Err(e) if attempt < MAX_RETRIES => {
                attempt += 1;
                let backoff = Duration::from_millis(300 * 2u64.pow(attempt));
                eprintln!("request failed ({e}), retrying ({attempt}/{MAX_RETRIES}) in {backoff:?}...");
                tokio::time::sleep(backoff).await;
            }
            Err(e) => return Err(e),
        }
    }
}
