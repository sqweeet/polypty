use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use vt100::{MouseProtocolEncoding, MouseProtocolMode};

use super::super::encode_mouse;

fn event(kind: MouseEventKind, modifiers: KeyModifiers) -> MouseEvent {
    MouseEvent {
        kind,
        column: 99,
        row: 99,
        modifiers,
    }
}

#[test]
fn sgr_uses_pane_local_coordinates() {
    let down = event(MouseEventKind::Down(MouseButton::Left), KeyModifiers::NONE);
    let up = event(
        MouseEventKind::Up(MouseButton::Right),
        KeyModifiers::CONTROL,
    );
    assert_eq!(
        encode_mouse(
            down,
            4,
            2,
            MouseProtocolMode::PressRelease,
            MouseProtocolEncoding::Sgr
        ),
        b"\x1b[<0;5;3M"
    );
    assert_eq!(
        encode_mouse(
            up,
            4,
            2,
            MouseProtocolMode::PressRelease,
            MouseProtocolEncoding::Sgr
        ),
        b"\x1b[<18;5;3m"
    );
}

#[test]
fn mouse_mode_filters_release_and_motion() {
    let release = event(MouseEventKind::Up(MouseButton::Left), KeyModifiers::NONE);
    let moved = event(MouseEventKind::Moved, KeyModifiers::NONE);
    assert!(encode_mouse(
        release,
        0,
        0,
        MouseProtocolMode::Press,
        MouseProtocolEncoding::Sgr
    )
    .is_empty());
    assert!(encode_mouse(
        moved,
        0,
        0,
        MouseProtocolMode::ButtonMotion,
        MouseProtocolEncoding::Sgr
    )
    .is_empty());
    assert_eq!(
        encode_mouse(
            moved,
            0,
            0,
            MouseProtocolMode::AnyMotion,
            MouseProtocolEncoding::Sgr
        ),
        b"\x1b[<35;1;1M"
    );
}

#[test]
fn legacy_encoding_is_well_formed() {
    let down = event(MouseEventKind::Down(MouseButton::Left), KeyModifiers::NONE);
    assert_eq!(
        encode_mouse(
            down,
            0,
            0,
            MouseProtocolMode::Press,
            MouseProtocolEncoding::Default
        ),
        b"\x1b[M !!"
    );
}
