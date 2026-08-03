mod store;

use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::input::Keymap;

pub(crate) use store::save_sidebar_shortcuts;

#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub keymap: Keymap,
    pub sidebar: SidebarConfig,
    pub shell: Option<String>,
    pub source_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarConfig {
    pub visible: bool,
    pub width: u16,
    pub shortcuts: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            keymap: Keymap::default(),
            sidebar: SidebarConfig {
                visible: true,
                width: 18,
                shortcuts: true,
            },
            shell: None,
            source_path: None,
        }
    }
}

impl Config {
    pub(crate) fn load() -> Result<Self> {
        let Some(source) = store::read()? else {
            return Ok(Self::default());
        };
        let mut config = match source.contents {
            Some(contents) => Self::parse(&contents)
                .with_context(|| format!("parse {}", source.path.display()))?,
            None => Self::default(),
        };
        config.source_path = Some(source.path);
        Ok(config)
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
                shortcuts: raw.sidebar.shortcuts,
            },
            shell: raw.shell,
            source_path: None,
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
    shortcuts: bool,
}

impl Default for SidebarFileConfig {
    fn default() -> Self {
        Self {
            visible: true,
            width: 18,
            shortcuts: true,
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

#[cfg(test)]
mod tests;
