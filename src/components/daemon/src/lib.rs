mod error;
mod lockfile;
mod serve;
mod server;
mod startup;
mod watch;

pub use error::*;
pub use server::*;
use smol::{Timer, net::TcpStream};
pub use startup::*;
use std::time::Duration;

pub async fn connect_to_daemon() -> Result<TcpStream, StartError> {
    // Try connecting to existing instance
    if let Ok(connection) = TcpStream::connect("127.0.0.1:6000").await {
        println!("Connected to existing daemon.");
        return Ok(connection);
    }

    spawn_daemon_process()?;

    for _ in 0..10 {
        if let Ok(connection) = TcpStream::connect("127.0.0.1:6000").await {
            return Ok(connection);
        }
        Timer::after(Duration::from_millis(20)).await;
    }

    Err(StartError::FailedToStart)
}
