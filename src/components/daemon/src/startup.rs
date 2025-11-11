use crate::lockfile::{lockfile_path, try_acquire_lock};
use smol::{fs::remove_file, process::Command};

pub fn try_become_daemon(main: impl FnOnce() -> std::io::Result<()>) -> std::io::Result<()> {
    let maybe_lock = try_acquire_lock()?;

    let Some(lockfile) = maybe_lock else {
        println!("Daemon already running.");
        return Ok(());
    };

    println!("Starting daemon...");
    (main)()?;

    println!("Daemon shutting down...");
    drop(lockfile);

    let _ = remove_file(&lockfile_path());
    Ok(())
}

pub fn spawn_daemon_process() -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    Command::new(exe).arg("--daemon").spawn()?;
    Ok(())
}
