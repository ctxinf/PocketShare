use crate::AppState;
use std::time::Duration;

pub fn spawn(state: AppState) {
    let interval_secs = state.config.args.cleanup_interval_secs;
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
    let deleted = crate::db::delete_expired(&conn, now)?;
    if deleted > 0 {
        tracing::info!("cleanup: removed {} expired share(s)", deleted);
    }
    Ok(())
}
