use crate::idle::IdleTracker;
use smol::{Timer, lock::Mutex};
use std::{sync::Arc, time::Duration};

pub struct FsWatcher {}

impl FsWatcher {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn watch(self: Arc<Self>, idle_tracker: Arc<Mutex<IdleTracker>>) {
        let watch_thread = std::thread::spawn(|| {});

        loop {
            Timer::after(Duration::from_millis(500)).await;

            if idle_tracker.lock().await.shutting_down() {
                let _ = watch_thread.join();
                break;
            }
        }
    }
}
