use std::io::Write;

use anyhow::Result;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::queue;
use crossterm::style::{Attribute, ResetColor, SetAttribute};
use vt100::Screen;

use crate::core::geometry::TerminalRect;

pub(super) fn terminal_cursor_state(
    rect: TerminalRect,
    screen: &Screen,
    suppress_cursor: bool,
) -> (u16, u16, bool) {
    let (row, col) = screen.cursor_position();
    let x = rect
        .x
        .saturating_add(col.min(rect.cols.max(1).saturating_sub(1)));
    let y = rect
        .y
        .saturating_add(row.min(rect.rows.max(1).saturating_sub(1)));
    (x, y, screen.hide_cursor() || suppress_cursor)
}

/// Restore the host cursor after a sidebar-only frame.
pub fn restore_terminal_cursor(
    out: &mut impl Write,
    rect: TerminalRect,
    screen: &Screen,
    suppress_cursor: bool,
) -> Result<()> {
    let (x, y, hidden) = terminal_cursor_state(rect, screen, suppress_cursor);
    queue!(out, ResetColor, SetAttribute(Attribute::Reset))?;
    if hidden {
        queue!(out, Hide)?;
    } else {
        queue!(out, MoveTo(x, y), Show)?;
    }
    Ok(())
}
