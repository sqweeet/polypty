use crossterm::event::{MouseButton, MouseEventKind};

use super::{app, find_tab, mouse};

#[test]
fn dragging_off_a_menu_item_cancels_the_click() {
    let mut app = app();
    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 2, 2))
        .unwrap();
    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 2, 2))
        .unwrap();
    app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 2, 3))
        .unwrap();
    app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 2, 3))
        .unwrap();

    assert_eq!(app.book.len(), 1);
    assert!(app.sidebar_menu.visible());
    assert!(!app.sidebar_menu.is_pressed());
}

#[test]
fn tab_click_activates_on_release_and_dragging_out_cancels_it() {
    let mut app = app();
    app.spawn_workspace().unwrap();
    app.draw(&mut Vec::new()).unwrap();
    let (column, row) = find_tab(&app, 0);
    assert_eq!(app.book.active_index(), 1);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row))
        .unwrap();
    assert_eq!(app.book.active_index(), 1);
    app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 30, row))
        .unwrap();
    app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 30, row))
        .unwrap();
    assert_eq!(app.book.active_index(), 1);

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row))
        .unwrap();
    app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), column, row))
        .unwrap();
    assert_eq!(app.book.active_index(), 0);
}

#[test]
fn reopening_context_menu_repaints_the_previous_overlay() {
    let mut app = app();
    app.draw(&mut Vec::new()).unwrap();

    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 2, 2))
        .unwrap();
    app.draw(&mut Vec::new()).unwrap();
    app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 2, 12))
        .unwrap();

    let mut frame = Vec::new();
    app.draw(&mut frame).unwrap();
    assert!(
        String::from_utf8_lossy(&frame).contains("\x1b[3;1H"),
        "the old menu rows were not restored"
    );
}
