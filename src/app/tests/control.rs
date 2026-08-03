use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::control::{ControlRequest, ControlResponse};

use super::{App, EmptyClipboard, FakeFactory};

fn app(kills: Arc<AtomicUsize>) -> App {
    let factory = FakeFactory {
        spawns: Arc::new(Mutex::new(Vec::new())),
        kills: Some(kills),
    };
    App::with_services(100, 30, Box::new(EmptyClipboard), Box::new(factory)).unwrap()
}

#[test]
fn control_client_can_create_list_and_select_tabs() {
    let kills = Arc::new(AtomicUsize::new(0));
    let mut app = app(Arc::clone(&kills));
    let created = app.handle_control(ControlRequest::NewTab);
    assert!(matches!(
        created,
        ControlResponse::Tab { tab } if tab.index == 2 && tab.active
    ));

    let listed = app.handle_control(ControlRequest::ListTabs);
    assert!(matches!(
        listed,
        ControlResponse::Tabs { tabs }
            if tabs.len() == 2 && tabs[0].panes == vec![1] && tabs[1].panes == vec![2]
    ));
    let selected = app.handle_control(ControlRequest::SelectTab {
        target: "@1".into(),
    });
    assert!(matches!(
        selected,
        ControlResponse::Tab { tab } if tab.index == 1 && tab.active
    ));
}

#[test]
fn final_tab_waits_for_explicit_exit_confirmation() {
    let kills = Arc::new(AtomicUsize::new(0));
    let mut app = app(Arc::clone(&kills));
    let response = app.handle_control(ControlRequest::CloseTab {
        target: "active".into(),
    });
    assert!(matches!(response, ControlResponse::Ack { .. }));
    assert!(app.exit_dialog.visible());
    assert_eq!(kills.load(Ordering::Relaxed), 0);

    let right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert!(!app.handle_key(right).unwrap());
    assert!(app.handle_key(enter).unwrap());
    assert_eq!(kills.load(Ordering::Relaxed), 1);
}

#[test]
fn escape_cancels_exit_without_killing_the_session() {
    let kills = Arc::new(AtomicUsize::new(0));
    let mut app = app(Arc::clone(&kills));
    app.request_exit();
    let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert!(!app.handle_key(escape).unwrap());
    assert!(!app.exit_dialog.visible());
    assert_eq!(kills.load(Ordering::Relaxed), 0);
}

#[test]
fn dialog_hover_selects_and_click_activates_buttons() {
    use crate::render::ExitDialogButton;

    let kills = Arc::new(AtomicUsize::new(0));
    let mut app = app(Arc::clone(&kills));
    app.request_exit();
    let layout = app.layout();
    let (exit_x, row) = find_button(layout, ExitDialogButton::Exit);
    let hover = mouse(MouseEventKind::Moved, exit_x, row);
    assert!(app.handle_mouse(hover).unwrap());
    assert!(app.exit_dialog.exit_selected());

    let (cancel_x, row) = find_button(layout, ExitDialogButton::Cancel);
    let cancel = mouse(MouseEventKind::Down(MouseButton::Left), cancel_x, row);
    assert!(app.handle_mouse(cancel).unwrap());
    assert!(app.exit_dialog.visible());
    let cancel = mouse(MouseEventKind::Up(MouseButton::Left), cancel_x, row);
    assert!(app.handle_mouse(cancel).unwrap());
    assert!(!app.exit_dialog.visible());
    assert_eq!(kills.load(Ordering::Relaxed), 0);

    app.request_exit();
    let exit = mouse(MouseEventKind::Down(MouseButton::Left), exit_x, row);
    assert!(app.handle_mouse(exit).unwrap());
    assert_eq!(kills.load(Ordering::Relaxed), 0);
    let exit = mouse(MouseEventKind::Up(MouseButton::Left), exit_x, row);
    assert!(!app.handle_mouse(exit).unwrap());
    assert_eq!(kills.load(Ordering::Relaxed), 1);
}

fn find_button(
    layout: crate::render::Layout,
    target: crate::render::ExitDialogButton,
) -> (u16, u16) {
    for row in 0..layout.rows {
        for column in 0..layout.cols {
            if crate::render::exit_dialog_hit(layout, column, row) == Some(target) {
                return (column, row);
            }
        }
    }
    panic!("dialog button is outside the layout")
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}
