mod error;
mod idle;
mod lockfile;
mod serve;
mod server;
mod startup;
mod watch;

pub use error::*;
pub use server::*;
pub use startup::*;
use std::{net::TcpStream, thread, time::Duration};

pub fn connect_to_daemon() -> Result<TcpStream, StartError> {
    // Try connecting to existing instance
    if let Ok(connection) = std::net::TcpStream::connect("127.0.0.1:6000") {
        println!("Connected to existing daemon.");
        return Ok(connection);
    }

    spawn_daemon_process()?;

    for _ in 0..10 {
        if let Ok(connection) = std::net::TcpStream::connect("127.0.0.1:6000") {
            return Ok(connection);
        }
        thread::sleep(Duration::from_millis(20));
    }

    Err(StartError::FailedToStart)
}
