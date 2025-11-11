use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
};

pub fn lockfile_path() -> PathBuf {
    std::env::current_dir().unwrap().join("adeptd.lock")
}

pub fn try_acquire_lock() -> std::io::Result<Option<File>> {
    let path = lockfile_path();

    #[cfg(unix)]
    {
        use nix::{
            libc,
            libc::{F_SETLK, F_WRLCK, SEEK_SET, fcntl, flock as RawFlock},
        };
        use std::os::unix::io::AsRawFd;

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
        use std::{
            ffi::OsStr,
            os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::FromRawHandle},
        };
        use windows_sys::Win32::Storage::FileSystem::*;

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
