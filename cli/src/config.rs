use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub server_url: String,
    pub api_key: String,
}

pub fn config_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("could not determine config dir")?
        .join("pshare");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("config.json"))
}

impl Config {
    pub fn load() -> Result<Config> {
        let path = config_path()?;
        let data = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "not logged in yet (no config at {}). run `pshare login <api_key> [--server <url>]` first",
                path.display()
            )
        })?;
        let cfg: Config = serde_json::from_str(&data)?;
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }
}
