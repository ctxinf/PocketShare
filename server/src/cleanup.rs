use crate::AppState;
use std::time::Duration;

pub fn spawn(state: AppState) {
    let interval_secs = state.config.settings.cleanup_interval_secs;
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            ticker.tick().await;
            if let Err(e) = run_once(&state).await {
                tracing::warn!("cleanup pass failed: {}", e);
            }
        }
    });
}

async fn run_once(state: &AppState) -> anyhow::Result<()> {
    let now = crate::now_ts();
    let (keys, assets_dir) = {
        let conn = state.db.lock().await;
        let keys = crate::db::expired_keys(&conn, now)?;
        (keys, state.config.assets_dir.clone())
    };
    for key in &keys {
        crate::storage::delete_asset(&assets_dir, key).await?;
    }
    let conn = state.db.lock().await;
    for key in &keys {
        crate::db::delete_tokens_for_share(&conn, key)?;
    }
    let deleted = crate::db::delete_expired(&conn, now)?;
    if deleted > 0 {
        tracing::info!("cleanup: removed {} expired share(s)", deleted);
    }
    let deleted_tokens = crate::db::delete_expired_tokens(&conn, now)?;
    if deleted_tokens > 0 {
        tracing::info!("cleanup: removed {} expired download token(s)", deleted_tokens);
    }
    Ok(())
}
