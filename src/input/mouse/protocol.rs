use crossterm::event::{MouseButton, MouseEventKind};
use vt100::MouseProtocolMode;

pub(super) fn event_code(kind: MouseEventKind, mode: MouseProtocolMode) -> Option<(u32, bool)> {
    if mode == MouseProtocolMode::None {
        return None;
    }
    match kind {
        MouseEventKind::Down(button) => Some((button_code(button), false)),
        MouseEventKind::Up(button) if mode != MouseProtocolMode::Press => {
            Some((button_code(button), true))
        }
        MouseEventKind::Drag(button)
            if matches!(
                mode,
                MouseProtocolMode::ButtonMotion | MouseProtocolMode::AnyMotion
            ) =>
        {
            Some((32 + button_code(button), false))
        }
        MouseEventKind::Moved if mode == MouseProtocolMode::AnyMotion => Some((35, false)),
        MouseEventKind::ScrollUp => Some((64, false)),
        MouseEventKind::ScrollDown => Some((65, false)),
        MouseEventKind::ScrollLeft => Some((66, false)),
        MouseEventKind::ScrollRight => Some((67, false)),
        _ => None,
    }
}

fn button_code(button: MouseButton) -> u32 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    }
}
