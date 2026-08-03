use std::io::Write;

use anyhow::Result;
use crossterm::cursor::Hide;
use crossterm::queue;
use vt100::Screen;

use crate::core::geometry::TerminalRect;

use super::cache::TermCache;
use super::cell::PaintCell;
use super::cursor::{restore_terminal_cursor, terminal_cursor_state};
use super::frame::desired_frame;
use super::pen::PaintPen;

pub fn draw_terminal_rect(
    out: &mut impl Write,
    rect: TerminalRect,
    screen: &Screen,
    cache: &mut TermCache,
    force: bool,
    suppress_cursor: bool,
) -> Result<()> {
    let cols = rect.cols.max(1);
    let rows = rect.rows.max(1);
    if force {
        cache.invalidate();
    }
    cache.ensure(cols, rows);

    let frame = desired_frame(screen, cols, rows);
    let (cursor_x, cursor_y, cursor_hidden) = terminal_cursor_state(rect, screen, suppress_cursor);
    queue!(out, Hide)?;
    paint_changes(out, rect, cache, &frame)?;

    cache.cells = frame;
    cache.valid_cells.fill(true);
    cache.cursor = (cursor_x, cursor_y);
    cache.cursor_hidden = cursor_hidden;
    restore_terminal_cursor(out, rect, screen, suppress_cursor)
}

fn paint_changes(
    out: &mut impl Write,
    rect: TerminalRect,
    cache: &TermCache,
    frame: &[PaintCell],
) -> Result<()> {
    let mut pen = PaintPen::new();
    for row in 0..rect.rows.max(1) {
        let mut col = 0;
        while col < rect.cols.max(1) {
            let index = cache.idx(row, col);
            let cell = &frame[index];
            if cell.width == 0 {
                col += 1;
                continue;
            }

            let valid = cache.valid_cells.get(index).copied().unwrap_or(false);
            let changed = cache
                .cells
                .get(index)
                .map(|old| old != cell)
                .unwrap_or(true);
            if !valid || changed {
                pen.paint(
                    out,
                    rect.x.saturating_add(col),
                    rect.y.saturating_add(row),
                    cell,
                )?;
            }
            col = col.saturating_add(u16::from(cell.width));
        }
    }
    Ok(())
}
