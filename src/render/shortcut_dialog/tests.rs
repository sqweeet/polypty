use super::*;

#[test]
fn scope_buttons_share_the_compact_dialog() {
    let layout = Layout::new(80, 24, true, 18);
    let geometry = geometry(layout);
    assert_eq!((geometry.width, geometry.height), (40, 6));
    assert_eq!(
        shortcut_dialog_hit(layout, geometry.session_x, geometry.button_y),
        Some(ShortcutScope::Session)
    );
    assert_eq!(
        shortcut_dialog_hit(layout, geometry.always_x, geometry.button_y),
        Some(ShortcutScope::Always)
    );
}

#[test]
fn dialog_explains_the_requested_change_and_failure() {
    let layout = Layout::new(80, 24, true, 18);
    let mut output = Vec::new();
    draw_shortcut_dialog(
        &mut output,
        layout,
        ShortcutDialogView {
            desired_visible: false,
            selected: ShortcutScope::Session,
            save_failed: true,
            opacity: 255,
            pressed: None,
            press_opacity: 0,
            session_opacity: 255,
            always_opacity: 0,
        },
    )
    .unwrap();
    let frame = String::from_utf8_lossy(&output);
    assert!(frame.contains("Hide shortcuts?"));
    assert!(frame.contains("Could not save config."));
    assert!(frame.contains("Session"));
    assert!(frame.contains("Always"));
}

#[test]
fn scope_selection_uses_intermediate_alpha() {
    crate::render::enable_color_passthrough();
    let mut output = Vec::new();
    draw_shortcut_dialog(
        &mut output,
        Layout::new(80, 24, true, 18),
        ShortcutDialogView {
            desired_visible: false,
            selected: ShortcutScope::Session,
            save_failed: false,
            opacity: 255,
            pressed: None,
            press_opacity: 0,
            session_opacity: 128,
            always_opacity: 0,
        },
    )
    .unwrap();

    assert!(String::from_utf8_lossy(&output).contains("48;2;58;58;58"));
}
