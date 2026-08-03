use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::input::Action;

use super::Config;

#[test]
fn empty_config_preserves_defaults() {
    let config = Config::parse("").unwrap();
    let key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::ALT);
    assert_eq!(config.keymap.map_key(key), Action::NewTab);
    assert!(config.sidebar.visible);
    assert_eq!(config.sidebar.width, 18);
    assert!(config.sidebar.shortcuts);
    assert_eq!(config.shell, None);
}

#[test]
fn config_overrides_bindings_sidebar_and_shell() {
    let config = Config::parse(
        r#"
shell = "/bin/fish"

[sidebar]
visible = false
width = 24
shortcuts = false

[bindings]
new-tab = "ctrl+n"
quit = ["ctrl+q", "f12"]
close-pane = []
"#,
    )
    .unwrap();
    let ctrl_n = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
    let old_new = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::ALT);
    assert_eq!(config.keymap.map_key(ctrl_n), Action::NewTab);
    assert_eq!(config.keymap.map_key(old_new), Action::Forward);
    assert!(!config.sidebar.visible);
    assert_eq!(config.sidebar.width, 24);
    assert!(!config.sidebar.shortcuts);
    assert_eq!(config.shell.as_deref(), Some("/bin/fish"));
}

#[test]
fn invalid_action_and_key_have_context() {
    let action = Config::parse("[bindings]\nnew-tba = 'alt+n'").unwrap_err();
    assert!(action.to_string().contains("new-tba"));

    let key = Config::parse("[bindings]\nnew-tab = 'alt+wat'").unwrap_err();
    assert!(format!("{key:#}").contains("unknown key `wat`"));
}

#[test]
fn duplicate_custom_chord_is_rejected() {
    let err = Config::parse("[bindings]\nnew-tab = 'ctrl+n'\nclose-tab = 'ctrl+n'").unwrap_err();
    assert!(err.to_string().contains("assigned to both"));
}
