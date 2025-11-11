use smol::{Timer, lock::Mutex};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub struct IdleTracker {
    pub last_active: Instant,
    pub num_connections: usize,
    pub should_shutdown: bool,
    pub max_idle_time: Duration,
}

impl IdleTracker {
    pub fn new() -> Self {
        Self {
            last_active: Instant::now(),
            num_connections: 0,
            should_shutdown: false,
            max_idle_time: Duration::from_secs(5),
        }
    }

    pub fn add_connection(&mut self) -> Result<(), ()> {
        if self.should_shutdown {
            return Err(());
        }

        self.num_connections += 1;
        self.last_active = Instant::now();
        Ok(())
    }

    pub fn remove_connection(&mut self) {
        self.num_connections -= 1;
    }

    pub fn shutting_down(&self) -> bool {
        self.num_connections == 0 && self.should_shutdown
    }

    pub fn shutdown_if_idle(&mut self) -> bool {
        if self.last_active + self.max_idle_time < Instant::now() {
            self.should_shutdown = true;
            true
        } else {
            false
        }
    }
}

pub async fn track_idle_time(idle_tracker: Arc<Mutex<IdleTracker>>) {
    loop {
        Timer::after(Duration::from_millis(500)).await;

        if idle_tracker.lock().await.shutdown_if_idle() {
            break;
        }
    }
}
