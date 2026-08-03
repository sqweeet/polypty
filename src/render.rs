use std::collections::BTreeMap;
use std::io::Write;

use anyhow::{Context, Result};
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::queue;
use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{Clear, ClearType};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use vt100::{Cell, Screen};

use crate::agent::{AgentState, AgentStatus};

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

/// Hit regions for sidebar mouse control.
#[derive(Debug, Clone, Default)]
pub struct SidebarMap {
    /// row -> tab index (None = empty / padding)
    pub row_tab: Vec<Option<usize>>,
    pub width: u16,
    pub has_visible_working: bool,
}

impl SidebarMap {
    pub fn tab_at(&self, col: u16, row: u16) -> Option<usize> {
        if col >= self.width {
            return None;
        }
        self.row_tab.get(row as usize).copied().flatten()
    }
}

/// Last painted sidebar rows. Resizes usually change only the footer and the
/// newly exposed edge, so avoid rewriting the entire sidebar every frame.
#[derive(Debug, Clone, Default)]
pub struct SidebarCache {
    width: u16,
    rows: Vec<SidebarPaintRow>,
}

impl SidebarCache {
    pub fn invalidate(&mut self) {
        self.rows.clear();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SidebarPaintRow {
    spans: Vec<SidebarPaintSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SidebarPaintSpan {
    text: String,
    bg: Color,
    fg: Color,
}

/// Sidebar tab row model — cmux-style primary + secondary.
#[derive(Debug, Clone)]
pub struct SidebarTab {
    pub primary: String,
    pub secondary: String,
    pub agent: Option<AgentStatus>,
    pub active: bool,
}

#[derive(Debug, Clone)]
struct TabCard {
    tab_idx: usize,
    active: bool,
    agent_state: Option<AgentState>,
    /// kind per line: 2 primary, 3 secondary, 6/7/8 agent state
    lines: Vec<(u8, String)>,
}

#[derive(Debug, Clone)]
struct SidebarContentRow {
    tab_idx: Option<usize>,
    active: bool,
    agent_state: Option<AgentState>,
    kind: u8,
    text: String,
}

fn build_cards(tabs: &[SidebarTab], inner_w: usize) -> Vec<TabCard> {
    let mut cards = Vec::with_capacity(tabs.len());
    let text_w = inner_w.max(1);

    for (i, tab) in tabs.iter().enumerate() {
        let mut lines = Vec::new();

        let (primary_kind, primary) = if let Some(status) = tab.agent {
            let kind = match status.state {
                AgentState::Ready => 6,
                AgentState::Working => 7,
                AgentState::Blocked => 8,
            };
            let primary = match status.state {
                AgentState::Working | AgentState::Ready => status.kind.label().to_string(),
                AgentState::Blocked => {
                    format!("{} · {}", status.kind.label(), status.state.label())
                }
            };
            (kind, primary)
        } else if tab.primary.is_empty() {
            (2, "shell".to_string())
        } else {
            (2, tab.primary.clone())
        };
        // A tab card is at most two rows: one title/process and one path.
        // Long titles ellipsize instead of pushing the path to a third row.
        for line in wrap_text(&primary, text_w, 1) {
            lines.push((primary_kind, line));
        }
        // Keep every card addressable even if a title consists entirely of
        // control characters (or cannot fit in a one-column sidebar).
        if lines.is_empty() {
            lines.push((primary_kind, wrap_text("shell", text_w, 1)[0].clone()));
        }

        if !tab.secondary.is_empty() {
            let sec = tab.secondary.trim();
            if !sec.is_empty() {
                let t = pad_fit(sec, text_w);
                let t = t.trim_end();
                if !t.is_empty() {
                    lines.push((3, t.to_string()));
                }
            }
        }

        cards.push(TabCard {
            tab_idx: i,
            active: tab.active,
            agent_state: tab.agent.map(|status| status.state),
            lines,
        });
    }
    cards
}

/// Select a compact, whole-card viewport containing the active tab.
///
/// Cards before the active one provide list context when they fit; remaining
/// space is filled from the following cards. If the viewport is only one row
/// high, the active primary line wins over its secondary metadata.
fn card_viewport_rows(cards: &[TabCard], capacity: usize) -> Vec<SidebarContentRow> {
    if cards.is_empty() || capacity == 0 {
        return Vec::new();
    }

    let active_idx = cards.iter().position(|card| card.active).unwrap_or(0);
    let active = &cards[active_idx];
    if active.lines.len() >= capacity {
        return active
            .lines
            .iter()
            .take(capacity)
            .map(|(kind, text)| SidebarContentRow {
                tab_idx: Some(active.tab_idx),
                active: active.active,
                agent_state: active.agent_state,
                kind: *kind,
                text: text.clone(),
            })
            .collect();
    }

    let mut start = active_idx;
    let mut end = active_idx + 1;
    let mut used = active.lines.len();

    while start > 0 {
        let previous_len = cards[start - 1].lines.len();
        if used + previous_len > capacity {
            break;
        }
        start -= 1;
        used += previous_len;
    }
    while end < cards.len() {
        let next_len = cards[end].lines.len();
        if used + next_len > capacity {
            break;
        }
        used += next_len;
        end += 1;
    }

    cards[start..end]
        .iter()
        .flat_map(|card| {
            card.lines.iter().map(|(kind, text)| SidebarContentRow {
                tab_idx: Some(card.tab_idx),
                active: card.active,
                agent_state: card.agent_state,
                kind: *kind,
                text: text.clone(),
            })
        })
        .collect()
}

fn working_glint_bg(active: bool, step: u64, column: usize, width: usize) -> Color {
    const SUBSTEPS_PER_CELL: u64 = 2;
    const WEIGHT: [u16; 9] = [255, 245, 217, 174, 126, 82, 45, 20, 0];

    let cycle = (width.saturating_add(8).max(1) as u64) * SUBSTEPS_PER_CELL;
    let center = (step % cycle) as i64 - 4 * SUBSTEPS_PER_CELL as i64;
    let column = (column as i64) * SUBSTEPS_PER_CELL as i64;
    let distance = (column - center).unsigned_abs() as usize;
    let weight = WEIGHT.get(distance).copied().unwrap_or(0);
    let (base, target) = if active {
        ((48, 48, 48), (64, 60, 52))
    } else {
        ((36, 36, 36), (50, 47, 41))
    };

    fn blend(base: u8, target: u8, weight: u16) -> u8 {
        let base = u16::from(base);
        let target = u16::from(target);
        ((base * (255 - weight) + target * weight + 127) / 255) as u8
    }

    Color::Rgb {
        r: blend(base.0, target.0, weight),
        g: blend(base.1, target.1, weight),
        b: blend(base.2, target.2, weight),
    }
}

fn sidebar_paint_spans(
    label: &str,
    width: usize,
    base_bg: Color,
    base_fg: Color,
    glint: Option<(bool, u64)>,
) -> Vec<SidebarPaintSpan> {
    let padded = pad_fit(label, width);
    let mut spans: Vec<SidebarPaintSpan> = Vec::new();
    let mut column = 0usize;

    for grapheme in UnicodeSegmentation::graphemes(padded.as_str(), true) {
        let grapheme_width = UnicodeWidthStr::width(grapheme);
        if grapheme_width == 0 {
            if let Some(span) = spans.last_mut() {
                span.text.push_str(grapheme);
            }
            continue;
        }
        let bg = glint
            .map(|(active, step)| working_glint_bg(active, step, column, width))
            .unwrap_or(base_bg);
        if let Some(span) = spans
            .last_mut()
            .filter(|span| span.bg == bg && span.fg == base_fg)
        {
            span.text.push_str(grapheme);
        } else {
            spans.push(SidebarPaintSpan {
                text: grapheme.to_string(),
                bg,
                fg: base_fg,
            });
        }
        column = column.saturating_add(grapheme_width);
    }
    spans
}

fn ready_badge_spans(
    label: &str,
    width: usize,
    base_bg: Color,
    base_fg: Color,
) -> Vec<SidebarPaintSpan> {
    const BADGE: &str = " READY ";
    const BADGE_WIDTH: usize = 7;
    let badge_bg = Color::Rgb {
        r: 105,
        g: 180,
        b: 132,
    };
    let badge_fg = Color::Rgb {
        r: 18,
        g: 28,
        b: 22,
    };

    if width <= BADGE_WIDTH {
        return vec![SidebarPaintSpan {
            text: pad_fit("READY", width),
            bg: badge_bg,
            fg: badge_fg,
        }];
    }

    let mut spans = sidebar_paint_spans(label, width - BADGE_WIDTH, base_bg, base_fg, None);
    spans.push(SidebarPaintSpan {
        text: BADGE.to_string(),
        bg: badge_bg,
        fg: badge_fg,
    });
    spans
}

fn sidebar_footer(width: usize, height: usize) -> Vec<(u8, &'static str)> {
    let full = [
        (4, "shortcuts"),
        (5, "Alt+t new tab"),
        (5, "Alt+w close tab"),
        (5, "Alt+[/] tabs"),
        (5, "Alt+v/s split"),
        (5, "Alt+hjkl pane"),
        (5, "Alt+x close pane"),
        (5, "Alt+b sidebar"),
        (5, "Alt+q quit"),
    ];
    let compact = [
        (4, "keys"),
        (5, "Alt+t new"),
        (5, "Alt+w tab"),
        (5, "Alt+[] tab"),
        (5, "Alt+v/s"),
        (5, "Alt+hjkl"),
        (5, "Alt+x pane"),
        (5, "Alt+b bar"),
        (5, "Alt+q out"),
    ];

    // Always leave one row for the "tabs" heading and two rows for the
    // active tab card. On short terminals, keep the most useful shortcuts.
    let max_rows = height.saturating_sub(3).min(full.len());
    if max_rows < 2 {
        return Vec::new();
    }

    let rows = if width >= 14 { &full } else { &compact };
    rows.iter().copied().take(max_rows).collect()
}

/// Multi-line sidebar cards + mouse hit map (cmux-like info).
pub fn draw_sidebar(
    out: &mut impl Write,
    layout: &Layout,
    tabs: &[SidebarTab],
    cache: &mut SidebarCache,
    glint_step: u64,
    force: bool,
) -> Result<SidebarMap> {
    let mut map = SidebarMap {
        row_tab: vec![None; layout.rows as usize],
        width: layout.sidebar_width,
        has_visible_working: false,
    };

    if !layout.sidebar_visible || layout.sidebar_width == 0 {
        return Ok(map);
    }

    let w = layout.sidebar_width as usize;
    let h = layout.rows as usize;
    // Full width for text — no gutter column / accent strip.
    let inner = w.max(1);

    // Neutral gray — equal RGB, no blue cast.
    let bg = Color::Rgb {
        r: 36,
        g: 36,
        b: 36,
    };
    let bg_active = Color::Rgb {
        r: 48,
        g: 48,
        b: 48,
    };
    let fg_active = Color::Rgb {
        r: 188,
        g: 188,
        b: 188,
    };
    let fg_idle = Color::Rgb {
        r: 120,
        g: 120,
        b: 120,
    };
    let fg_sec_active = Color::Rgb {
        r: 130,
        g: 130,
        b: 130,
    };
    let fg_sec_idle = Color::Rgb {
        r: 96,
        g: 96,
        b: 96,
    };
    let fg_blocked = Color::Rgb {
        r: 220,
        g: 105,
        b: 105,
    };

    let cards = build_cards(tabs, inner);
    let footer = sidebar_footer(w, h);
    let footer_start = h.saturating_sub(footer.len());

    // (tab_idx, active, agent state, kind, text). Keep the section heading attached
    // directly to a viewport of cards. On a one-row terminal the active card
    // takes priority over the heading, so resize can never hide the active tab.
    let content_height = footer_start;
    let show_heading = content_height > usize::from(!cards.is_empty());
    let mut rows = Vec::new();
    if show_heading {
        rows.push(SidebarContentRow {
            tab_idx: None,
            active: false,
            agent_state: None,
            kind: 4,
            text: "tabs".to_string(),
        });
    }
    rows.extend(card_viewport_rows(
        &cards,
        content_height.saturating_sub(rows.len()),
    ));

    let mut painted_rows = Vec::with_capacity(h);
    for y in 0..h {
        let (tab_idx, active, agent_state, kind, text) = if y >= footer_start {
            let (kind, text) = footer[y - footer_start];
            (None, false, None, kind, text)
        } else if y < rows.len() {
            let row = &rows[y];
            (
                row.tab_idx,
                row.active,
                row.agent_state,
                row.kind,
                row.text.as_str(),
            )
        } else {
            (None, false, None, 0u8, "")
        };

        if let Some(idx) = tab_idx {
            map.row_tab[y] = Some(idx);
        }
        let working_row = agent_state == Some(AgentState::Working);
        map.has_visible_working |= working_row;

        let row_bg = if active && kind != 0 { bg_active } else { bg };
        let fg = match kind {
            2 if active => fg_active,
            2 => fg_idle,
            3 if active => fg_sec_active,
            3 | 4 => fg_sec_idle,
            5 => fg_idle,
            6 if active => fg_active,
            6 => fg_idle,
            7 if active => fg_active,
            7 => fg_idle,
            8 => fg_blocked,
            _ => fg_idle,
        };

        let spans = if kind == 6 {
            ready_badge_spans(text, w, row_bg, fg)
        } else {
            sidebar_paint_spans(
                text,
                w,
                row_bg,
                fg,
                working_row.then_some((active, glint_step)),
            )
        };
        painted_rows.push(SidebarPaintRow { spans });
    }

    let width_changed = cache.width != layout.sidebar_width;
    let mut cursor_hidden = false;
    for (y, row) in painted_rows.iter().enumerate() {
        let changed = force || width_changed || cache.rows.get(y) != Some(row);
        if !changed {
            continue;
        }
        if !cursor_hidden {
            // Sidebar painting can walk to the final row. draw_terminal restores
            // the real PTY cursor after all changed rows are painted.
            queue!(out, Hide)?;
            cursor_hidden = true;
        }
        queue!(out, MoveTo(0, y as u16))?;
        for span in &row.spans {
            queue!(
                out,
                SetForegroundColor(span.fg),
                SetBackgroundColor(span.bg),
                Print(&span.text)
            )?;
        }
    }

    cache.width = layout.sidebar_width;
    cache.rows = painted_rows;
    if cursor_hidden {
        queue!(out, ResetColor, SetAttribute(Attribute::Reset))?;
    }
    Ok(map)
}

fn wrap_text(s: &str, width: usize, max_lines: usize) -> Vec<String> {
    if width == 0 || max_lines == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0usize;
    let mut truncated = false;

    for raw_grapheme in UnicodeSegmentation::graphemes(s, true) {
        let grapheme: String = raw_grapheme
            .chars()
            .filter(|character| !character.is_control())
            .collect();
        if grapheme.is_empty() {
            continue;
        }

        let grapheme_width = UnicodeWidthStr::width(grapheme.as_str());
        if grapheme_width == 0 {
            // A leading standalone combining mark has no safe host cell to
            // attach to. Marks belonging to visible text are part of the same
            // extended grapheme and take the ordinary path below.
            if !cur.is_empty() {
                cur.push_str(&grapheme);
            }
            continue;
        }
        if grapheme_width > width {
            truncated = true;
            continue;
        }
        if cur_w + grapheme_width > width && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
            cur_w = 0;
            if lines.len() >= max_lines {
                truncated = true;
                break;
            }
        }
        cur.push_str(&grapheme);
        cur_w += grapheme_width;
    }
    if lines.len() < max_lines && !cur.is_empty() {
        lines.push(cur);
    }

    if truncated && !lines.is_empty() {
        let last = lines.last_mut().expect("non-empty lines");
        while UnicodeWidthStr::width(last.as_str()) + 1 > width && !last.is_empty() {
            let start = UnicodeSegmentation::grapheme_indices(last.as_str(), true)
                .next_back()
                .map(|(index, _)| index)
                .unwrap_or(0);
            last.truncate(start);
        }
        if UnicodeWidthStr::width(last.as_str()) < width {
            last.push('…');
        }
    }
    lines
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

fn pad_fit(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut w = 0usize;
    for raw_grapheme in UnicodeSegmentation::graphemes(s, true) {
        let grapheme: String = raw_grapheme
            .chars()
            .filter(|character| !character.is_control())
            .collect();
        if grapheme.is_empty() {
            continue;
        }
        let grapheme_width = UnicodeWidthStr::width(grapheme.as_str());
        if grapheme_width == 0 && out.is_empty() {
            continue;
        }
        if w + grapheme_width > width {
            break;
        }
        out.push_str(&grapheme);
        w += grapheme_width;
    }
    while w < width {
        out.push(' ');
        w += 1;
    }
    out
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

    #[test]
    fn sidebar_text_preserves_combining_graphemes_without_false_ellipsis() {
        let title = "e\u{301}";

        assert_eq!(wrap_text(title, 4, 1), vec![title.to_string()]);
        assert_eq!(pad_fit(title, 4), format!("{title}   "));
        assert_eq!(UnicodeWidthStr::width(pad_fit(title, 4).as_str()), 4);
    }

    #[test]
    fn sidebar_text_keeps_zwj_emoji_atomic() {
        let emoji = "👩‍💻";
        let title = format!("{emoji}x");

        assert_eq!(UnicodeWidthStr::width(emoji), 2);
        assert_eq!(wrap_text(&title, 3, 1), vec![title]);
        assert_eq!(pad_fit(emoji, 4), format!("{emoji}  "));
        assert_eq!(UnicodeWidthStr::width(pad_fit(emoji, 4).as_str()), 4);
    }

    #[test]
    fn glint_spans_preserve_exact_unicode_row_width() {
        for width in [1, 10, 18] {
            let spans = sidebar_paint_spans(
                "claude 👩‍💻",
                width,
                Color::Rgb {
                    r: 48,
                    g: 48,
                    b: 48,
                },
                Color::White,
                Some((true, 7)),
            );
            let text: String = spans.iter().map(|span| span.text.as_str()).collect();
            assert_eq!(UnicodeWidthStr::width(text.as_str()), width);
        }
    }

    #[test]
    fn tab_cards_have_no_blank_rows_between_them() {
        let tabs = [
            SidebarTab {
                primary: "one".into(),
                secondary: String::new(),
                agent: None,
                active: true,
            },
            SidebarTab {
                primary: "two".into(),
                secondary: String::new(),
                agent: None,
                active: false,
            },
        ];

        let cards = build_cards(&tabs, 18);
        assert!(cards.iter().all(|card| card.lines.len() == 1));
        assert_eq!(cards[0].lines[0].1, "one");
        assert!(cards
            .iter()
            .flat_map(|card| &card.lines)
            .all(|(kind, _)| *kind != 0));
        assert_eq!(sidebar_footer(18, 12)[0].1, "shortcuts");

        let long = [SidebarTab {
            primary: "a very long process title".into(),
            secondary: "~/projects/mux".into(),
            agent: None,
            active: true,
        }];
        let cards = build_cards(&long, 8);
        assert_eq!(cards[0].lines.len(), 2);
        assert_eq!(cards[0].lines[0].0, 2);
        assert_eq!(cards[0].lines[1].0, 3);
    }

    #[test]
    fn sidebar_footer_is_anchored_and_not_clickable() {
        let layout = Layout::new(40, 12, true, 18);
        let tabs = [SidebarTab {
            primary: "shell".into(),
            secondary: "~/projects/mux".into(),
            agent: None,
            active: true,
        }];
        let mut out = Vec::new();
        let mut cache = SidebarCache::default();

        let map = draw_sidebar(&mut out, &layout, &tabs, &mut cache, 0, false).unwrap();
        let footer_rows = sidebar_footer(18, 12).len();
        let footer_start = 12 - footer_rows;

        assert!(map.row_tab[0].is_none());
        assert!(map.row_tab[1].is_some());
        assert!(map.row_tab[footer_start..].iter().all(Option::is_none));
        assert!(String::from_utf8_lossy(&out).contains("tabs"));
        assert!(String::from_utf8_lossy(&out).contains("Alt+t new tab"));
        assert!(String::from_utf8_lossy(&out).contains("Alt+q quit"));

        out.clear();
        draw_sidebar(&mut out, &layout, &tabs, &mut cache, 0, false).unwrap();
        assert!(out.is_empty(), "unchanged sidebar should emit no bytes");
    }

    #[test]
    fn sidebar_viewport_keeps_active_tab_visible_when_shrinking() {
        let tabs: Vec<SidebarTab> = (0..8)
            .map(|idx| SidebarTab {
                primary: format!("agent-{idx}"),
                secondary: if idx == 6 {
                    "~/projects/mux".to_string()
                } else {
                    String::new()
                },
                agent: None,
                active: idx == 6,
            })
            .collect();
        let mut cache = SidebarCache::default();

        for rows in [12, 7, 5, 2, 1] {
            let layout = Layout::new(40, rows, true, 18);
            let mut out = Vec::new();
            let map = draw_sidebar(&mut out, &layout, &tabs, &mut cache, 0, false).unwrap();

            assert!(
                map.row_tab.contains(&Some(6)),
                "active tab disappeared at {rows} rows: {:?}",
                map.row_tab
            );
            assert!(map
                .row_tab
                .iter()
                .flatten()
                .all(|tab_idx| *tab_idx < tabs.len()));
        }

        let one_row = Layout::new(40, 1, true, 18);
        let mut out = Vec::new();
        let map = draw_sidebar(&mut out, &one_row, &tabs, &mut cache, 0, true).unwrap();
        assert_eq!(map.row_tab, vec![Some(6)]);
        assert!(String::from_utf8_lossy(&out).contains("agent-6"));
    }

    #[test]
    fn agent_status_keeps_working_label_minimal() {
        let mut tabs = [SidebarTab {
            primary: "node".into(),
            secondary: "~/projects/mux".into(),
            agent: Some(AgentStatus {
                kind: crate::agent::AgentKind::Codex,
                state: AgentState::Working,
            }),
            active: true,
        }];

        let cards = build_cards(&tabs, 18);
        assert_eq!(cards[0].lines.len(), 2);
        assert_eq!(cards[0].lines[0], (7, "codex".into()));
        assert_eq!(cards[0].lines[1], (3, "~/projects/mux".into()));

        tabs[0].agent.as_mut().unwrap().state = AgentState::Ready;
        assert_eq!(build_cards(&tabs, 18)[0].lines[0], (6, "codex".into()));
        tabs[0].agent.as_mut().unwrap().state = AgentState::Blocked;
        assert_eq!(
            build_cards(&tabs, 18)[0].lines[0],
            (8, "codex · blocked".into())
        );
    }

    #[test]
    fn working_glint_covers_both_rows_of_the_card() {
        let layout = Layout::new(40, 12, true, 18);
        let tabs = [SidebarTab {
            primary: "node".into(),
            secondary: "~/projects/mux".into(),
            agent: Some(AgentStatus {
                kind: crate::agent::AgentKind::Codex,
                state: AgentState::Working,
            }),
            active: true,
        }];
        let mut out = Vec::new();
        let mut cache = SidebarCache::default();

        let map = draw_sidebar(&mut out, &layout, &tabs, &mut cache, 1, false).unwrap();
        assert!(map.has_visible_working);
        let primary = cache.rows[1].clone();
        let secondary = cache.rows[2].clone();
        let primary_text: String = primary
            .spans
            .iter()
            .map(|span| span.text.as_str())
            .collect();
        assert_eq!(primary_text, pad_fit("codex", 18));

        out.clear();
        draw_sidebar(&mut out, &layout, &tabs, &mut cache, 1, false).unwrap();
        assert!(out.is_empty());

        draw_sidebar(&mut out, &layout, &tabs, &mut cache, 7, false).unwrap();
        assert!(!out.is_empty());
        assert_ne!(cache.rows[1], primary);
        assert_ne!(cache.rows[2], secondary);
    }

    #[test]
    fn glint_colors_move_in_small_interpolated_steps() {
        fn rgb(color: Color) -> (u8, u8, u8) {
            match color {
                Color::Rgb { r, g, b } => (r, g, b),
                _ => panic!("expected RGB color"),
            }
        }

        assert_eq!(rgb(working_glint_bg(true, 18, 5, 18)), (64, 60, 52));
        assert_eq!(rgb(working_glint_bg(true, 0, 17, 18)), (48, 48, 48));

        for active in [false, true] {
            for column in 0..18 {
                let before = rgb(working_glint_bg(active, 10, column, 18));
                let after = rgb(working_glint_bg(active, 11, column, 18));
                assert!(before.0.abs_diff(after.0) <= 4);
                assert!(before.1.abs_diff(after.1) <= 4);
                assert!(before.2.abs_diff(after.2) <= 4);
            }
        }
    }

    #[test]
    fn ready_is_a_right_aligned_badge_with_dark_text() {
        let base_bg = Color::Rgb {
            r: 48,
            g: 48,
            b: 48,
        };
        let base_fg = Color::Rgb {
            r: 188,
            g: 188,
            b: 188,
        };
        let spans = ready_badge_spans("codex", 18, base_bg, base_fg);
        let text: String = spans.iter().map(|span| span.text.as_str()).collect();
        let badge = spans.last().unwrap();

        assert_eq!(UnicodeWidthStr::width(text.as_str()), 18);
        assert!(text.ends_with(" READY "));
        assert_eq!(
            badge.bg,
            Color::Rgb {
                r: 105,
                g: 180,
                b: 132
            }
        );
        assert_eq!(
            badge.fg,
            Color::Rgb {
                r: 18,
                g: 28,
                b: 22
            }
        );

        for width in [1, 5, 7, 10, 18] {
            let spans = ready_badge_spans("codex", width, base_bg, base_fg);
            let text: String = spans.iter().map(|span| span.text.as_str()).collect();
            assert_eq!(UnicodeWidthStr::width(text.as_str()), width);
            assert_eq!(spans.last().unwrap().fg, badge.fg);
            assert_eq!(spans.last().unwrap().bg, badge.bg);
        }
    }

    #[test]
    fn offscreen_working_tab_does_not_keep_glint_running() {
        let tabs: Vec<SidebarTab> = (0..8)
            .map(|index| SidebarTab {
                primary: format!("tab-{index}"),
                secondary: String::new(),
                agent: (index == 0).then_some(AgentStatus {
                    kind: crate::agent::AgentKind::Codex,
                    state: AgentState::Working,
                }),
                active: index == 6,
            })
            .collect();
        let layout = Layout::new(40, 5, true, 18);
        let mut out = Vec::new();
        let mut cache = SidebarCache::default();

        let map = draw_sidebar(&mut out, &layout, &tabs, &mut cache, 1, false).unwrap();
        assert!(!map.has_visible_working);
    }
}
