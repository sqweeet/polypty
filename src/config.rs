use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::input::Keymap;

#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub keymap: Keymap,
    pub sidebar: SidebarConfig,
    pub shell: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarConfig {
    pub visible: bool,
    pub width: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            keymap: Keymap::default(),
            sidebar: SidebarConfig {
                visible: true,
                width: 18,
            },
            shell: None,
        }
    }
}

impl Config {
    pub(crate) fn load() -> Result<Self> {
        let explicit = std::env::var_os("MUX_CONFIG").map(PathBuf::from);
        let Some(path) = explicit.clone().or_else(default_path) else {
            return Ok(Self::default());
        };
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound && explicit.is_none() => {
                return Ok(Self::default());
            }
            Err(err) => return Err(err).with_context(|| format!("read {}", path.display())),
        };
        Self::parse(&source).with_context(|| format!("parse {}", path.display()))
    }

    fn parse(source: &str) -> Result<Self> {
        let raw: FileConfig = toml::from_str(source).context("invalid TOML")?;
        if raw
            .shell
            .as_ref()
            .is_some_and(|shell| shell.trim().is_empty())
        {
            bail!("`shell` cannot be empty");
        }
        if raw.sidebar.width == 0 {
            bail!("`sidebar.width` must be greater than zero");
        }
        let entries = raw
            .bindings
            .into_iter()
            .map(|(name, value)| (name, value.into_vec()))
            .collect();
        Ok(Self {
            keymap: Keymap::configured(entries)?,
            sidebar: SidebarConfig {
                visible: raw.sidebar.visible,
                width: raw.sidebar.width,
            },
            shell: raw.shell,
        })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
struct FileConfig {
    shell: Option<String>,
    sidebar: SidebarFileConfig,
    bindings: BTreeMap<String, BindingValue>,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SidebarFileConfig {
    visible: bool,
    width: u16,
}

impl Default for SidebarFileConfig {
    fn default() -> Self {
        Self {
            visible: true,
            width: 18,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BindingValue {
    One(String),
    Many(Vec<String>),
}

impl BindingValue {
    fn into_vec(self) -> Vec<String> {
        match self {
            Self::One(value) => vec![value],
            Self::Many(values) => values,
        }
    }
}

fn default_path() -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("XDG_CONFIG_HOME") {
        let root = PathBuf::from(root);
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
