use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::encode_key;

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn characters_and_cursor_keys_are_encoded() {
    assert_eq!(
        encode_key(key(KeyCode::Char('a'), KeyModifiers::NONE), false, false),
        b"a"
    );
    assert_eq!(
        encode_key(key(KeyCode::Char('c'), KeyModifiers::CONTROL), false, false),
        vec![3]
    );
    assert_eq!(
        encode_key(key(KeyCode::Up, KeyModifiers::NONE), true, false),
        b"\x1bOA"
    );
    assert_eq!(
        encode_key(key(KeyCode::Up, KeyModifiers::NONE), false, false),
        b"\x1b[A"
    );
}

#[test]
fn modified_unicode_is_not_truncated() {
    let event = key(
        KeyCode::Char('ж'),
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    );
    let mut expected = vec![0x1b];
    expected.extend_from_slice("ж".as_bytes());
    assert_eq!(encode_key(event, false, false), expected);
}

#[test]
fn both_backtab_forms_are_encoded() {
    let shifted = KeyModifiers::SHIFT;
    assert_eq!(
        encode_key(key(KeyCode::BackTab, shifted), false, false),
        b"\x1b[Z"
    );
    assert_eq!(
        encode_key(key(KeyCode::Tab, shifted), false, false),
        b"\x1b[Z"
    );
    let modified = KeyModifiers::CONTROL | KeyModifiers::SHIFT;
    assert_eq!(
        encode_key(key(KeyCode::BackTab, modified), false, false),
        b"\x1b[1;6Z"
    );
}

#[test]
fn function_keys_use_xterm_sequences() {
    assert_eq!(
        encode_key(key(KeyCode::F(1), KeyModifiers::NONE), false, false),
        b"\x1bOP"
    );
    assert_eq!(
        encode_key(key(KeyCode::F(4), KeyModifiers::NONE), false, false),
        b"\x1bOS"
    );
    assert_eq!(
        encode_key(key(KeyCode::F(1), KeyModifiers::CONTROL), false, false),
        b"\x1b[1;5P"
    );
    let all = KeyModifiers::SHIFT | KeyModifiers::ALT | KeyModifiers::CONTROL;
    assert_eq!(
        encode_key(key(KeyCode::F(12), all), false, false),
        b"\x1b[24;8~"
    );
}
