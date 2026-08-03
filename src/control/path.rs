use std::path::PathBuf;

use anyhow::{bail, Result};

pub(crate) fn socket_path() -> Result<PathBuf> {
    if let Some(value) = std::env::var_os("MUX_SOCKET") {
        let path = PathBuf::from(value);
        if !path.is_absolute() {
            bail!("MUX_SOCKET must be an absolute path");
        }
        return Ok(path);
    }
    if let Some(value) = std::env::var_os("XDG_RUNTIME_DIR") {
        let path = PathBuf::from(value);
        if path.is_absolute() {
            return Ok(path.join("mux.control.sock"));
        }
    }
    Ok(std::env::temp_dir().join(format!("mux-{}.sock", user_id())))
}

#[cfg(unix)]
fn user_id() -> u32 {
    unsafe { libc::geteuid() }
}

#[cfg(not(unix))]
fn user_id() -> u32 {
    std::process::id()
}
