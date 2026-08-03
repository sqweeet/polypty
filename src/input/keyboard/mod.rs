//! Keyboard-to-terminal sequence encoding.

mod character;
mod csi;
mod function;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use character::encode_character;
use csi::{arrow, backtab, modded, modded_letter};
use function::encode_function;

/// Encode a key event the way a terminal sends it to a child PTY.
pub fn encode_key(key: KeyEvent, app_cursor: bool, app_keypad: bool) -> Vec<u8> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let _ = app_keypad; // Reserved for numpad SS3 forms.

    match key.code {
        KeyCode::Char(c) => encode_character(c, ctrl, alt),
        KeyCode::Enter if alt => b"\x1b\r".to_vec(),
        KeyCode::Enter => b"\r".to_vec(),
        KeyCode::Tab if shift => backtab(ctrl, alt),
        KeyCode::Tab if alt => b"\x1b\t".to_vec(),
        KeyCode::Tab => b"\t".to_vec(),
        KeyCode::BackTab => backtab(ctrl, alt),
        KeyCode::Backspace if alt => b"\x1b\x7f".to_vec(),
        KeyCode::Backspace if ctrl => vec![0x08],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => b"\x1b".to_vec(),
        KeyCode::Delete => modded(b"3~", ctrl, alt, shift),
        KeyCode::Insert => modded(b"2~", ctrl, alt, shift),
        KeyCode::Home if app_cursor && csi::modifier(ctrl, alt, shift) == 1 => b"\x1bOH".to_vec(),
        KeyCode::Home => modded_letter(b'H', ctrl, alt, shift),
        KeyCode::End if app_cursor && csi::modifier(ctrl, alt, shift) == 1 => b"\x1bOF".to_vec(),
        KeyCode::End => modded_letter(b'F', ctrl, alt, shift),
        KeyCode::PageUp => modded(b"5~", ctrl, alt, shift),
        KeyCode::PageDown => modded(b"6~", ctrl, alt, shift),
        KeyCode::Up => arrow(b'A', ctrl, alt, shift, app_cursor),
        KeyCode::Down => arrow(b'B', ctrl, alt, shift, app_cursor),
        KeyCode::Right => arrow(b'C', ctrl, alt, shift, app_cursor),
        KeyCode::Left => arrow(b'D', ctrl, alt, shift, app_cursor),
        KeyCode::F(number) => encode_function(number, ctrl, alt, shift),
        KeyCode::Null => vec![0],
        _ => Vec::new(),
    }
}
