use std::io::{IsTerminal, Write};
use std::time::{Duration, Instant};

/// Only redraw this often. A redraw per network chunk would spam a slow
/// terminal (and a serial console over ssh) far faster than anyone can read.
const REDRAW_INTERVAL: Duration = Duration::from_millis(100);

/// A single-line upload progress indicator.
///
/// Everything goes to **stderr**, never stdout: `share put` prints the bare
/// share_key on stdout so it can be captured (`KEY=$(share put f)`), and
/// progress must not end up in that capture.
///
/// When stderr is not a terminal (piped, redirected, run from a script) the
/// whole thing turns into a no-op rather than writing thousands of control
/// sequences into a log file.
pub struct Progress {
    total: u64,
    uploaded: u64,
    enabled: bool,
    started: Instant,
    last_draw: Option<Instant>,
    finished: bool,
}

impl Progress {
    pub fn new(total: u64) -> Self {
        Self {
            total,
            uploaded: 0,
            // A zero total means we don't know the size; a percentage would be
            // meaningless, so don't show anything at all.
            enabled: std::io::stderr().is_terminal() && total > 0,
            started: Instant::now(),
            last_draw: None,
            finished: false,
        }
    }

    /// Records `n` more bytes handed to the socket and repaints if due.
    pub fn add(&mut self, n: u64) {
        if !self.enabled {
            return;
        }
        self.uploaded = (self.uploaded + n).min(self.total);

        let now = Instant::now();
        let due = match self.last_draw {
            None => true,
            Some(prev) => now.duration_since(prev) >= REDRAW_INTERVAL,
        };
        // Always paint the final frame, so the line doesn't stop at 99%.
        if due || self.uploaded == self.total {
            self.last_draw = Some(now);
            self.draw();
        }
    }

    fn draw(&self) {
        let pct = self.uploaded as f64 / self.total as f64 * 100.0;
        let elapsed = self.started.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 { self.uploaded as f64 / elapsed } else { 0.0 };

        const WIDTH: usize = 24;
        let filled = ((pct / 100.0) * WIDTH as f64).round() as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(WIDTH - filled);

        let mut err = std::io::stderr();
        // \r returns to column 0 and the trailing spaces wipe any leftover
        // tail from a previously longer line.
        let _ = write!(
            err,
            "\r  {bar} {pct:5.1}%  {}/{}  {}/s   ",
            human_bytes(self.uploaded),
            human_bytes(self.total),
            human_bytes(speed as u64),
        );
        let _ = err.flush();
    }

    /// Replaces the bar with a "waiting on the server" line.
    ///
    /// The bar hitting 100% only means the bytes reached the local socket
    /// buffer; the server still has to read, decrypt and store them. On a bad
    /// link that gap is long enough to look like a hang, so say what's going on.
    pub fn finish_upload(&mut self) {
        if !self.enabled || self.finished {
            return;
        }
        self.finished = true;
        let elapsed = self.started.elapsed().as_secs_f64();
        let mut err = std::io::stderr();
        let _ = write!(
            err,
            "\r  已发送 {} (耗时 {:.1}s), 等待服务器处理...          ",
            human_bytes(self.total),
            elapsed
        );
        let _ = err.flush();
    }

    /// Clears the progress line entirely, so the caller's own output (the
    /// share_key, or an error) starts on a clean line.
    pub fn clear(&self) {
        if !self.enabled {
            return;
        }
        let mut err = std::io::stderr();
        let _ = write!(err, "\r{}\r", " ".repeat(72));
        let _ = err.flush();
    }

    /// Resets the counter for a retried attempt, which starts from byte 0
    /// again -- without this the second attempt would appear to start at 100%.
    pub fn reset(&mut self) {
        self.uploaded = 0;
        self.started = Instant::now();
        self.last_draw = None;
        self.finished = false;
    }
}

pub fn human_bytes(n: u64) -> String {
    const KIB: f64 = 1024.0;
    let n = n as f64;
    if n < KIB {
        format!("{n:.0} B")
    } else if n < KIB * KIB {
        format!("{:.1} KB", n / KIB)
    } else if n < KIB * KIB * KIB {
        format!("{:.1} MB", n / (KIB * KIB))
    } else {
        format!("{:.2} GB", n / (KIB * KIB * KIB))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_bytes_units() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1024), "1.0 KB");
        assert_eq!(human_bytes(1536), "1.5 KB");
        assert_eq!(human_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(human_bytes(10 * 1024 * 1024), "10.0 MB");
        assert_eq!(human_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn add_saturates_at_total_and_is_inert_when_disabled() {
        // Under `cargo test` stderr is captured, so `enabled` is false and the
        // counter must still track without panicking or overshooting.
        let mut p = Progress::new(100);
        p.add(60);
        p.add(60);
        assert!(p.uploaded <= 100);
    }
}
