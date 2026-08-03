use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::{map_key, Action, Keymap};

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

#[test]
fn configured_binding_replaces_default_and_wins_conflicts() {
    let keymap = Keymap::configured(vec![("new-tab".into(), vec!["alt+w".into()])]).unwrap();
    let alt = KeyModifiers::ALT;

    assert_eq!(
        keymap.map_key(KeyEvent::new(KeyCode::Char('w'), alt)),
        Action::NewTab
    );
    assert_eq!(
        keymap.map_key(KeyEvent::new(KeyCode::Char('t'), alt)),
        Action::Forward
    );
}

#[test]
fn empty_binding_list_unbinds_an_action() {
    let keymap = Keymap::configured(vec![("close-pane".into(), Vec::new())]).unwrap();
    let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT);
    assert_eq!(keymap.map_key(key), Action::Forward);
    assert_eq!(keymap.binding_label(Action::ClosePane), None);
}

#[test]
fn named_keys_and_modifiers_are_supported() {
    let keymap =
        Keymap::configured(vec![("pane-right".into(), vec!["ctrl+alt+left".into()])]).unwrap();
    let key = KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL | KeyModifiers::ALT);
    assert_eq!(keymap.map_key(key), Action::PaneRight);
}
