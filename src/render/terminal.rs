use std::io::Write;

use anyhow::Result;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::queue;
use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use vt100::{Cell, Screen};

use super::TerminalRect;
#[cfg(test)]
use super::{begin_sync, enable_color_passthrough, end_sync, Layout};

/// Last painted terminal pane — used for cell-level diffs so we don't
/// rewrite the whole screen (and flicker the cursor) on every agent tick.
#[derive(Debug, Clone)]
pub struct TermCache {
    cols: u16,
    rows: u16,
    cells: Vec<PaintCell>,
    cursor: (u16, u16),
    cursor_hidden: bool,
    /// Per-cell validity lets geometry changes preserve the overlapping grid
    /// while newly exposed rows/columns are repainted.
    valid_cells: Vec<bool>,
}

impl Default for TermCache {
    fn default() -> Self {
        Self {
            cols: 0,
            rows: 0,
            cells: Vec::new(),
            cursor: (0, 0),
            cursor_hidden: true,
            valid_cells: Vec::new(),
        }
    }
}

impl TermCache {
    pub fn invalidate(&mut self) {
        self.valid_cells.fill(false);
    }

    /// Establish the known screen state after a reset-color + full clear.
    pub fn reset_blank(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
        let len = (cols as usize) * (rows as usize);
        self.cells = vec![PaintCell::blank(); len];
        self.valid_cells = vec![true; len];
    }

    fn ensure(&mut self, cols: u16, rows: u16) {
        let len = (cols as usize) * (rows as usize);
        if self.cols == cols
            && self.rows == rows
            && self.cells.len() == len
            && self.valid_cells.len() == len
        {
            return;
        }

        let old_cols = self.cols;
        let old_rows = self.rows;
        let old_cells = std::mem::take(&mut self.cells);
        let old_valid = std::mem::take(&mut self.valid_cells);

        self.cols = cols;
        self.rows = rows;
        self.cells = vec![PaintCell::blank(); len];
        self.valid_cells = vec![false; len];

        // Alternate-screen terminal grids are preserved from the top-left by
        // Alacritty during resize. Copy that intersection so a one-column or
        // one-row resize only paints the newly exposed cells.
        let copy_cols = old_cols.min(cols);
        let copy_rows = old_rows.min(rows);
        for row in 0..copy_rows {
            for col in 0..copy_cols {
                let old_idx = (row as usize) * (old_cols as usize) + col as usize;
                let new_idx = (row as usize) * (cols as usize) + col as usize;
                if let Some(cell) = old_cells.get(old_idx) {
                    self.cells[new_idx] = cell.clone();
                    self.valid_cells[new_idx] = old_valid.get(old_idx).copied().unwrap_or(false);
                }
            }
        }
    }

