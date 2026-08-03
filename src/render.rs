use std::collections::BTreeMap;
use std::io::Write;

use anyhow::{Context, Result};
use crossterm::cursor::{Hide, MoveTo};
use crossterm::queue;
use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{Clear, ClearType};

mod sidebar;
mod terminal;

pub use sidebar::{draw_sidebar, SidebarCache, SidebarMap, SidebarTab};
pub use terminal::{draw_terminal_rect, restore_terminal_cursor, TermCache};

/// Geometry of the mux chrome + terminal pane.
#[derive(Debug, Clone, Copy)]
pub struct Layout {
    #[allow(dead_code)]
    pub cols: u16,
    pub rows: u16,
    pub sidebar_visible: bool,
    pub sidebar_width: u16,
    pub term_x: u16,
    pub term_y: u16,
    pub term_cols: u16,
    pub term_rows: u16,
}

impl Layout {
    pub fn new(cols: u16, rows: u16, sidebar_visible: bool, sidebar_width: u16) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);

        let sb = if sidebar_visible {
            // Keep a usable terminal pane; allow a wide sidebar otherwise.
            let max_sb = cols.saturating_sub(20);
            sidebar_width.min(max_sb).max(1).min(cols.saturating_sub(1))
        } else {
            0
        };

        // Flush against the terminal pane — no gutter column.
        let term_x = sb;
        let term_cols = cols.saturating_sub(term_x).max(1);
        let term_rows = rows.max(1);

        Self {
            cols,
            rows,
            sidebar_visible: sb > 0,
            sidebar_width: sb,
            term_x,
            term_y: 0,
            term_cols,
            term_rows,
        }
    }

    pub fn terminal_rect(&self) -> TerminalRect {
        TerminalRect {
            x: self.term_x,
            y: self.term_y,
            cols: self.term_cols,
            rows: self.term_rows,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalRect {
    pub x: u16,
    pub y: u16,
    pub cols: u16,
    pub rows: u16,
}

impl TerminalRect {
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x
            && y >= self.y
            && x < self.x.saturating_add(self.cols)
            && y < self.y.saturating_add(self.rows)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Divider {
    Vertical { x: u16, y: u16, len: u16 },
    Horizontal { x: u16, y: u16, len: u16 },
}

pub fn clear(out: &mut impl Write) -> Result<()> {
    queue!(
        out,
        ResetColor,
        SetAttribute(Attribute::Reset),
        Clear(ClearType::All),
        Hide,
        MoveTo(0, 0)
    )
    .context("clear")?;
    Ok(())
}

/// Preserve colors emitted by child terminals even when mux itself inherits
/// `NO_COLOR`. A terminal multiplexer must reproduce child SGR state; this is
/// process-wide crossterm configuration and should be enabled once at startup.
pub fn enable_color_passthrough() {
    crossterm::style::Colored::set_ansi_color_disabled(false);
}

/// Bracket a paint batch so supporting terminals apply it atomically.
/// Kills cursor/frame flicker while agents stream TUI updates.
pub fn begin_sync(out: &mut impl Write) -> Result<()> {
    // Synchronized output + host autowrap off. Printing the physical
    // bottom-right cell with autowrap enabled can scroll the whole frame up,
    // leaving one default-background row below the sidebar.
    out.write_all(b"\x1b[?2026h\x1b[?7l")
        .context("sync begin")?;
    Ok(())
}

pub fn end_sync(out: &mut impl Write) -> Result<()> {
    // Restore normal host autowrap before committing the frame.
    out.write_all(b"\x1b[?7h\x1b[?2026l").context("sync end")?;
    Ok(())
}

pub fn draw_dividers(out: &mut impl Write, dividers: &[Divider]) -> Result<()> {
    if dividers.is_empty() {
        return Ok(());
    }

    const UP: u8 = 1;
    const DOWN: u8 = 2;
    const LEFT: u8 = 4;
    const RIGHT: u8 = 8;

    // Build one connected line graph. Nested splits frequently terminate next
    // to a parent divider; rendering independent `│` and `─` glyphs leaves a
    // visible half-cell gap. Connection-aware tees/crosses reach every edge.
    let mut cells: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    for divider in dividers {
        match *divider {
            Divider::Vertical { x, y, len } => {
                for offset in 0..len {
                    *cells.entry((x, y.saturating_add(offset))).or_default() |= UP | DOWN;
                }
            }
            Divider::Horizontal { x, y, len } => {
                for offset in 0..len {
                    *cells.entry((x.saturating_add(offset), y)).or_default() |= LEFT | RIGHT;
                }
            }
        }
    }

    let positions: Vec<(u16, u16)> = cells.keys().copied().collect();
    for (x, y) in positions {
        let mut links = cells[&(x, y)];
        if x > 0 && cells.contains_key(&(x - 1, y)) {
            links |= LEFT;
        }
        if cells.contains_key(&(x.saturating_add(1), y)) {
            links |= RIGHT;
        }
        if y > 0 && cells.contains_key(&(x, y - 1)) {
            links |= UP;
        }
        if cells.contains_key(&(x, y.saturating_add(1))) {
            links |= DOWN;
        }
        cells.insert((x, y), links);
    }

    let fg = Color::Rgb {
        r: 76,
        g: 76,
        b: 76,
    };
    let bg = Color::Rgb {
        r: 21,
        g: 21,
        b: 21,
    };
    queue!(out, Hide, SetForegroundColor(fg), SetBackgroundColor(bg))?;
    for ((x, y), links) in cells {
        let glyph = match links {
            bits if bits == UP | DOWN | LEFT | RIGHT => '┼',
            bits if bits == UP | DOWN | RIGHT => '├',
            bits if bits == UP | DOWN | LEFT => '┤',
            bits if bits == LEFT | RIGHT | DOWN => '┬',
            bits if bits == LEFT | RIGHT | UP => '┴',
            bits if bits == DOWN | RIGHT => '┌',
            bits if bits == DOWN | LEFT => '┐',
            bits if bits == UP | RIGHT => '└',
            bits if bits == UP | LEFT => '┘',
            bits if bits & (LEFT | RIGHT) != 0 && bits & (UP | DOWN) == 0 => '─',
            bits if bits & (UP | DOWN) != 0 && bits & (LEFT | RIGHT) == 0 => '│',
            bits if bits & (LEFT | RIGHT) != 0 => '─',
            _ => '│',
        };
        queue!(out, MoveTo(x, y), Print(glyph))?;
    }
    queue!(out, ResetColor, SetAttribute(Attribute::Reset))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    #[test]
    fn nested_dividers_render_connected_tees() {
        let mut out = Vec::new();
        draw_dividers(
            &mut out,
            &[
                Divider::Vertical { x: 4, y: 0, len: 4 },
                Divider::Horizontal { x: 5, y: 2, len: 6 },
            ],
        )
        .unwrap();

        let mut host = vt100::Parser::new(4, 11, 0);
        host.process(&out);
        assert_eq!(host.screen().cell(2, 4).unwrap().contents(), "├");
        assert_eq!(host.screen().cell(2, 5).unwrap().contents(), "─");
        assert_eq!(host.screen().cell(2, 10).unwrap().contents(), "─");
        assert_eq!(host.screen().cell(0, 4).unwrap().contents(), "│");
    }

    #[test]
    fn frame_guard_prevents_bottom_row_scroll() {
        let mut out = Vec::new();
        begin_sync(&mut out).unwrap();
        end_sync(&mut out).unwrap();

        assert!(contains(&out, b"\x1b[?7l"));
        assert!(contains(&out, b"\x1b[?7h"));
        assert!(contains(&out, b"\x1b[?2026h"));
        assert!(contains(&out, b"\x1b[?2026l"));
    }
}
