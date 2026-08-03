//! Mouse-to-terminal protocol encoding.

mod legacy;
mod protocol;

use crossterm::event::{KeyModifiers, MouseEvent};
use vt100::{MouseProtocolEncoding, MouseProtocolMode};

/// Encode a pane-local mouse event for the child TUI.
pub fn encode_mouse(
    event: MouseEvent,
    col: u16,
    row: u16,
    mode: MouseProtocolMode,
    encoding: MouseProtocolEncoding,
) -> Vec<u8> {
    let Some((mut code, release)) = protocol::event_code(event.kind, mode) else {
        return Vec::new();
    };
    if event.modifiers.contains(KeyModifiers::SHIFT) {
        code += 4;
    }
    if event.modifiers.contains(KeyModifiers::ALT) {
        code += 8;
    }
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        code += 16;
    }

    let x = u32::from(col) + 1;
    let y = u32::from(row) + 1;
    match encoding {
        MouseProtocolEncoding::Sgr => {
            let suffix = if release { 'm' } else { 'M' };
            format!("\x1b[<{code};{x};{y}{suffix}").into_bytes()
        }
        MouseProtocolEncoding::Default | MouseProtocolEncoding::Utf8 => {
            if release {
                code = 3 + (code & !0b11);
            }
            legacy::encode(code, x, y, encoding)
        }
    }
}