    fn idx(&self, row: u16, col: u16) -> usize {
        (row as usize) * (self.cols as usize) + (col as usize)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PaintCell {
    /// Displayed grapheme (usually 1 char; may be empty for wide-cont).
    text: String,
    /// Columns occupied (0 = skip/continuation, 1 or 2 normally).
    width: u8,
    fg: PackedColor,
    bg: PackedColor,
    attrs: u8,
}

impl PaintCell {
    fn blank() -> Self {
        Self {
            text: " ".into(),
            width: 1,
            fg: PackedColor::DEFAULT,
            bg: PackedColor::DEFAULT,
            attrs: 0,
        }
    }
}

const ATTR_BOLD: u8 = 1;
const ATTR_ITALIC: u8 = 2;
const ATTR_UNDERLINE: u8 = 4;
const ATTR_INVERSE: u8 = 8;
const ATTR_DIM: u8 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PackedColor(u32);

impl PackedColor {
    const DEFAULT: Self = Self(0xFFFF_FFFF);

    fn from_vt(c: vt100::Color) -> Self {
        match c {
            vt100::Color::Default => Self::DEFAULT,
            vt100::Color::Idx(i) => Self(0x0100_0000 | (i as u32)),
            vt100::Color::Rgb(r, g, b) => {
                Self(0x0200_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
            }
        }
    }

    fn to_crossterm(self) -> Option<Color> {
        if self == Self::DEFAULT {
            return None;
        }
        let tag = (self.0 >> 24) & 0xff;
        match tag {
            1 => Some(Color::AnsiValue((self.0 & 0xff) as u8)),
            2 => {
                let r = ((self.0 >> 16) & 0xff) as u8;
                let g = ((self.0 >> 8) & 0xff) as u8;
                let b = (self.0 & 0xff) as u8;
                Some(Color::Rgb { r, g, b })
            }
            _ => None,
        }
    }
}

/// Diff-render the VT100 screen into the terminal pane.
///
/// Only cells that changed since `cache` are written. The whole batch is
/// wrapped in synchronized output so the host terminal never shows a
/// half-drawn frame or a cursor jump mid-paint.
#[cfg(test)]
fn draw_terminal(
    out: &mut impl Write,
    layout: &Layout,
    screen: &Screen,
    cache: &mut TermCache,
    force: bool,
    suppress_cursor: bool,
) -> Result<()> {
    draw_terminal_rect(
        out,
        layout.terminal_rect(),
        screen,
        cache,
        force,
        suppress_cursor,
    )
}

fn terminal_cursor_state(
    rect: TerminalRect,
    screen: &Screen,
    suppress_cursor: bool,
) -> (u16, u16, bool) {
    let (cur_row, cur_col) = screen.cursor_position();
    let cx = rect
        .x
        .saturating_add(cur_col.min(rect.cols.max(1).saturating_sub(1)));
    let cy = rect
        .y
        .saturating_add(cur_row.min(rect.rows.max(1).saturating_sub(1)));
    (cx, cy, screen.hide_cursor() || suppress_cursor)
}

/// Restore the one real host cursor after a sidebar-only frame without
/// rebuilding the active pane's entire terminal grid.
pub fn restore_terminal_cursor(
    out: &mut impl Write,
    rect: TerminalRect,
    screen: &Screen,
    suppress_cursor: bool,
) -> Result<()> {
    let (cx, cy, cursor_hidden) = terminal_cursor_state(rect, screen, suppress_cursor);
    queue!(out, ResetColor, SetAttribute(Attribute::Reset))?;
    if cursor_hidden {
        queue!(out, Hide)?;
    } else {
        queue!(out, MoveTo(cx, cy), Show)?;
    }
    Ok(())
}

pub fn draw_terminal_rect(
    out: &mut impl Write,
    rect: TerminalRect,
    screen: &Screen,
    cache: &mut TermCache,
    force: bool,
    suppress_cursor: bool,
) -> Result<()> {
    let origin_x = rect.x;
    let origin_y = rect.y;
    let view_cols = rect.cols.max(1);
    let view_rows = rect.rows.max(1);

    if force {
        cache.invalidate();
    }
    cache.ensure(view_cols, view_rows);

    let (scr_rows, scr_cols) = screen.size();

    // Build desired frame into a scratch buffer first.
    let mut next = vec![PaintCell::blank(); (view_cols as usize) * (view_rows as usize)];
    for row in 0..view_rows {
        let mut col: u16 = 0;
        while col < view_cols {
            let idx = (row as usize) * (view_cols as usize) + (col as usize);

            if row >= scr_rows || col >= scr_cols {
                next[idx] = PaintCell::blank();
                col += 1;
                continue;
            }

            let cell = screen.cell(row, col);
            let painted = cell_to_paint(cell);

            if painted.width == 0 {
                // Wide continuation — leave a blank marker so diffs stay aligned.
                next[idx] = PaintCell {
                    text: String::new(),
                    width: 0,
                    fg: PackedColor::DEFAULT,
                    bg: PackedColor::DEFAULT,
                    attrs: 0,
                };
                col += 1;
                continue;
            }

            if col as usize + painted.width as usize > view_cols as usize {
                // Clip overflow to blanks.
                for c in col..view_cols {
                    let i = (row as usize) * (view_cols as usize) + (c as usize);
                    next[i] = PaintCell::blank();
                }
                break;
            }

            next[idx] = painted.clone();
            // Mark continuation columns so we don't double-print.
            for k in 1..painted.width {
                let i = (row as usize) * (view_cols as usize) + (col as usize + k as usize);
                next[i] = PaintCell {
                    text: String::new(),
                    width: 0,
                    fg: painted.fg,
                    bg: painted.bg,
                    attrs: painted.attrs,
                };
            }
            col += painted.width as u16;
        }
    }

    let (cx, cy, cursor_hidden) = terminal_cursor_state(rect, screen, suppress_cursor);
    // A TUI frame can arrive over several PTY reads. During that short burst,
    // don't expose transient cursor positions (often the bottom-right corner).

    // Caller owns the outer sync frame. Keep cursor hidden during paint.
    queue!(out, Hide)?;

    // Style state machine — avoid Reset on every cell.
    let mut last_fg = PackedColor(0xDEAD_BEEF);
    let mut last_bg = PackedColor(0xDEAD_BEEF);
    let mut last_attrs: u8 = 0xFF;
    let mut pen_x: i32 = -1;
    let mut pen_y: i32 = -1;

    for row in 0..view_rows {
        let mut col: u16 = 0;
        while col < view_cols {
            let idx = cache.idx(row, col);
            let cell = &next[idx];

            if cell.width == 0 {
                col += 1;
                continue;
            }

            let changed = !cache.valid_cells.get(idx).copied().unwrap_or(false)
                || cache.cells.get(idx).map(|c| c != cell).unwrap_or(true);
            if !changed {
                col = col.saturating_add(cell.width as u16);
                continue;
            }

            let abs_x = origin_x + col;
            let abs_y = origin_y + row;

            // Move only when pen is not already here (sequential writes).
            if pen_x != abs_x as i32 || pen_y != abs_y as i32 {
                queue!(out, MoveTo(abs_x, abs_y))?;
            }

            // Apply style only on change.
            if cell.fg != last_fg || cell.bg != last_bg || cell.attrs != last_attrs {
                queue!(out, ResetColor, SetAttribute(Attribute::Reset))?;

                if let Some(c) = cell.fg.to_crossterm() {
                    queue!(out, SetForegroundColor(c))?;
                }
                if let Some(c) = cell.bg.to_crossterm() {
                    queue!(out, SetBackgroundColor(c))?;
                }
                if cell.attrs & ATTR_BOLD != 0 {
                    queue!(out, SetAttribute(Attribute::Bold))?;
                }
                if cell.attrs & ATTR_DIM != 0 {
                    queue!(out, SetAttribute(Attribute::Dim))?;
                }
                if cell.attrs & ATTR_ITALIC != 0 {
                    queue!(out, SetAttribute(Attribute::Italic))?;
                }
                if cell.attrs & ATTR_UNDERLINE != 0 {
                    queue!(out, SetAttribute(Attribute::Underlined))?;
                }
                // Preserve reverse as an attribute instead of swapping the
                // packed colors ourselves. Default foreground/background do
                // not have concrete RGB values to swap, and combining both a
                // manual swap and SGR 7 would double-invert explicit colors.
                if cell.attrs & ATTR_INVERSE != 0 {
                    queue!(out, SetAttribute(Attribute::Reverse))?;
                }
                last_fg = cell.fg;
                last_bg = cell.bg;
                last_attrs = cell.attrs;
            }

            let text = if cell.text.is_empty() {
                " "
            } else {
                cell.text.as_str()
            };
            queue!(out, Print(text))?;

            pen_x = abs_x as i32 + cell.width as i32;
            pen_y = abs_y as i32;
            col = col.saturating_add(cell.width as u16);
        }
    }

    // Commit cache.
    cache.cells = next;
    cache.valid_cells.fill(true);
    cache.cursor = (cx, cy);
    cache.cursor_hidden = cursor_hidden;

    // Place cursor once, at the end — never toggled mid-frame.
    restore_terminal_cursor(out, rect, screen, suppress_cursor)
}

fn cell_to_paint(cell: Option<&Cell>) -> PaintCell {
    let Some(cell) = cell else {
        return PaintCell::blank();
    };

    if cell.is_wide_continuation() {
        return PaintCell {
            text: String::new(),
            width: 0,
            fg: PackedColor::DEFAULT,
            bg: PackedColor::DEFAULT,
            attrs: 0,
        };
    }

    let contents = cell.contents();
    let text = if contents.is_empty() {
        " ".to_string()
    } else {
        contents.to_owned()
    };

    // `Screen` has already assigned this text to terminal cells using its
    // Unicode tables. Recomputing the width duplicates that decision and can
    // drift after dependency updates. Trust the authoritative cell geometry
    // so the following cell is never mistaken for a wide continuation.
    // Combining marks live in the same one-column Cell.
    let width = if cell.is_wide() { 2 } else { 1 };

    let mut attrs = 0u8;
    if cell.bold() {
        attrs |= ATTR_BOLD;
    }
    if cell.dim() {
        attrs |= ATTR_DIM;
    }
    if cell.italic() {
        attrs |= ATTR_ITALIC;
    }
    if cell.underline() {
        attrs |= ATTR_UNDERLINE;
    }
    if cell.inverse() {
        attrs |= ATTR_INVERSE;
    }

    PaintCell {
        text,
        width,
        fg: PackedColor::from_vt(cell.fgcolor()),
        bg: PackedColor::from_vt(cell.bgcolor()),
        attrs,
    }
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
    fn terminal_cache_preserves_only_the_resize_intersection() {
        let mut cache = TermCache::default();
        cache.ensure(3, 2);
        cache.valid_cells.fill(true);
        let marked = cache.idx(1, 2);
        cache.cells[marked].text = "x".into();

        cache.ensure(5, 3);
        assert_eq!(cache.cells[cache.idx(1, 2)].text, "x");
        assert!(cache.valid_cells[cache.idx(1, 2)]);
        assert!(!cache.valid_cells[cache.idx(0, 3)]);
        assert!(!cache.valid_cells[cache.idx(2, 0)]);

        cache.ensure(2, 1);
        assert_eq!(cache.cells.len(), 2);
        assert!(cache.valid_cells.iter().all(|valid| *valid));
    }

    fn render_frame(
        layout: &Layout,
        screen: &Screen,
        cache: &mut TermCache,
        force: bool,
    ) -> Vec<u8> {
        enable_color_passthrough();
        let mut out = Vec::new();
        begin_sync(&mut out).unwrap();
        draw_terminal(&mut out, layout, screen, cache, force, false).unwrap();
        end_sync(&mut out).unwrap();
        out
    }

    fn assert_same_cells(left: &Screen, right: &Screen, rows: u16, cols: u16) {
        for row in 0..rows {
            for col in 0..cols {
                assert_eq!(
                    cell_to_paint(left.cell(row, col)),
                    cell_to_paint(right.cell(row, col)),
                    "cell mismatch at ({row}, {col})"
                );
            }
        }
    }

    #[test]
    fn renderer_uses_vt_cell_geometry_for_unicode() {
        let mut child = vt100::Parser::new(2, 8, 0);
        child.process("☰X\r\n界e\u{301}Z".as_bytes());

        let menu = child.screen().cell(0, 0).unwrap();
        let menu_width = cell_to_paint(Some(menu)).width;
        assert_eq!(menu_width, if menu.is_wide() { 2 } else { 1 });
        assert_eq!(
            child
                .screen()
                .cell(0, u16::from(menu_width))
                .unwrap()
                .contents(),
            "X"
        );

        assert!(child.screen().cell(1, 0).unwrap().is_wide());
        assert!(child.screen().cell(1, 1).unwrap().is_wide_continuation());
        assert_eq!(child.screen().cell(1, 2).unwrap().contents(), "e\u{301}");
        assert_eq!(cell_to_paint(child.screen().cell(1, 2)).width, 1);

        let layout = Layout::new(8, 2, false, 0);
        let mut cache = TermCache::default();
        let mut host = vt100::Parser::new(2, 8, 0);
        host.process(&render_frame(&layout, child.screen(), &mut cache, true));

        assert_same_cells(child.screen(), host.screen(), 2, 8);
        assert_eq!(
            host.screen()
                .cell(0, u16::from(menu_width))
                .unwrap()
                .contents(),
            "X",
            "the cell following ☰ must not be swallowed by a width mismatch"
        );
    }

    #[test]
    fn renderer_preserves_dim_cells() {
        let mut child = vt100::Parser::new(1, 8, 0);
        child.process(b"\x1b[2mdim\x1b[22mN");

        assert!(child.screen().cell(0, 0).unwrap().dim());
        assert!(!child.screen().cell(0, 3).unwrap().dim());
        assert_ne!(cell_to_paint(child.screen().cell(0, 0)).attrs & ATTR_DIM, 0);

        let layout = Layout::new(8, 1, false, 0);
        let mut cache = TermCache::default();
        let frame = render_frame(&layout, child.screen(), &mut cache, true);
        assert!(contains(&frame, b"\x1b[2m"));

        let mut host = vt100::Parser::new(1, 8, 0);
        host.process(&frame);
        assert_same_cells(child.screen(), host.screen(), 1, 8);
        assert!(host.screen().cell(0, 0).unwrap().dim());
        assert!(!host.screen().cell(0, 3).unwrap().dim());
    }

    #[test]
    fn renderer_preserves_inverse_with_default_and_explicit_colors() {
        let mut child = vt100::Parser::new(1, 8, 0);
        child.process(b"\x1b[7mD\x1b[27m \x1b[31;44;7mC\x1b[0m");

        assert!(child.screen().cell(0, 0).unwrap().inverse());
        assert!(child.screen().cell(0, 2).unwrap().inverse());

        let layout = Layout::new(8, 1, false, 0);
        let mut cache = TermCache::default();
        let frame = render_frame(&layout, child.screen(), &mut cache, true);
        assert!(contains(&frame, b"\x1b[7m"));

        let mut host = vt100::Parser::new(1, 8, 0);
        host.process(&frame);
        assert_same_cells(child.screen(), host.screen(), 1, 8);
    }

    #[test]
    fn agent_frame_survives_partial_redraw_and_resize() {
        let mut child = vt100::Parser::new(5, 20, 0);
        child.process(
            concat!(
                "\x1b[?1049h\x1b[2J\x1b[H",
                "\x1b[48;2;18;20;24m\x1b[38;2;120;200;255m agent ☰X ",
                "\x1b[2;1H\x1b[48;2;24;26;30m\x1b[38;2;130;220;160mstatus: ",
                "\x1b[1mRUN\x1b[22m",
                "\x1b[3;1H\x1b[0mwide 界 combine e\u{301}",
                "\x1b[4;1Hpartial: old",
                "\x1b[4;10H\x1b[?25l"
            )
            .as_bytes(),
        );
        assert!(child.screen().alternate_screen());

        let initial_layout = Layout::new(20, 5, false, 0);
        let mut cache = TermCache::default();
        let mut host = vt100::Parser::new(5, 20, 0);
        let initial_bytes = render_frame(&initial_layout, child.screen(), &mut cache, true);
        assert!(contains(&initial_bytes, b"\x1b[38;2;120;200;255m"));
        assert!(contains(&initial_bytes, b"\x1b[48;2;18;20;24m"));
        host.process(&initial_bytes);
        assert_same_cells(child.screen(), host.screen(), 5, 20);
        assert!(host.screen().hide_cursor());
        assert_eq!(
            cell_to_paint(child.screen().cell(0, 1)).fg,
            PackedColor::from_vt(vt100::Color::Rgb(120, 200, 255))
        );

        child.process(b"\x1b[4;10H\x1b[38;2;255;170;70mNEW\x1b[0m\x1b[5;7H\x1b[?25h");
        let delta = render_frame(&initial_layout, child.screen(), &mut cache, false);
        assert!(contains(&delta, b"NEW"));
        assert!(!contains(&delta, "agent ☰X".as_bytes()));
        host.process(&delta);
        assert_same_cells(child.screen(), host.screen(), 5, 20);
        assert_eq!(
            host.screen().cursor_position(),
            child.screen().cursor_position()
        );
        assert!(!host.screen().hide_cursor());

        child.screen_mut().set_size(4, 14);
        host.screen_mut().set_size(4, 14);
        let shrunk = Layout::new(14, 4, false, 0);
        host.process(&render_frame(&shrunk, child.screen(), &mut cache, false));

        let mut fresh_cache = TermCache::default();
        let mut fresh_host = vt100::Parser::new(4, 14, 0);
        fresh_host.process(&render_frame(
            &shrunk,
            child.screen(),
            &mut fresh_cache,
            true,
        ));
        assert_same_cells(host.screen(), fresh_host.screen(), 4, 14);

        child.screen_mut().set_size(6, 22);
        child.process(b"\x1b[6;18Hedge");
        host.screen_mut().set_size(6, 22);
        let grown = Layout::new(22, 6, false, 0);
        host.process(&render_frame(&grown, child.screen(), &mut cache, false));

        let mut fresh_cache = TermCache::default();
        let mut fresh_host = vt100::Parser::new(6, 22, 0);
        fresh_host.process(&render_frame(
            &grown,
            child.screen(),
            &mut fresh_cache,
            true,
        ));
        assert_same_cells(host.screen(), fresh_host.screen(), 6, 22);
        assert_eq!(
            host.screen().cursor_position(),
            child.screen().cursor_position()
        );
    }

    #[test]
    fn preserved_resize_diff_matches_a_fresh_full_frame() {
        let mut child = vt100::Parser::new(3, 5, 0);
        child.process(b"abc\r\n\x1b[31mxy\x1b[0m");

        let initial = Layout::new(5, 3, false, 0);
        let mut cache = TermCache::default();
        let mut host = vt100::Parser::new(3, 5, 0);
        host.process(&render_frame(&initial, child.screen(), &mut cache, true));

        let grown = Layout::new(8, 5, false, 0);
        host.screen_mut().set_size(5, 8);
        host.process(&render_frame(&grown, child.screen(), &mut cache, false));

        let mut fresh_cache = TermCache::default();
        let mut fresh_host = vt100::Parser::new(5, 8, 0);
        fresh_host.process(&render_frame(
            &grown,
            child.screen(),
            &mut fresh_cache,
            true,
        ));
        assert_same_cells(host.screen(), fresh_host.screen(), 5, 8);

        let shrunk = Layout::new(4, 2, false, 0);
        host.screen_mut().set_size(2, 4);
        host.process(&render_frame(&shrunk, child.screen(), &mut cache, false));

        let mut fresh_cache = TermCache::default();
        let mut fresh_host = vt100::Parser::new(2, 4, 0);
        fresh_host.process(&render_frame(
            &shrunk,
            child.screen(),
            &mut fresh_cache,
            true,
        ));
        assert_same_cells(host.screen(), fresh_host.screen(), 2, 4);
    }

    #[test]
    fn suppresses_and_restores_cursor_without_repainting_cells() {
        let parser = vt100::Parser::new(2, 3, 0);
        let layout = Layout::new(3, 2, false, 0);
        let mut cache = TermCache::default();
        let mut out = Vec::new();

        draw_terminal(&mut out, &layout, parser.screen(), &mut cache, true, true).unwrap();
        assert!(contains(&out, b"\x1b[?25l"));
        assert!(!contains(&out, b"\x1b[?25h"));

        out.clear();
        draw_terminal(&mut out, &layout, parser.screen(), &mut cache, false, false).unwrap();
        assert!(contains(&out, b"\x1b[?25h"));
    }
}
