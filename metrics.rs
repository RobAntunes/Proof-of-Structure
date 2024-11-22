use std::time::{Duration, Instant};
use blake3::Hash;
use tracing::info;

/// Local node metrics with zero synchronization
#[derive(Debug, Clone)]
pub struct NodeMetrics {
    node_id: String,
    start_time: Instant,
    last_window_start: Instant,
    window_tx_count: u64,
    total_tx_count: u64,
    last_processing_time: Duration,
    warmup_complete: bool,
}

impl NodeMetrics {
    pub fn new(node_id: impl Into<String>, _window_size: u64) -> Self {
        let now = Instant::now();
        Self {
            node_id: node_id.into(),
            start_time: now,
            last_window_start: now,
            window_tx_count: 0,
            total_tx_count: 0,
            last_processing_time: Duration::from_nanos(0),
            warmup_complete: false,
        }
    }

    /// Record a batch of processed transactions and log metrics
    pub fn record_batch(&mut self, num_transactions: u64, processing_time: Duration) {
        // Update counters
        self.total_tx_count = self.total_tx_count.saturating_add(num_transactions);
        self.window_tx_count = self.window_tx_count.saturating_add(num_transactions);
        
        // Update processing time using weighted average
        let old_weight = 0.8;
        let new_weight = 1.0 - old_weight;
        let old_nanos = self.last_processing_time.as_nanos() as f64;
        let new_nanos = processing_time.as_nanos() as f64 / num_transactions as f64; // Per-transaction time
        let avg_nanos = (old_weight * old_nanos + new_weight * new_nanos) as u128;
        self.last_processing_time = Duration::from_nanos(avg_nanos as u64);

        // Check if it's time to log metrics (every second)
        let window_elapsed = self.last_window_start.elapsed();
        if window_elapsed >= Duration::from_secs(1) {
            self.log_metrics();
            self.window_tx_count = 0;
            self.last_window_start = Instant::now();
        }
    }

    /// Log current metrics
    fn log_metrics(&self) {
        let window_elapsed = self.last_window_start.elapsed();
        let window_secs = window_elapsed.as_secs_f64();
        let current_tps = if window_secs > 0.0 {
            self.window_tx_count as f64 / window_secs
        } else {
            0.0
        };
        
        let total_elapsed = self.start_time.elapsed();
        let total_secs = total_elapsed.as_secs_f64();
        let avg_tps = if total_secs > 0.0 && self.warmup_complete {
            self.total_tx_count as f64 / total_secs
        } else {
            0.0
        };

        // Log metrics only if we have data
        if self.total_tx_count > 0 || current_tps > 0.0 {
            info!(
                node_id = %self.node_id,
                current_tps = %current_tps,
                average_tps = %avg_tps,
                total_transactions = %self.total_tx_count,
                average_latency_ms = %self.last_processing_time.as_millis(),
                "Node metrics"
            );
        }
    }

    /// Mark warmup as complete
    pub fn complete_warmup(&mut self) {
        self.warmup_complete = true;
        self.start_time = Instant::now(); // Reset start time after warmup
        self.total_tx_count = 0; // Reset counters after warmup
        self.window_tx_count = 0;
        self.last_window_start = Instant::now();
    }

    /// Get total transactions processed
    pub fn total_transactions(&self) -> u64 {
        self.total_tx_count
    }

    /// Get historical average TPS
    pub fn historical_average_tps(&self) -> f64 {
        let total_time = self.start_time.elapsed().as_secs_f64();
        self.total_tx_count as f64 / total_time
    }

    /// Get average processing time
    pub fn average_processing_time(&self) -> Duration {
        self.last_processing_time
    }
}
