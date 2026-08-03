use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::{input::Keymap, render::ShortcutScope};

use super::{App, EmptyClipboard, FakeFactory};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
fn scope_dialog_separates_session_and_persistent_changes() {
    let root = temporary_root("scope");
    let path = root.join("mux/config.toml");
    let mut app = app(Some(path.clone()));

    app.open_shortcut_dialog(false);
    click_scope(&mut app, ShortcutScope::Session);
    assert!(!app.shortcuts_visible);
    assert!(!path.exists());

    app.open_shortcut_dialog(true);
    click_scope(&mut app, ShortcutScope::Always);
    assert!(app.shortcuts_visible);
    assert!(!app.shortcut_dialog.visible());
    assert!(fs::read_to_string(&path)
        .unwrap()
        .contains("shortcuts = true"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_persistence_keeps_the_dialog_open() {
    let root = temporary_root("failure");
    fs::create_dir_all(&root).unwrap();
    let mut app = app(Some(root.clone()));

    app.open_shortcut_dialog(false);
    click_scope(&mut app, ShortcutScope::Always);
    let view = app.shortcut_dialog.view().unwrap();
    assert!(view.save_failed);
    assert!(app.shortcuts_visible);

    click_scope(&mut app, ShortcutScope::Session);
    assert!(!app.shortcuts_visible);
    assert!(!app.shortcut_dialog.visible());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn dragging_off_a_scope_button_cancels_confirmation() {
    let mut app = app(None);
    app.open_shortcut_dialog(false);
    let (column, row) = find_scope(&app, ShortcutScope::Session);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row))
        .unwrap();
    app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 0, 0))
        .unwrap();
    app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 0, 0))
        .unwrap();

    assert!(app.shortcuts_visible);
    assert!(app.shortcut_dialog.visible());
}

fn app(config_path: Option<PathBuf>) -> App {
    let factory = FakeFactory {
        spawns: Arc::new(Mutex::new(Vec::new())),
        kills: None,
    };
    App::with_components(
        100,
        30,
        Box::new(EmptyClipboard),
        Box::new(factory),
        Keymap::default(),
        (true, 18, true),
        config_path,
    )
    .unwrap()
}

fn click_scope(app: &mut App, target: ShortcutScope) {
    let (column, row) = find_scope(app, target);
    assert!(app
        .handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row,))
        .unwrap());
    assert!(app
        .handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), column, row,))
        .unwrap());
}

fn find_scope(app: &App, target: ShortcutScope) -> (u16, u16) {
    let layout = app.layout();
    for row in 0..layout.rows {
        for column in 0..layout.cols {
            if crate::render::shortcut_dialog_hit(layout, column, row) == Some(target) {
                return (column, row);
            }
        }
    }
    panic!("scope button is outside the dialog")
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn temporary_root(label: &str) -> PathBuf {
    let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "mux-dialog-test-{}-{label}-{sequence}",
        std::process::id()
    ))
}
