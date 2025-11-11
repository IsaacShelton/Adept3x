use smol::{
    Timer,
    io::{self, AsyncWriteExt},
    lock::Mutex,
    net::{TcpListener, TcpStream},
    prelude::*,
};
use std::{
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

#[derive(Debug)]
pub enum Event {}

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

pub fn server_main() -> io::Result<()> {
    let (_rx, _tx) = mpsc::channel::<Event>();

    smol::block_on(async {
        // Create a listener.
        let listener = TcpListener::bind("127.0.0.1:6000").await?;
        println!("Listening on {}", listener.local_addr()?);
        let mut incoming = listener.incoming();
        let idle_tracker = Arc::new(Mutex::new(IdleTracker::new()));

        {
            let idle_tracker = idle_tracker.clone();

            smol::spawn(async move {
                loop {
                    Timer::after(Duration::from_millis(500)).await;
                    if idle_tracker.lock().await.shutdown_if_idle() {
                        break;
                    }
                }
            })
            .detach();
        }

        loop {
            let mut next_connection = async || Ok(incoming.next().await);
            let timeout = async || Err(Timer::after(Duration::from_millis(100)).await);

            if let Ok(Some(Ok(connection))) = smol::future::or(next_connection(), timeout()).await {
                let idle_tracker = idle_tracker.clone();

                smol::spawn(async move {
                    if idle_tracker.lock().await.add_connection().is_ok() {
                        let _ = handle_client(connection).await;
                        idle_tracker.lock().await.remove_connection();
                    }
                })
                .detach();
            }

            if idle_tracker.lock().await.shutting_down() {
                break;
            }
        }

        Ok(())
    })
}

async fn handle_client(mut stream: TcpStream) -> io::Result<()> {
    println!("Server received connection");
    stream.write_all(b"hello\nworld!").await.unwrap();
    stream.close().await.unwrap();
    Ok(())
}
