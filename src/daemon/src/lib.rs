mod daemon;
mod handle_client;
mod logger;
mod show;

pub use crate::daemon::Daemon;
use std::{io, sync::Arc};

pub fn main_loop(daemon: Daemon) -> io::Result<()> {
    let daemon = Arc::new(daemon);

    if logger::setup().is_err() {
        eprintln!("Failed to create daemon log file");
    }

    #[cfg(target_family = "unix")]
    {
        use crate::handle_client::handle_client;
        use std::{io::ErrorKind, time::Duration};

        daemon
            .listener
            .set_nonblocking(true)
            .expect("Failed to set to non-blocking");

        // Executor thread
        let exe_daemon = Arc::clone(&daemon);
        std::thread::spawn(|| {
            let daemon = exe_daemon;

            loop {
                use request::Rt;
                use std::time::{Duration, Instant};

                if daemon.should_exit() {
                    return;
                }

                let mut query = { daemon.queries.lock().unwrap().pop_front() };

                let Some(query) = query.as_mut() else {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                };

                daemon.idle_tracker.still_active();
                let soon = Instant::now() + Duration::from_millis(50);
                let mut rt = daemon.rt.lock().unwrap();
                match rt.block_on(query, request::TimeoutAt(soon)) {
                    Ok(value) => {
                        (&query.then)(&query.connection, value);
                    }
                    Err(top_errors) => {
                        for error in top_errors.iter_unordered() {
                            println!("Got error: {:?}", error);
                        }
                    }
                }
            }
        });

        // Accept clients
        loop {
            match daemon.listener.accept() {
                Ok((stream, _address)) => {
                    let daemon = Arc::clone(&daemon);

                    std::thread::spawn(move || {
                        if daemon.idle_tracker.add_connection().is_ok() {
                            use connection::Connection;

                            if let Err(err) = stream.set_nonblocking(false).and_then(|_| {
                                stream.set_read_timeout(Some(Duration::from_millis(5)))
                            }) {
                                log::error!(
                                    "Client connection closed before able to setup - {}",
                                    err
                                );
                                daemon.idle_tracker.remove_connection();
                                return;
                            }

                            let desc = format!("{:?}", stream.local_addr());
                            let connection = Connection::new_unix(stream);
                            handle_client(daemon.as_ref(), connection, desc);
                            daemon.idle_tracker.remove_connection();
                        }
                    });
                }
                Err(error) => {
                    if !matches!(error.kind(), ErrorKind::WouldBlock) {
                        log::error!("Failed to accept client: {:?}", error);
                    }
                }
            }

            if daemon.should_exit() {
                return Ok(());
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[cfg(target_family = "windows")]
    {
        let _ = daemon;
        panic!("Windows is not supported")
    }
}
