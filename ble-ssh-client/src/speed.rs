use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Tracks bytes transferred and computes real-time speed.
pub struct SpeedTracker {
    tx_bytes: Arc<AtomicU64>,
    rx_bytes: Arc<AtomicU64>,
    start: Instant,
    last_tx: AtomicU64,
    last_rx: AtomicU64,
    last_time: std::sync::Mutex<Instant>,
}

impl SpeedTracker {
    pub fn new() -> Arc<Self> {
        let now = Instant::now();
        Arc::new(Self {
            tx_bytes: Arc::new(AtomicU64::new(0)),
            rx_bytes: Arc::new(AtomicU64::new(0)),
            start: now,
            last_tx: AtomicU64::new(0),
            last_rx: AtomicU64::new(0),
            last_time: std::sync::Mutex::new(now),
        })
    }

    pub fn add_tx(&self, bytes: u64) {
        self.tx_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn add_rx(&self, bytes: u64) {
        self.rx_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn total_tx(&self) -> u64 {
        self.tx_bytes.load(Ordering::Relaxed)
    }

    pub fn total_rx(&self) -> u64 {
        self.rx_bytes.load(Ordering::Relaxed)
    }

    /// Returns (tx_speed_bytes_per_sec, rx_speed_bytes_per_sec) since last call.
    pub fn speed(&self) -> (f64, f64) {
        let now = Instant::now();
        let mut last = self.last_time.lock().unwrap();
        let elapsed = now.duration_since(*last).as_secs_f64();
        if elapsed < 0.001 {
            return (0.0, 0.0);
        }

        let cur_tx = self.tx_bytes.load(Ordering::Relaxed);
        let cur_rx = self.rx_bytes.load(Ordering::Relaxed);
        let prev_tx = self.last_tx.swap(cur_tx, Ordering::Relaxed);
        let prev_rx = self.last_rx.swap(cur_rx, Ordering::Relaxed);
        *last = now;

        let tx_speed = (cur_tx - prev_tx) as f64 / elapsed;
        let rx_speed = (cur_rx - prev_rx) as f64 / elapsed;
        (tx_speed, rx_speed)
    }

    pub fn elapsed(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

pub fn format_bytes(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1024.0 * 1024.0 {
        format!("{:.1} MB/s", bytes_per_sec / (1024.0 * 1024.0))
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.1} KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

pub fn format_total(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
