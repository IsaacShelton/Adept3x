use smol::Timer;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

pub struct IdleTracker {
    shared: IdleTrackerShared,
}

pub struct IdleTrackerShared {
    started: Instant,
    last_active_ms: AtomicU64,
    should_shutdown: AtomicBool,
    num_connections: AtomicU64,
    max_idle_time_ms: AtomicU64,
    original_max_idle_time_ms: u64,
}

impl IdleTracker {
    pub fn new(max_idle_time: Duration) -> Self {
        let started = Instant::now();
        let max_idle_time_ms = max_idle_time.as_millis() as u64;

        Self {
            shared: IdleTrackerShared {
                started,
                last_active_ms: AtomicU64::new(0),
                num_connections: 0.into(),
                should_shutdown: false.into(),
                max_idle_time_ms: max_idle_time_ms.into(),
                original_max_idle_time_ms: max_idle_time_ms,
            },
        }
    }

    pub fn still_active(&self) {
        // NOTE: It's okay if it's not monotonic, we can save some performance this way

        let ms = Instant::now()
            .duration_since(self.shared.started)
            .as_millis() as u64;

        self.shared.last_active_ms.store(ms, Ordering::Relaxed);
    }

    pub fn set_max_idle_time(&self, new_max_idle_time: Option<Duration>) {
        match new_max_idle_time {
            Some(new_max_idle_time) => {
                let max_idle_time_ms = new_max_idle_time.as_millis() as u64;

                self.shared
                    .max_idle_time_ms
                    .store(max_idle_time_ms, Ordering::Relaxed)
            }
            None => self
                .shared
                .max_idle_time_ms
                .store(self.shared.original_max_idle_time_ms, Ordering::Relaxed),
        }
    }

    pub fn add_connection(&self) -> Result<(), ()> {
        if self.shutting_down() {
            return Err(());
        }

        self.shared.num_connections.fetch_add(1, Ordering::Relaxed);
        self.still_active();
        Ok(())
    }

    pub fn remove_connection(&self) {
        self.shared.num_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn shutting_down(&self) -> bool {
        self.shared.num_connections.load(Ordering::Relaxed) == 0
            && self.shared.should_shutdown.load(Ordering::Relaxed)
    }

    pub fn shutdown_if_idle(&self) -> bool {
        let no_connections = self.shared.num_connections.load(Ordering::Relaxed) == 0;

        let now = Instant::now()
            .duration_since(self.shared.started)
            .as_millis() as u64;

        let expire_at = self.shared.last_active_ms.load(Ordering::Relaxed)
            + self.shared.max_idle_time_ms.load(Ordering::Relaxed);

        if no_connections && now > expire_at {
            self.shared.should_shutdown.store(true, Ordering::Relaxed);
            true
        } else {
            self.shared.should_shutdown.load(Ordering::Relaxed)
        }
    }
}

pub async fn track_idle_time(idle_tracker: Arc<IdleTracker>) {
    loop {
        Timer::after(Duration::from_millis(500)).await;

        if idle_tracker.shutdown_if_idle() {
            break;
        }
    }
}
