mod painter;

use std::io::Write;

use anyhow::Result;

use super::Layout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShortcutScope {
    Session,
    Always,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ShortcutDialogView {
    pub desired_visible: bool,
    pub selected: ShortcutScope,
    pub save_failed: bool,
    pub opacity: u8,
    pub pressed: Option<ShortcutScope>,
    pub press_opacity: u8,
    pub session_opacity: u8,
    pub always_opacity: u8,
}

#[derive(Clone, Copy)]
pub(super) struct ShortcutDialogGeometry {
    pub(super) x: u16,
    pub(super) y: u16,
    pub(super) width: u16,
    pub(super) height: u16,
    pub(super) button_y: u16,
    pub(super) session_x: u16,
    pub(super) always_x: u16,
}

pub(crate) fn draw_shortcut_dialog(
    out: &mut impl Write,
    layout: Layout,
    view: ShortcutDialogView,
) -> Result<()> {
    painter::draw(out, geometry(layout), view)
}

pub(crate) fn shortcut_dialog_hit(layout: Layout, column: u16, row: u16) -> Option<ShortcutScope> {
    let geometry = geometry(layout);
    if row != geometry.button_y || geometry.width < 19 || geometry.height < 2 {
        return None;
    }
    if (geometry.session_x..geometry.session_x + 9).contains(&column) {
        Some(ShortcutScope::Session)
    } else if (geometry.always_x..geometry.always_x + 8).contains(&column) {
        Some(ShortcutScope::Always)
    } else {
        None
    }
}

fn geometry(layout: Layout) -> ShortcutDialogGeometry {
    let outer_padding = u16::from(layout.cols > 4) * 2;
    let width = layout.cols.saturating_sub(outer_padding).clamp(1, 40);
    let height = layout.rows.clamp(1, 6);
    let x = (layout.cols.saturating_sub(width)) / 2;
    let y = (layout.rows.saturating_sub(height)) / 2;
    let buttons_x = x + width.saturating_sub(19) / 2;
    ShortcutDialogGeometry {
        x,
        y,
        width,
        height,
        button_y: y + height.saturating_sub(2),
        session_x: buttons_x,
        always_x: buttons_x + 11,
    }
}

#[cfg(test)]
mod tests;
