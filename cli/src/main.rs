mod client;
mod config;
mod retry;

use anyhow::{bail, Context, Result};
use client::GetResult;
use config::Config;
use std::io::Read;
use std::path::{Path, PathBuf};

fn config_path_display() -> String {
    config::config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "(could not determine config dir)".to_string())
}

fn usage() -> ! {
    eprintln!(
        "usage:
  share login <api_key> [--server <url>]
  share put [<share_key>] [<file_path>|<text>] [--ttl <secs>] [--private]
  share get <share_key> [<out_path>]
  share delete <share_key>
  share list

config file: {}",
        config_path_display()
    );
    std::process::exit(1);
}

fn print_help() {
    println!(
        "share - CLI for the personal CLI-Web share tool

usage:
  share login <api_key> [--server <url>]   save server url + api_key
  share put [<share_key>] [<file_path>|<text>] [--ttl <secs>] [--private]
  share get <share_key> [<out_path>]
  share delete <share_key>
  share list
  share help | -h | --help

config file: {}",
        config_path_display()
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
    }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "login" => cmd_login(args),
        "put" => cmd_put(args).await,
        "get" => cmd_get(args).await,
        "delete" => cmd_delete(args).await,
        "list" => cmd_list(args).await,
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        _ => usage(),
    }
}

fn cmd_login(mut args: Vec<String>) -> Result<()> {
    let server_url = take_flag_value(&mut args, "--server").unwrap_or_else(|| "http://127.0.0.1:3000".to_string());
    if args.len() != 1 {
        usage();
    }
    let api_key = args.remove(0);
    let cfg = Config {
        server_url: server_url.trim_end_matches('/').to_string(),
        api_key,
    };
    cfg.save()?;
    println!("logged in to {}", cfg.server_url);
    Ok(())
}

fn is_share_key(s: &str) -> bool {
    (s.len() == 3 || s.len() == 4) && s.chars().all(|c| c.is_ascii_digit())
}

fn take_flag_value(args: &mut Vec<String>, flag: &str) -> Option<String> {
    if let Some(pos) = args.iter().position(|a| a == flag) {
        args.remove(pos);
        if pos < args.len() {
            return Some(args.remove(pos));
        }
    }
    None
}

fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
    if let Some(pos) = args.iter().position(|a| a == flag) {
        args.remove(pos);
        true
    } else {
        false
    }
}

async fn cmd_put(mut args: Vec<String>) -> Result<()> {
    let cfg = Config::load()?;
    let ttl: Option<i64> = take_flag_value(&mut args, "--ttl")
        .map(|v| v.parse().context("--ttl must be a number of seconds"))
        .transpose()?;
    let private = take_flag(&mut args, "--private");
    let public = if private { Some(false) } else { None };

    let (key, content_arg): (Option<String>, Option<String>) = match args.len() {
        0 => (None, None),
        1 => {
            if is_share_key(&args[0]) {
                (Some(args.remove(0)), None)
            } else {
                (None, Some(args.remove(0)))
            }
        }
        2 => (Some(args.remove(0)), Some(args.remove(0))),
        _ => usage(),
    };

    let share_key = match content_arg {
        Some(content) if Path::new(&content).is_file() => {
            client::put_file(&cfg, key.as_deref(), Path::new(&content), ttl, public).await?
        }
        Some(text) => client::put_content(&cfg, key.as_deref(), text.as_bytes(), ttl, public).await?,
        None => {
            let mut buf = Vec::new();
            std::io::stdin()
                .read_to_end(&mut buf)
                .context("failed reading stdin")?;
            if buf.is_empty() {
                bail!("no content provided: pass a file path, text, or pipe data via stdin");
            }
            client::put_content(&cfg, key.as_deref(), &buf, ttl, public).await?
        }
    };

    println!("{share_key}");
    Ok(())
}

async fn cmd_get(mut args: Vec<String>) -> Result<()> {
    let cfg = Config::load()?;
    if args.is_empty() || args.len() > 2 {
        usage();
    }
    let key = args.remove(0);
    let out_arg = if args.is_empty() { None } else { Some(args.remove(0)) };

    match client::get(&cfg, &key).await? {
        GetResult::Text(text) => {
            if let Some(out) = out_arg {
                let out_path = resolve_out_path(&out, "share.txt");
                std::fs::write(&out_path, text)?;
                println!("saved to {}", out_path.display());
            } else {
                print!("{text}");
            }
        }
        GetResult::File { file_name, bytes } => {
            let out_path = match out_arg {
                Some(out) => resolve_out_path(&out, &file_name),
                None => PathBuf::from(&file_name),
            };
            if let Some(parent) = out_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            std::fs::write(&out_path, bytes)?;
            println!("saved to {}", out_path.display());
        }
    }
    Ok(())
}

async fn cmd_delete(args: Vec<String>) -> Result<()> {
    if args.len() != 1 {
        usage();
    }
    let cfg = Config::load()?;
    client::delete(&cfg, &args[0]).await?;
    println!("deleted {}", args[0]);
    Ok(())
}

fn resolve_out_path(out: &str, default_file_name: &str) -> PathBuf {
    let p = PathBuf::from(out);
    if out.ends_with('/') || p.is_dir() {
        p.join(default_file_name)
    } else {
        p
    }
}

async fn cmd_list(args: Vec<String>) -> Result<()> {
    if !args.is_empty() {
        usage();
    }
    let cfg = Config::load()?;
    let items = client::list(&cfg).await?;
    if items.is_empty() {
        println!("(no public shares)");
        return Ok(());
    }
    for item in items {
        let name = item.raw_file_name.unwrap_or_else(|| "-".to_string());
        println!(
            "{}\t{}\t{}\tcreated_at={}\texpires_at={}",
            item.share_key, item.kind, name, item.created_at, item.expires_at
        );
    }
    Ok(())
}
