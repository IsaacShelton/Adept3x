mod idle;
mod lockfile;
mod serve;
mod server;
mod startup;
mod watch;

use crate::{
    server::server_main,
    startup::{spawn_daemon_process, try_become_daemon},
};
use std::{path::PathBuf, process::ExitCode, thread, time::Duration};

pub fn daemonize_main(path: PathBuf, max_idle_time: Duration) -> ExitCode {
    match start(path, max_idle_time) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{}", err);
            ExitCode::FAILURE
        }
    }
}

fn start(path: PathBuf, max_idle_time: Duration) -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--daemon".into()) {
        return try_become_daemon(&path.clone(), || server_main(max_idle_time));
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
