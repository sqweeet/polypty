use std::ffi::OsStr;

use anyhow::{bail, Result};

#[cfg(unix)]
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawFd,
    },
    path::{Path, PathBuf},
};

#[cfg(unix)]
use anyhow::Context;

#[derive(Debug)]
pub(super) struct InstanceGuard {
    #[cfg(unix)]
    _lock: File,
}

impl InstanceGuard {
    pub(super) fn acquire() -> Result<Self> {
        let marker = std::env::var_os("MUX");
        reject_nested(marker.as_deref())?;

        #[cfg(unix)]
        return Self::acquire_at(&default_lock_path());

        #[cfg(not(unix))]
        Ok(Self {})
    }

    #[cfg(unix)]
    fn acquire_at(path: &Path) -> Result<Self> {
        let mut file = open_lock(path)?;
        if let Err(err) = try_lock(&file) {
            if err.kind() == std::io::ErrorKind::WouldBlock {
                let mut owner = String::new();
                let _ = (&file).read_to_string(&mut owner);
                if let Ok(pid) = owner.trim().parse::<u32>() {
                    bail!("another mux instance is already running (pid {pid})");
                }
                bail!("another mux instance is already running");
            }
            return Err(err).with_context(|| format!("lock mux instance at {}", path.display()));
        }
        file.set_len(0).context("clear mux instance lock")?;
        file.seek(SeekFrom::Start(0))
            .context("rewind mux instance lock")?;
        writeln!(file, "{}", std::process::id()).context("write mux instance owner")?;
        file.flush().context("flush mux instance owner")?;
        Ok(Self { _lock: file })
    }
}

fn reject_nested(marker: Option<&OsStr>) -> Result<()> {
    if marker.is_some() {
        bail!("refusing to start mux inside an existing mux session");
    }
    Ok(())
}

#[cfg(unix)]
fn open_lock(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
        .with_context(|| format!("open mux instance lock at {}", path.display()))?;
    if file.metadata()?.uid() != unsafe { libc::geteuid() } {
        bail!(
            "mux instance lock is owned by another user: {}",
            path.display()
        );
    }
    Ok(file)
}

#[cfg(unix)]
fn try_lock(file: &File) -> std::io::Result<()> {
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn default_lock_path() -> PathBuf {
    if let Some(path) = std::env::var_os("XDG_RUNTIME_DIR") {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            return path.join("mux.instance.lock");
        }
    }
    std::env::temp_dir().join(format!("mux-{}.lock", unsafe { libc::geteuid() }))
}

#[cfg(test)]
mod tests;
