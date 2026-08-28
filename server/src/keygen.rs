use crate::db;
use crate::error::AppError;
use rand::Rng;
use rusqlite::Connection;

const MAX_RETRIES: u32 = 20;
const THREE_DIGIT_SPACE: i64 = 1000;
const FOUR_DIGIT_SPACE: i64 = 10000;
const UPGRADE_THRESHOLD: f64 = 0.8;

pub fn generate_share_key(conn: &Connection, now: i64) -> Result<String, AppError> {
    let active = db::active_count(conn, now)?;
    let use_four_digits = (active as f64) / (THREE_DIGIT_SPACE as f64) > UPGRADE_THRESHOLD;
    let digits = if use_four_digits { 4 } else { 3 };
    let space = if use_four_digits { FOUR_DIGIT_SPACE } else { THREE_DIGIT_SPACE };

    let mut rng = rand::thread_rng();
    for _ in 0..MAX_RETRIES {
        let candidate: i64 = rng.gen_range(0..space);
        let key = format!("{:0width$}", candidate, width = digits);
        if !db::key_exists(conn, &key)? {
            return Ok(key);
        }
    }
    Err(AppError::CapacityFull)
}
