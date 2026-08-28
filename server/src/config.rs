use clap::Parser;
use rand::Rng;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(long, env = "SHARE_LISTEN_ADDR", default_value = "127.0.0.1:3000")]
    pub listen_addr: String,

    #[arg(long, env = "SHARE_DATA_DIR", default_value = "./data")]
    pub data_dir: PathBuf,

    #[arg(long, env = "SHARE_MAX_FILE_BYTES", default_value_t = 100 * 1024 * 1024)]
    pub max_file_bytes: u64,

    #[arg(long, env = "SHARE_DEFAULT_TTL_SECS", default_value_t = 3600)]
    pub default_ttl_secs: i64,

    #[arg(long, env = "SHARE_CLEANUP_INTERVAL_SECS", default_value_t = 60)]
    pub cleanup_interval_secs: u64,
}

#[derive(Clone)]
pub struct Config {
    pub args: Args,
    pub api_key: String,
    pub db_path: PathBuf,
    pub assets_dir: PathBuf,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let args = Args::parse();
        fs::create_dir_all(&args.data_dir)?;
        let assets_dir = args.data_dir.join("assets");
        fs::create_dir_all(&assets_dir)?;
        let db_path = args.data_dir.join("share.sqlite");
        let api_key = load_or_create_api_key(&args.data_dir)?;
        Ok(Config { args, api_key, db_path, assets_dir })
    }
}

fn load_or_create_api_key(data_dir: &PathBuf) -> anyhow::Result<String> {
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
