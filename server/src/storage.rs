use crate::crypto;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

pub fn asset_path(assets_dir: &Path, share_key: &str) -> PathBuf {
    assets_dir.join(share_key)
}

pub async fn write_asset(assets_dir: &Path, share_key: &str, data: Vec<u8>) -> std::io::Result<()> {
    let path = asset_path(assets_dir, share_key);
    let obfuscated = crypto::obfuscate(&data);
    tokio::fs::write(path, obfuscated).await
}

pub async fn asset_len(assets_dir: &Path, share_key: &str) -> std::io::Result<u64> {
    let path = asset_path(assets_dir, share_key);
    let meta = tokio::fs::metadata(path).await?;
    Ok(meta.len())
}

pub async fn read_asset(assets_dir: &Path, share_key: &str) -> std::io::Result<Vec<u8>> {
    let path = asset_path(assets_dir, share_key);
    let raw = tokio::fs::read(path).await?;
    Ok(crypto::deobfuscate(&raw))
}

/// Reads and decrypts only the plaintext byte range `[start, end_exclusive)`,
/// without decrypting the rest of the file. Seeks to the nearest BLOCK-aligned
/// boundary on disk, reads just that span, and trims after decrypting.
pub async fn read_asset_range(
    assets_dir: &Path,
    share_key: &str,
    start: u64,
    end_exclusive: u64,
) -> std::io::Result<Vec<u8>> {
    let block = crypto::BLOCK as u64;
    let path = asset_path(assets_dir, share_key);
    let total_len = tokio::fs::metadata(&path).await?.len();
    let end_exclusive = end_exclusive.min(total_len);
    if start >= end_exclusive {
        return Ok(Vec::new());
    }

    let block_start = (start / block) * block;
    let block_end = (((end_exclusive + block - 1) / block) * block).min(total_len);

    let mut file = tokio::fs::File::open(&path).await?;
    file.seek(std::io::SeekFrom::Start(block_start)).await?;
    let mut raw = vec![0u8; (block_end - block_start) as usize];
    file.read_exact(&mut raw).await?;

    let decrypted = crypto::deobfuscate_range(&raw, block_start as usize);
    let trim_start = (start - block_start) as usize;
    let trim_end = trim_start + (end_exclusive - start) as usize;
    Ok(decrypted[trim_start..trim_end].to_vec())
}

pub async fn delete_asset(assets_dir: &Path, share_key: &str) -> std::io::Result<()> {
    let path = asset_path(assets_dir, share_key);
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}
