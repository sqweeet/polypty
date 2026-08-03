mod config;
mod defaults;

use anyhow::{bail, Context, Result};
use crossterm::event::KeyEvent;

use super::{chord::KeyChord, Action};
use config::{action_name, config_action};
use defaults::{default_label, default_map_key};

#[derive(Debug, Clone)]
struct ActionBindings {
    action: Action,
    chords: Vec<KeyChord>,
}

#[derive(Debug, Clone, Default)]
pub struct Keymap {
    overrides: Vec<ActionBindings>,
}

impl Keymap {
    pub(crate) fn configured(entries: Vec<(String, Vec<String>)>) -> Result<Self> {
        let mut overrides: Vec<ActionBindings> = Vec::new();
        for (name, values) in entries {
            let action =
                config_action(&name).with_context(|| format!("unknown action `{name}`"))?;
            if overrides.iter().any(|entry| entry.action == action) {
                bail!("action `{name}` is configured more than once");
            }
            let mut chords = Vec::new();
            for value in values {
                let chord = KeyChord::parse(&value)
                    .with_context(|| format!("invalid binding for `{name}`"))?;
                if let Some(entry) = overrides.iter().find(|entry| entry.chords.contains(&chord)) {
                    bail!(
                        "key `{chord}` is assigned to both `{}` and `{name}`",
                        action_name(entry.action)
                    );
                }
                if chords.contains(&chord) {
                    bail!("key `{chord}` is repeated for `{name}`");
                }
                chords.push(chord);
            }
            overrides.push(ActionBindings { action, chords });
        }
        Ok(Self { overrides })
    }

    pub fn map_key(&self, key: KeyEvent) -> Action {
        for binding in &self.overrides {
            if binding.chords.iter().any(|chord| chord.matches(key)) {
                return binding.action;
            }
        }
        let action = default_map_key(key);
        if self.overrides.iter().any(|entry| entry.action == action) {
            Action::Forward
        } else {
            action
        }
    }

    pub(crate) fn binding_label(&self, action: Action) -> Option<String> {
        self.overrides
            .iter()
            .find(|entry| entry.action == action)
            .map(|entry| entry.chords.first().map(ToString::to_string))
            .unwrap_or_else(|| default_label(action).map(str::to_string))
    }
}

#[cfg(test)]
pub fn map_key(key: KeyEvent) -> Action {
    Keymap::default().map_key(key)
}
