use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::{map_key, Action};

#[test]
fn mux_shortcuts_are_mapped() {
    let alt = KeyModifiers::ALT;
    let cases = [
        ('t', Action::NewTab),
        ('v', Action::SplitVertical),
        ('s', Action::SplitHorizontal),
        ('x', Action::ClosePane),
        ('h', Action::PaneLeft),
    ];
    for (key, expected) in cases {
        assert_eq!(map_key(KeyEvent::new(KeyCode::Char(key), alt)), expected);
    }
}

#[test]
fn russian_layout_uses_physical_bindings() {
    let alt = KeyModifiers::ALT;
    let cases = [
        ('й', Action::Quit),
        ('е', Action::NewTab),
        ('ц', Action::CloseTab),
        ('м', Action::SplitVertical),
    ];
    for (key, expected) in cases {
        assert_eq!(map_key(KeyEvent::new(KeyCode::Char(key), alt)), expected);
    }
}

#[test]
fn plain_character_is_forwarded() {
    let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    assert_eq!(map_key(key), Action::Forward);
}
