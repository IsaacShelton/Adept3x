use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
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
        self.shared.last_active_ms.store(
            Instant::now()
                .duration_since(self.shared.started)
                .as_millis() as u64,
            Ordering::SeqCst,
        );
    }

    pub fn set_max_idle_time(&self, new_max_idle_time: Option<Duration>) {
        match new_max_idle_time {
            Some(new_max_idle_time) => self
                .shared
                .max_idle_time_ms
                .store(new_max_idle_time.as_millis() as u64, Ordering::SeqCst),
            None => self
                .shared
                .max_idle_time_ms
                .store(self.shared.original_max_idle_time_ms, Ordering::SeqCst),
        }
    }

    pub fn add_connection(&self) -> Result<(), ()> {
        self.shared.num_connections.fetch_add(1, Ordering::SeqCst);
        self.still_active();

        if self.is_shutting_down() {
            Err(())
        } else {
            Ok(())
        }
    }

    pub fn remove_connection(&self) {
        self.shared.num_connections.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn is_shutting_down(&self) -> bool {
        self.shared.should_shutdown.load(Ordering::SeqCst)
    }

    pub fn should_shutdown(&self) -> bool {
        let no_connections = self.shared.num_connections.load(Ordering::SeqCst) == 0;

        let now = Instant::now()
            .duration_since(self.shared.started)
            .as_millis() as u64;

        let expire_at = self.shared.last_active_ms.load(Ordering::SeqCst)
            + self.shared.max_idle_time_ms.load(Ordering::SeqCst);

        if no_connections && now > expire_at {
            self.shared.should_shutdown.store(true, Ordering::SeqCst);
            true
        } else {
            self.shared.should_shutdown.load(Ordering::SeqCst)
        }
    }
}
