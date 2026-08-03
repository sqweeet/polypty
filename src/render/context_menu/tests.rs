use super::*;

fn menu() -> SidebarMenuView {
    SidebarMenuView {
        anchor_column: 15,
        anchor_row: 23,
        selected: SidebarMenuAction::NewTab,
        shortcuts_visible: true,
        can_close: false,
        opacity: 255,
        pressed: None,
        press_opacity: 0,
    }
}

#[test]
fn menu_stays_in_sidebar_and_maps_both_rows() {
    let layout = Layout::new(80, 24, true, 18);
    let geometry = geometry(layout, menu()).unwrap();
    assert_eq!(geometry.x + geometry.width, layout.sidebar_width);
    assert_eq!(geometry.y + geometry.height, layout.rows);
    assert_eq!(
        sidebar_menu_hit(layout, menu(), geometry.x, geometry.y),
        Some(SidebarMenuAction::NewTab)
    );
    assert_eq!(
        sidebar_menu_hit(layout, menu(), geometry.x, geometry.y + 1),
        Some(SidebarMenuAction::ToggleShortcuts)
    );
    assert_eq!(sidebar_menu_hit(layout, menu(), layout.term_x, 23), None);
}

#[test]
fn menu_uses_runtime_shortcut_label() {
    let layout = Layout::new(80, 24, true, 18);
    let mut output = Vec::new();
    draw_sidebar_menu(&mut output, layout, menu()).unwrap();
    assert!(String::from_utf8_lossy(&output).contains("Hide shortcuts"));
}

#[test]
fn tab_menu_inserts_close_between_create_and_shortcuts() {
    let layout = Layout::new(80, 24, true, 18);
    let tab_menu = SidebarMenuView {
        can_close: true,
        ..menu()
    };
    let geometry = geometry(layout, tab_menu).unwrap();
    assert_eq!(geometry.height, 3);
    assert_eq!(
        sidebar_menu_hit(layout, tab_menu, geometry.x, geometry.y + 1),
        Some(SidebarMenuAction::CloseTab)
    );
    assert_eq!(
        sidebar_menu_hit(layout, tab_menu, geometry.x, geometry.y + 2),
        Some(SidebarMenuAction::ToggleShortcuts)
    );
}

#[test]
fn opacity_and_press_state_blend_the_menu_palette() {
    crate::render::enable_color_passthrough();
    let layout = Layout::new(80, 24, true, 18);
    let mut faded = Vec::new();
    draw_sidebar_menu(
        &mut faded,
        layout,
        SidebarMenuView {
            opacity: 0,
            ..menu()
        },
    )
    .unwrap();
    assert!(String::from_utf8_lossy(&faded).contains("48;2;36;36;36"));

    let mut pressed = Vec::new();
    draw_sidebar_menu(
        &mut pressed,
        layout,
        SidebarMenuView {
            opacity: 255,
            pressed: Some(SidebarMenuAction::NewTab),
            press_opacity: 255,
            ..menu()
        },
    )
    .unwrap();
    assert!(String::from_utf8_lossy(&pressed).contains("48;2;88;88;88"));
}
