use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::render::SidebarMenuAction;

use super::{App, EmptyClipboard, FakeFactory};

fn app() -> App {
    let factory = FakeFactory {
        spawns: Arc::new(Mutex::new(Vec::new())),
        kills: None,
    };
    App::with_services(100, 30, Box::new(EmptyClipboard), Box::new(factory)).unwrap()
}

#[test]
fn context_menu_is_sidebar_only_and_new_tab_works() {
    let mut app = app();
    let terminal_column = app.layout().term_x + 1;
    assert!(!app
        .handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            terminal_column,
            2,
        ))
        .unwrap());
    assert!(!app.sidebar_menu.visible());

    assert!(app
        .handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 2, 2))
        .unwrap());
    assert!(app.sidebar_menu.visible());
    assert!(app
        .handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 2, 2))
        .unwrap());
    assert_eq!(app.book.len(), 1);
    assert!(app
        .handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 2, 2))
        .unwrap());
    assert_eq!(app.book.len(), 2);
    assert!(!app.sidebar_menu.visible());
}

#[test]
fn context_menu_opens_the_shortcut_scope_dialog() {
    let mut app = app();
    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 2, 2))
        .unwrap();
    assert!(app
        .handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 2, 3))
        .unwrap());
    assert!(app
        .handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 2, 3))
        .unwrap());
    assert!(app.shortcut_dialog.visible());
    assert!(app.shortcuts_visible);

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.shortcuts_visible);
    assert!(!app.sidebar_menu.visible());
    assert!(!app.shortcut_dialog.visible());

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 2, 2))
        .unwrap();
    let menu = app.sidebar_menu.view(app.shortcuts_visible).unwrap();
    assert!(!menu.shortcuts_visible);
}

#[test]
fn tab_context_menu_closes_the_clicked_tab() {
    let mut app = app();
    app.spawn_workspace().unwrap();
    let kept_id = app.book.get(1).unwrap().id();
    app.draw(&mut Vec::new()).unwrap();

    let (tab_x, tab_y) = find_tab(&app, 0);
    app.handle_mouse(mouse(
        MouseEventKind::Down(MouseButton::Right),
        tab_x,
        tab_y,
    ))
    .unwrap();
    let (close_x, close_y) = find_menu_action(&app, SidebarMenuAction::CloseTab);
    assert!(app
        .handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            close_x,
            close_y,
        ))
        .unwrap());
    assert_eq!(app.book.len(), 2);
    assert!(app
        .handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            close_x,
            close_y,
        ))
        .unwrap());

    assert_eq!(app.book.len(), 1);
    assert_eq!(app.book.get(0).unwrap().id(), kept_id);
}

fn find_tab(app: &App, target: usize) -> (u16, u16) {
    let layout = app.layout();
    for row in 0..layout.rows {
        for column in 0..layout.sidebar_width {
            if app.presenter.sidebar_tab_at(column, row) == Some(target) {
                return (column, row);
            }
        }
    }
    panic!("tab is outside the sidebar")
}

fn find_menu_action(app: &App, target: SidebarMenuAction) -> (u16, u16) {
    let layout = app.layout();
    let menu = app.sidebar_menu.view(app.shortcuts_visible).unwrap();
    for row in 0..layout.rows {
        for column in 0..layout.sidebar_width {
            if crate::render::sidebar_menu_hit(layout, menu, column, row) == Some(target) {
                return (column, row);
            }
        }
    }
    panic!("menu action is outside the sidebar")
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

mod drag;
