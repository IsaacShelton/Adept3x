use crate::data_units::ByteUnits;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

pub struct CompilationStats {
    pub start_time: Instant,
    files_processed: AtomicU64,
    bytes_processed: AtomicU64,
    num_files_failed: AtomicU64,
    num_module_files_failed: AtomicU64,
}

impl CompilationStats {
    pub fn start() -> Self {
        Self {
            start_time: Instant::now(),
            files_processed: AtomicU64::new(0),
            bytes_processed: AtomicU64::new(0),
            num_files_failed: AtomicU64::new(0),
            num_module_files_failed: AtomicU64::new(0),
        }
    }

    pub fn seconds_elapsed(&self) -> f64 {
        self.start_time.elapsed().as_millis() as f64 / 1000.0
    }

    pub fn failed_files_estimate(&self) -> u64 {
        self.num_files_failed.load(Ordering::Relaxed)
    }

    pub fn failed_modules_estimate(&self) -> u64 {
        self.num_module_files_failed.load(Ordering::Relaxed)
    }

    pub fn bytes_processed_estimate(&self) -> u64 {
        self.bytes_processed.load(Ordering::Relaxed)
    }

    pub fn files_processed_estimate(&self) -> u64 {
        self.files_processed.load(Ordering::Relaxed)
    }

    pub fn fail_file(&self) {
        self.num_files_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn fail_module_file(&self) {
        self.num_module_files_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn process_file(&self) {
        self.files_processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn process_bytes(&self, count: ByteUnits) {
        self.bytes_processed
            .fetch_add(count.bytes(), Ordering::Relaxed);
    }
}
