mod command;

use command::read_first;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardKind {
    Clipboard,
    Primary,
}

/// Replaceable clipboard boundary used by application input handling.
pub trait Clipboard {
    fn read(&self, kind: ClipboardKind) -> Option<String>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClipboard;

impl Clipboard for SystemClipboard {
    fn read(&self, kind: ClipboardKind) -> Option<String> {
        match kind {
            ClipboardKind::Clipboard => read_first(CLIPBOARD_COMMANDS),
            ClipboardKind::Primary => {
                read_first(PRIMARY_COMMANDS).or_else(|| read_first(CLIPBOARD_COMMANDS))
            }
        }
    }
}

const CLIPBOARD_COMMANDS: &[&[&str]] = &[
    &["wl-paste", "--no-newline"],
    &["xclip", "-selection", "clipboard", "-o"],
    &["xsel", "--clipboard", "--output"],
];

const PRIMARY_COMMANDS: &[&[&str]] = &[
    &["wl-paste", "--primary", "--no-newline"],
    &["xclip", "-selection", "primary", "-o"],
    &["xsel", "--primary", "--output"],
];
