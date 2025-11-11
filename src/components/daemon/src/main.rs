mod server_main;

use crate::server_main::server_main;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
use std::{
    fs::{File, OpenOptions, remove_file},
    path::PathBuf,
    process::Command,
    thread,
    time::Duration,
};
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::*;

fn lockfile_path() -> PathBuf {
    std::env::current_dir().unwrap().join("adeptd.lock")
}

fn try_acquire_lock() -> std::io::Result<Option<File>> {
    let path = lockfile_path();

    #[cfg(unix)]
    {
        use nix::{
            libc,
            libc::{F_SETLK, F_WRLCK, SEEK_SET, fcntl, flock as RawFlock},
        };

        let file = OpenOptions::new().create(true).write(true).open(&path)?;

        let fl = RawFlock {
            l_type: F_WRLCK as i16, // (exclusive lock)
            l_whence: SEEK_SET as i16,
            l_start: 0,
            l_len: 0, // (whole file)
            l_pid: 0,
        };

        // F_SETLK for a whole-file exclusive lock
        if unsafe { fcntl(file.as_raw_fd(), F_SETLK, &fl) } == -1 {
            let errno = unsafe { *libc::__error() };

            if errno == libc::EACCES || errno == libc::EAGAIN {
                return Ok(None);
            } else {
                return Err(std::io::Error::last_os_error());
            }
        }

        Ok(Some(file))
    }

    #[cfg(windows)]
    {
        use std::{ffi::OsStr, os::windows::ffi::OsStrExt};

        let wide: Vec<u16> = OsStr::new(&path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0, // no sharing
                std::ptr::null_mut(),
                OPEN_ALWAYS,
                FILE_ATTRIBUTE_NORMAL,
                0,
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            Ok(None)
        } else {
            Ok(Some(unsafe { File::from_raw_handle(handle as _) }))
        }
    }
}

fn start_daemon() -> std::io::Result<()> {
    let maybe_lock = try_acquire_lock()?;

    let Some(lockfile) = maybe_lock else {
        println!("Daemon already running.");
        return Ok(());
    };

    println!("Starting daemon...");
    server_main()?;

    println!("Daemon shutting down...");
    drop(lockfile);

    let _ = remove_file(&lockfile_path());
    Ok(())
}

fn spawn_daemon() -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    Command::new(exe).arg("--daemon").spawn()?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--daemon".into()) {
        return start_daemon();
    }

    // Try connecting to existing instance
    if std::net::TcpStream::connect("127.0.0.1:6000").is_ok() {
        println!("Connected to existing daemon.");
        return Ok(());
    }

    println!("No daemon found, launching one...");
    spawn_daemon()?;

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
