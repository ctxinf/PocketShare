use crate::crypto;
use std::path::{Path, PathBuf};

pub fn asset_path(assets_dir: &Path, share_key: &str) -> PathBuf {
    assets_dir.join(share_key)
}

pub async fn write_asset(assets_dir: &Path, share_key: &str, data: Vec<u8>) -> std::io::Result<()> {
    let path = asset_path(assets_dir, share_key);
    let obfuscated = crypto::obfuscate(&data);
    tokio::fs::write(path, obfuscated).await
}

pub async fn read_asset(assets_dir: &Path, share_key: &str) -> std::io::Result<Vec<u8>> {
    let path = asset_path(assets_dir, share_key);
    let raw = tokio::fs::read(path).await?;
    Ok(crypto::deobfuscate(&raw))
}

pub async fn delete_asset(assets_dir: &Path, share_key: &str) -> std::io::Result<()> {
    let path = asset_path(assets_dir, share_key);
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}
