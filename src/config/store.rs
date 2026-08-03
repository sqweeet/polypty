use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use toml_edit::{table, value, DocumentMut};

pub(super) struct ConfigSource {
    pub(super) path: PathBuf,
    pub(super) contents: Option<String>,
}

pub(super) fn read() -> Result<Option<ConfigSource>> {
    let explicit = std::env::var_os("MUX_CONFIG").map(PathBuf::from);
    let Some(path) = explicit.clone().or_else(default_path) else {
        return Ok(None);
    };
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => Some(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && explicit.is_none() => None,
        Err(error) => {
            return Err(error).with_context(|| format!("read {}", path.display()));
        }
    };
    Ok(Some(ConfigSource { path, contents }))
}

pub(crate) fn save_sidebar_shortcuts(path: &Path, visible: bool) -> Result<()> {
    let destination = writable_destination(path)?;
    let source = match fs::read_to_string(&destination) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(error).with_context(|| format!("read {}", destination.display()));
        }
    };
    let mut document = source
        .parse::<DocumentMut>()
        .with_context(|| format!("parse {}", destination.display()))?;
    if document.get("sidebar").is_none() {
        document["sidebar"] = table();
    }
    document["sidebar"]["shortcuts"] = value(visible);
    atomic_write(&destination, document.to_string().as_bytes())
}

fn writable_destination(path: &Path) -> Result<PathBuf> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            fs::canonicalize(path).with_context(|| format!("resolve {}", path.display()))
        }
        Ok(_) => Ok(path.to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path.to_owned()),
        Err(error) => Err(error).with_context(|| format!("inspect {}", path.display())),
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let permissions = fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions());
    for attempt in 0..100 {
        let temporary = parent.join(format!(".mux-config-{}-{attempt}.tmp", std::process::id()));
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error).context("create temporary config"),
        };
        let result = (|| -> Result<()> {
            file.write_all(contents).context("write temporary config")?;
            if let Some(permissions) = permissions.clone() {
                file.set_permissions(permissions)
                    .context("preserve config permissions")?;
            }
            file.sync_all().context("sync temporary config")?;
            drop(file);
            fs::rename(&temporary, path).with_context(|| format!("replace {}", path.display()))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        return result;
    }
    anyhow::bail!("could not allocate a temporary config file")
}

fn default_path() -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        if root.is_absolute() {
            return Some(root.join("mux/config.toml"));
        }
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config/mux/config.toml"))
}

#[cfg(test)]
mod tests;
