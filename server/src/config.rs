use clap::Parser;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// The only command line argument: where the config file lives. Everything
/// else is configured through that file so it can be edited and picked up by
/// a plain service restart.
#[derive(Parser, Debug, Clone)]
#[command(about = "PocketShare server. all settings live in the config file (see --config).")]
pub struct Args {
    /// Path to the TOML config file. Created with defaults on first run.
    #[arg(long, default_value_os_t = default_config_path())]
    pub config: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub listen_addr: String,
    pub data_dir: PathBuf,
    pub path_prefix: String,
    pub max_file_bytes: u64,
    pub default_ttl_secs: i64,
    pub cleanup_interval_secs: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:3000".to_string(),
            data_dir: default_data_dir(),
            path_prefix: String::new(),
            max_file_bytes: 100 * 1024 * 1024,
            default_ttl_secs: 3600,
            cleanup_interval_secs: 60,
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub settings: Settings,
    pub config_path: PathBuf,
    pub api_key: String,
    pub db_path: PathBuf,
    pub assets_dir: PathBuf,
}

fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn default_config_path() -> PathBuf {
    config_home().join("myshare-server/config.toml")
}

fn default_data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(|h| PathBuf::from(h).join("myshare-server"))
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".local/share/myshare-server"))
        })
        .unwrap_or_else(|| PathBuf::from("./data"))
}

/// Normalizes a user-supplied path prefix into either an empty string or a
/// `/foo/bar` form (leading slash, no trailing slash), so callers can always
/// build routes as `format!("{prefix}/put")`.
pub fn normalize_path_prefix(raw: &str) -> String {
    let trimmed = raw.trim().trim_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("/{trimmed}")
    }
}

/// The commented template written on first run. Kept as a literal (rather than
/// serializing `Settings`) so the file the user edits explains each field.
fn config_template(settings: &Settings) -> String {
    format!(
        r#"# PocketShare 配置文件
#
# 改完这个文件后重启服务即可生效:
#   systemctl --user restart pocket-share-server

# 监听地址。默认只监听本机;要让局域网/公网访问,改成 "0.0.0.0:3000",
# 并务必配合防火墙或反向代理 + HTTPS。
listen_addr = "{listen_addr}"

# 数据目录:sqlite 数据库、上传的文件、api_key 都放在这里。
data_dir = {data_dir}

# 路径前缀。留空表示挂在域名根路径下。
# 如果这台机器上域名根路径已经被别的服务占用,可以填 "/myshare",
# 这样 Web 页面在 https://example.com/myshare/ ,接口也都在该前缀下。
# 反向代理原样透传即可,不要 strip 掉前缀。
path_prefix = "{path_prefix}"

# 单个文件大小上限(字节)。默认 100 MiB。
max_file_bytes = {max_file_bytes}

# 分享的默认有效期(秒)。默认 1 小时。
default_ttl_secs = {default_ttl_secs}

# 过期数据的清理间隔(秒)。
cleanup_interval_secs = {cleanup_interval_secs}
"#,
        listen_addr = settings.listen_addr,
        data_dir = toml::Value::String(settings.data_dir.to_string_lossy().into_owned()),
        path_prefix = settings.path_prefix,
        max_file_bytes = settings.max_file_bytes,
        default_ttl_secs = settings.default_ttl_secs,
        cleanup_interval_secs = settings.cleanup_interval_secs,
    )
}

fn load_settings(path: &Path) -> anyhow::Result<Settings> {
    if !path.exists() {
        let settings = Settings::default();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, config_template(&settings))?;
        tracing::info!("created default config at {}", path.display());
        return Ok(settings);
    }
    let raw = fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("读取配置文件 {} 失败: {e}", path.display()))?;
    toml::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("解析配置文件 {} 失败: {e}", path.display()))
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let args = Args::parse();
        let mut settings = load_settings(&args.config)?;
        settings.path_prefix = normalize_path_prefix(&settings.path_prefix);
        fs::create_dir_all(&settings.data_dir)?;
        let assets_dir = settings.data_dir.join("assets");
        fs::create_dir_all(&assets_dir)?;
        let db_path = settings.data_dir.join("myshare.sqlite");
        let api_key = load_or_create_api_key(&settings.data_dir)?;
        Ok(Config { settings, config_path: args.config, api_key, db_path, assets_dir })
    }
}

fn load_or_create_api_key(data_dir: &Path) -> anyhow::Result<String> {
    let key_path = data_dir.join("api_key");
    if key_path.exists() {
        let key = fs::read_to_string(&key_path)?;
        return Ok(key.trim().to_string());
    }
    let key: String = {
        let mut rng = rand::thread_rng();
        (0..32)
            .map(|_| {
                let choices = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                choices[rng.gen_range(0..choices.len())] as char
            })
            .collect()
    };
    fs::write(&key_path, &key)?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_config_round_trips_windows_paths() {
        let settings = Settings {
            data_dir: PathBuf::from(r"C:\Users\test\AppData\Local\myshare-server"),
            ..Settings::default()
        };
        let parsed: Settings = toml::from_str(&config_template(&settings)).unwrap();
        assert_eq!(parsed.data_dir, settings.data_dir);
    }
}
