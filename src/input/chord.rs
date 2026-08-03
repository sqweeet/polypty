use std::fmt;

use anyhow::{bail, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KeyChord {
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl KeyChord {
    pub(super) fn parse(value: &str) -> Result<Self> {
        let parts: Vec<_> = value.split('+').map(str::trim).collect();
        if parts.is_empty() || parts.iter().any(|part| part.is_empty()) {
            bail!("invalid key chord `{value}`; use the name `plus` for +");
        }
        let (key, modifiers) = parts.split_last().expect("parts are not empty");
        let mut flags = KeyModifiers::NONE;
        for modifier in modifiers {
            flags.insert(parse_modifier(modifier)?);
        }
        Ok(Self {
            code: parse_code(key)?,
            modifiers: flags,
        })
    }

    pub(super) fn matches(&self, event: KeyEvent) -> bool {
        self.code == normalize_code(event.code) && self.modifiers == event.modifiers
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (flag, label) in [
            (KeyModifiers::CONTROL, "Ctrl"),
            (KeyModifiers::ALT, "Alt"),
            (KeyModifiers::SHIFT, "Shift"),
            (KeyModifiers::SUPER, "Super"),
            (KeyModifiers::HYPER, "Hyper"),
            (KeyModifiers::META, "Meta"),
        ] {
            if self.modifiers.contains(flag) {
                write!(output, "{label}+")?;
            }
        }
        write!(output, "{}", code_label(self.code))
    }
}

fn parse_modifier(value: &str) -> Result<KeyModifiers> {
    match value.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Ok(KeyModifiers::CONTROL),
        "alt" | "option" => Ok(KeyModifiers::ALT),
        "shift" => Ok(KeyModifiers::SHIFT),
        "super" | "command" | "win" => Ok(KeyModifiers::SUPER),
        "hyper" => Ok(KeyModifiers::HYPER),
        "meta" => Ok(KeyModifiers::META),
        _ => bail!("unknown key modifier `{value}`"),
    }
}

fn parse_code(value: &str) -> Result<KeyCode> {
    let lower = value.to_ascii_lowercase();
    let named = match lower.as_str() {
        "backspace" => Some(KeyCode::Backspace),
        "enter" | "return" => Some(KeyCode::Enter),
        "left" => Some(KeyCode::Left),
        "right" => Some(KeyCode::Right),
        "up" => Some(KeyCode::Up),
        "down" => Some(KeyCode::Down),
        "home" => Some(KeyCode::Home),
        "end" => Some(KeyCode::End),
        "pageup" | "page-up" => Some(KeyCode::PageUp),
        "pagedown" | "page-down" => Some(KeyCode::PageDown),
        "tab" => Some(KeyCode::Tab),
        "backtab" | "back-tab" => Some(KeyCode::BackTab),
        "delete" => Some(KeyCode::Delete),
        "insert" => Some(KeyCode::Insert),
        "esc" | "escape" => Some(KeyCode::Esc),
        "space" => Some(KeyCode::Char(' ')),
        "plus" => Some(KeyCode::Char('+')),
        "minus" => Some(KeyCode::Char('-')),
        "equals" | "equal" => Some(KeyCode::Char('=')),
        _ => None,
    };
    if let Some(code) = named {
        return Ok(code);
    }
    if let Some(number) = lower.strip_prefix('f').and_then(|tail| tail.parse().ok()) {
        if (1..=24).contains(&number) {
            return Ok(KeyCode::F(number));
        }
    }
    let mut chars = value.chars();
    if let (Some(character), None) = (chars.next(), chars.next()) {
        return Ok(KeyCode::Char(lower_char(character)));
    }
    bail!("unknown key `{value}`")
}

fn normalize_code(code: KeyCode) -> KeyCode {
    match code {
        KeyCode::Char(character) => KeyCode::Char(lower_char(character)),
        other => other,
    }
}

fn lower_char(character: char) -> char {
    character.to_lowercase().next().unwrap_or(character)
}

fn code_label(code: KeyCode) -> String {
    match code {
        KeyCode::Char(' ') => "Space".into(),
        KeyCode::Char(character) => character.to_string(),
        KeyCode::F(number) => format!("F{number}"),
        other => format!("{other:?}"),
    }
}
