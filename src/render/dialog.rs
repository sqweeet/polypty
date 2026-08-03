mod painter;

use std::io::Write;

use anyhow::Result;

use super::Layout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExitDialogButton {
    Cancel,
    Exit,
}

#[derive(Clone, Copy)]
pub(super) struct DialogGeometry {
    pub(super) x: u16,
    pub(super) y: u16,
    pub(super) width: u16,
    pub(super) height: u16,
    pub(super) button_y: u16,
    pub(super) cancel_x: u16,
    pub(super) exit_x: u16,
}

pub(crate) fn draw_exit_dialog(
    out: &mut impl Write,
    layout: Layout,
    exit_selected: bool,
    opacity: u8,
    pressed: Option<ExitDialogButton>,
    press_opacity: u8,
    selection_opacity: (u8, u8),
) -> Result<()> {
    painter::draw(
        out,
        geometry(layout),
        exit_selected,
        opacity,
        pressed,
        press_opacity,
        selection_opacity,
    )
}

pub(crate) fn exit_dialog_hit(layout: Layout, column: u16, row: u16) -> Option<ExitDialogButton> {
    let geometry = geometry(layout);
    if row != geometry.button_y || geometry.width < 16 || geometry.height < 2 {
        return None;
    }
    if (geometry.cancel_x..geometry.cancel_x + 8).contains(&column) {
        Some(ExitDialogButton::Cancel)
    } else if (geometry.exit_x..geometry.exit_x + 6).contains(&column) {
        Some(ExitDialogButton::Exit)
    } else {
        None
    }
}

fn geometry(layout: Layout) -> DialogGeometry {
    let outer_padding = u16::from(layout.cols > 4) * 2;
    let width = layout.cols.saturating_sub(outer_padding).clamp(1, 40);
    let height = layout.rows.clamp(1, 6);
    let x = (layout.cols.saturating_sub(width)) / 2;
    let y = (layout.rows.saturating_sub(height)) / 2;
    let buttons_x = x + width.saturating_sub(16) / 2;
    DialogGeometry {
        x,
        y,
        width,
        height,
        button_y: y + height.saturating_sub(2),
        cancel_x: buttons_x,
        exit_x: buttons_x + 10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_map_matches_centered_buttons() {
        let layout = Layout::new(80, 24, true, 18);
        let geometry = geometry(layout);
        assert_eq!((geometry.width, geometry.height), (40, 6));
        assert_eq!(
            exit_dialog_hit(layout, geometry.cancel_x, geometry.button_y),
            Some(ExitDialogButton::Cancel)
        );
        assert_eq!(
            exit_dialog_hit(layout, geometry.exit_x, geometry.button_y),
            Some(ExitDialogButton::Exit)
        );
    }

    #[test]
    fn dialog_frame_contains_message_and_two_buttons() {
        let mut output = Vec::new();
        draw_exit_dialog(
            &mut output,
            Layout::new(80, 24, true, 18),
            false,
            255,
            None,
            0,
            (255, 0),
        )
        .unwrap();
        let frame = String::from_utf8_lossy(&output);
        assert!(frame.contains("Exit polypty?"));
        assert!(frame.contains("Cancel"));
        assert!(frame.contains("Exit"));
    }

    #[test]
    fn exit_button_uses_the_destructive_palette() {
        crate::render::enable_color_passthrough();
        let layout = Layout::new(80, 24, true, 18);
        let mut idle = Vec::new();
        draw_exit_dialog(&mut idle, layout, false, 255, None, 0, (255, 0)).unwrap();
        assert!(String::from_utf8_lossy(&idle).contains("38;2;220;105;105"));

        let mut selected = Vec::new();
        draw_exit_dialog(&mut selected, layout, true, 255, None, 0, (0, 255)).unwrap();
        assert!(String::from_utf8_lossy(&selected).contains("48;2;104;48;48"));
    }
}
