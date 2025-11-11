mod fs_watcher;
mod idle;
mod lockfile;
mod serve;
mod server_main;
mod startup;

use crate::{
    server_main::server_main,
    startup::{spawn_daemon_process, try_become_daemon},
};
use std::{thread, time::Duration};

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--daemon".into()) {
        return try_become_daemon(|| server_main());
    }

    // Try connecting to existing instance
    if std::net::TcpStream::connect("127.0.0.1:6000").is_ok() {
        println!("Connected to existing daemon.");
        return Ok(());
    }

    println!("No daemon found, launching one...");
    spawn_daemon_process()?;

    for _ in 0..10 {
        if std::net::TcpStream::connect("127.0.0.1:6000").is_ok() {
            println!("Daemon started!");
            return Ok(());
        }
        thread::sleep(Duration::from_millis(200));
    }

    eprintln!("Failed to start daemon.");
    Ok(())
}
