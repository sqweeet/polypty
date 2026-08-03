use std::io::Write;
use std::time::Duration;

use anyhow::Result;
use crossterm::cursor::{Hide, MoveTo};
use crossterm::queue;
use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::agent::{AgentState, AgentStatus};

use super::Layout;

/// Hit regions for sidebar mouse control.
#[derive(Debug, Clone, Default)]
pub struct SidebarMap {
    /// row -> tab index (None = empty / padding)
    pub row_tab: Vec<Option<usize>>,
    pub width: u16,
    visible_glints: Vec<(u64, GlintFrame)>,
}

impl SidebarMap {
    pub fn tab_at(&self, col: u16, row: u16) -> Option<usize> {
        if col >= self.width {
            return None;
        }
        self.row_tab.get(row as usize).copied().flatten()
    }

    pub fn visible_glints(&self) -> &[(u64, GlintFrame)] {
        &self.visible_glints
    }

    #[cfg(test)]
    pub(crate) fn with_visible_glints(visible_glints: Vec<(u64, GlintFrame)>) -> Self {
        Self {
            visible_glints,
            ..Self::default()
        }
    }
}

/// One card-local animation frame. Frames 1..48 are the visible sweep; 50
/// collapses both off-card endpoints and the two-second rest, so the renderer
/// sleeps instead of repainting an unchanged base background.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlintFrame(u8);

impl GlintFrame {
    const FRAME_MS: u128 = 80;
    const SWEEP_FRAMES: u128 = 50;
    const CYCLE_FRAMES: u128 = 75;
    const REST: Self = Self(Self::SWEEP_FRAMES as u8);

    pub(crate) fn for_elapsed(elapsed: Duration) -> Self {
        let frame = (elapsed.as_millis() / Self::FRAME_MS) % Self::CYCLE_FRAMES;
        // The sweep is fully outside the card at both endpoints. Collapse
        // those visually base-only frames into REST as well, avoiding two
        // no-op sidebar writes at the rest/cycle boundaries.
        if frame > 0 && frame + 1 < Self::SWEEP_FRAMES {
            Self(frame as u8)
        } else {
            Self::REST
        }
    }

    fn progress(self) -> Option<f32> {
        (self.0 < Self::SWEEP_FRAMES as u8)
            .then_some(f32::from(self.0) / (Self::SWEEP_FRAMES as f32 - 1.0))
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
    pub key: u64,
    pub primary: String,
    pub secondary: String,
    pub agent: Option<AgentStatus>,
    pub glint_frame: Option<GlintFrame>,
    pub active: bool,
}

#[derive(Debug, Clone)]
struct TabCard {
    tab_idx: usize,
    key: u64,
    active: bool,
    agent_state: Option<AgentState>,
    glint_frame: Option<GlintFrame>,
    /// kind per line: 2 primary, 3 secondary, 6/7/8 agent state
    lines: Vec<(u8, String)>,
}

#[derive(Debug, Clone)]
struct SidebarContentRow {
    tab_idx: Option<usize>,
    key: Option<u64>,
    active: bool,
    agent_state: Option<AgentState>,
    glint_frame: Option<GlintFrame>,
    kind: u8,
    text: String,
}

fn agent_primary_label(status: AgentStatus, width: usize) -> String {
    let count = status.panes.max(1);
    let mut identity = status.kind.label().to_string();
    if count > 1 {
        let suffix = if status.mixed_kinds {
            format!("+{}", count - 1)
        } else {
            format!(" ×{count}")
        };
        identity.push_str(&suffix);
    }

    if status.state != AgentState::Blocked {
        return identity;
    }

    const BLOCKED: &str = "blocked";
    const BLOCKED_SUFFIX: &str = " · blocked";
    let blocked_width = UnicodeWidthStr::width(BLOCKED);
    let suffix_width = UnicodeWidthStr::width(BLOCKED_SUFFIX);
    if width < blocked_width {
        return "!".to_string();
    }
    if width <= suffix_width {
        return BLOCKED.to_string();
    }

    let identity_width = width - suffix_width;
    let identity = wrap_text(&identity, identity_width, 1)
        .into_iter()
        .next()
        .unwrap_or_default();
    format!("{identity}{BLOCKED_SUFFIX}")
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
            (kind, agent_primary_label(status, text_w))
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
            key: tab.key,
            active: tab.active,
            agent_state: tab.agent.map(|status| status.state),
            glint_frame: tab.glint_frame,
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
                key: Some(active.key),
                active: active.active,
                agent_state: active.agent_state,
                glint_frame: active.glint_frame,
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
                key: Some(card.key),
                active: card.active,
                agent_state: card.agent_state,
                glint_frame: card.glint_frame,
                kind: *kind,
                text: text.clone(),
            })
        })
        .collect()
}

fn working_glint_bg(active: bool, frame: GlintFrame, column: usize, width: usize) -> Color {
    let (base, target) = if active { (48u8, 72u8) } else { (36u8, 54u8) };

    // A moving highlight is unreadable in an extremely narrow bar. Keep the
    // semantic working lift, but remove motion and its timer entirely.
    if width < 6 {
        let lift = if active { 56 } else { 42 };
        return Color::Rgb {
            r: lift,
            g: lift,
            b: lift,
        };
    }

    let Some(progress) = frame.progress() else {
        return Color::Rgb {
            r: base,
            g: base,
            b: base,
        };
    };

    let radius = (width as f32 / 4.0).clamp(1.5, 4.0);
    let last_column = width.saturating_sub(1) as f32;
    let center = -radius + progress * (last_column + radius * 2.0);
    let distance = (column as f32 - center).abs();
    let linear = (1.0 - distance / radius).clamp(0.0, 1.0);
    let weight = linear * linear * (3.0 - 2.0 * linear);
    let value = (f32::from(base) + f32::from(target - base) * weight).round() as u8;

    Color::Rgb {
        r: value,
        g: value,
        b: value,
    }
}

fn sidebar_paint_spans(
    label: &str,
    width: usize,
    base_bg: Color,
    base_fg: Color,
    glint: Option<(bool, GlintFrame)>,
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
            .map(|(active, frame)| working_glint_bg(active, frame, column, width))
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
    const BADGE: &str = " ✓ ";
    const BADGE_WIDTH: usize = 3;
    let badge_bg = Color::Rgb {
        r: 55,
        g: 82,
        b: 65,
    };
    let badge_fg = Color::Rgb {
        r: 184,
        g: 219,
        b: 196,
    };

    if width < BADGE_WIDTH {
        let mut text = " ".repeat(width.saturating_sub(1));
        if width > 0 {
            text.push('✓');
        }
        return vec![SidebarPaintSpan {
            text,
            bg: badge_bg,
            fg: badge_fg,
        }];
    }

    if width == BADGE_WIDTH {
        return vec![SidebarPaintSpan {
            text: BADGE.to_string(),
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
    force: bool,
) -> Result<SidebarMap> {
    let mut map = SidebarMap {
        row_tab: vec![None; layout.rows as usize],
        width: layout.sidebar_width,
        visible_glints: Vec::new(),
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
            key: None,
            active: false,
            agent_state: None,
            glint_frame: None,
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
        let (tab_idx, key, active, agent_state, glint_frame, kind, text) = if y >= footer_start {
            let (kind, text) = footer[y - footer_start];
            (None, None, false, None, None, kind, text)
        } else if y < rows.len() {
            let row = &rows[y];
            (
                row.tab_idx,
                row.key,
                row.active,
                row.agent_state,
                row.glint_frame,
                row.kind,
                row.text.as_str(),
            )
        } else {
            (None, None, false, None, None, 0u8, "")
        };

        if let Some(idx) = tab_idx {
            map.row_tab[y] = Some(idx);
        }
        let working_row = agent_state == Some(AgentState::Working);
        if working_row && w >= 6 {
            if let (Some(key), Some(frame)) = (key, glint_frame) {
                if !map
                    .visible_glints
                    .iter()
                    .any(|(visible, _)| *visible == key)
                {
                    map.visible_glints.push((key, frame));
                }
            }
        }

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
                working_row.then_some((active, glint_frame.unwrap_or(GlintFrame::REST))),
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
                Some((true, GlintFrame(7))),
            );
            let text: String = spans.iter().map(|span| span.text.as_str()).collect();
            assert_eq!(UnicodeWidthStr::width(text.as_str()), width);
        }
    }

    #[test]
    fn tab_cards_have_no_blank_rows_between_them() {
        let tabs = [
            SidebarTab {
                key: 1,
                primary: "one".into(),
                secondary: String::new(),
                agent: None,
                glint_frame: None,
                active: true,
            },
            SidebarTab {
                key: 2,
                primary: "two".into(),
                secondary: String::new(),
                agent: None,
                glint_frame: None,
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
            key: 1,
            primary: "a very long process title".into(),
            secondary: "~/projects/mux".into(),
            agent: None,
            glint_frame: None,
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
            key: 1,
            primary: "shell".into(),
            secondary: "~/projects/mux".into(),
            agent: None,
            glint_frame: None,
            active: true,
        }];
        let mut out = Vec::new();
        let mut cache = SidebarCache::default();

        let map = draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
        let footer_rows = sidebar_footer(18, 12).len();
        let footer_start = 12 - footer_rows;

        assert!(map.row_tab[0].is_none());
        assert!(map.row_tab[1].is_some());
        assert!(map.row_tab[footer_start..].iter().all(Option::is_none));
        assert!(String::from_utf8_lossy(&out).contains("tabs"));
        assert!(String::from_utf8_lossy(&out).contains("Alt+t new tab"));
        assert!(String::from_utf8_lossy(&out).contains("Alt+q quit"));

        out.clear();
        draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
        assert!(out.is_empty(), "unchanged sidebar should emit no bytes");
    }

    #[test]
    fn sidebar_viewport_keeps_active_tab_visible_when_shrinking() {
        let tabs: Vec<SidebarTab> = (0..8)
            .map(|idx| SidebarTab {
                key: idx as u64,
                primary: format!("agent-{idx}"),
                secondary: if idx == 6 {
                    "~/projects/mux".to_string()
                } else {
                    String::new()
                },
                agent: None,
                glint_frame: None,
                active: idx == 6,
            })
            .collect();
        let mut cache = SidebarCache::default();

        for rows in [12, 7, 5, 2, 1] {
            let layout = Layout::new(40, rows, true, 18);
            let mut out = Vec::new();
            let map = draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();

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
        let map = draw_sidebar(&mut out, &one_row, &tabs, &mut cache, true).unwrap();
        assert_eq!(map.row_tab, vec![Some(6)]);
        assert!(String::from_utf8_lossy(&out).contains("agent-6"));
    }

    #[test]
    fn agent_status_keeps_working_label_minimal() {
        let mut tabs = [SidebarTab {
            key: 1,
            primary: "node".into(),
            secondary: "~/projects/mux".into(),
            agent: Some(AgentStatus::single(
                crate::agent::AgentKind::Codex,
                AgentState::Working,
            )),
            glint_frame: Some(GlintFrame(10)),
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
    fn agent_status_labels_split_panes_compactly() {
        let mut tabs = [SidebarTab {
            key: 1,
            primary: "node".into(),
            secondary: "~/projects/mux".into(),
            agent: Some(AgentStatus {
                kind: crate::agent::AgentKind::Codex,
                state: AgentState::Working,
                panes: 2,
                mixed_kinds: false,
            }),
            glint_frame: Some(GlintFrame(10)),
            active: true,
        }];

        assert_eq!(build_cards(&tabs, 18)[0].lines[0], (7, "codex ×2".into()));
        tabs[0].agent.as_mut().unwrap().state = AgentState::Ready;
        assert_eq!(build_cards(&tabs, 18)[0].lines[0], (6, "codex ×2".into()));
        tabs[0].agent.as_mut().unwrap().state = AgentState::Blocked;
        assert_eq!(
            build_cards(&tabs, 18)[0].lines[0],
            (8, "codex ×2 · blocked".into())
        );

        let status = tabs[0].agent.as_mut().unwrap();
        status.kind = crate::agent::AgentKind::Claude;
        status.state = AgentState::Working;
        status.mixed_kinds = true;
        assert_eq!(build_cards(&tabs, 18)[0].lines[0], (7, "claude+1".into()));

        tabs[0].agent.as_mut().unwrap().state = AgentState::Blocked;
        assert_eq!(
            build_cards(&tabs, 18)[0].lines[0],
            (8, "claude+1 · blocked".into())
        );
        assert_eq!(
            build_cards(&tabs, 12)[0].lines[0],
            (8, "c… · blocked".into())
        );
        assert_eq!(build_cards(&tabs, 8)[0].lines[0], (8, "blocked".into()));
        assert_eq!(build_cards(&tabs, 5)[0].lines[0], (8, "!".into()));
    }

    #[test]
    fn working_glint_covers_both_rows_of_the_card() {
        let layout = Layout::new(40, 12, true, 18);
        let mut tabs = [SidebarTab {
            key: 1,
            primary: "node".into(),
            secondary: "~/projects/mux".into(),
            agent: Some(AgentStatus::single(
                crate::agent::AgentKind::Codex,
                AgentState::Working,
            )),
            glint_frame: Some(GlintFrame(10)),
            active: true,
        }];
        let mut out = Vec::new();
        let mut cache = SidebarCache::default();

        let map = draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
        assert_eq!(map.visible_glints(), &[(1, GlintFrame(10))]);
        let primary = cache.rows[1].clone();
        let secondary = cache.rows[2].clone();
        let primary_text: String = primary
            .spans
            .iter()
            .map(|span| span.text.as_str())
            .collect();
        assert_eq!(primary_text, pad_fit("codex", 18));

        out.clear();
        draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
        assert!(out.is_empty());

        tabs[0].glint_frame = Some(GlintFrame(20));
        draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
        assert!(!out.is_empty());
        assert_ne!(cache.rows[1], primary);
        assert_ne!(cache.rows[2], secondary);
    }

    #[test]
    fn glint_is_neutral_smooth_and_has_a_true_rest() {
        fn rgb(color: Color) -> (u8, u8, u8) {
            match color {
                Color::Rgb { r, g, b } => (r, g, b),
                _ => panic!("expected RGB color"),
            }
        }

        assert_eq!(
            rgb(working_glint_bg(true, GlintFrame::REST, 5, 18)),
            (48, 48, 48)
        );
        assert_eq!(
            rgb(working_glint_bg(false, GlintFrame::REST, 5, 18)),
            (36, 36, 36)
        );
        assert_eq!(
            rgb(working_glint_bg(true, GlintFrame(25), 8, 18)),
            (70, 70, 70)
        );

        for active in [false, true] {
            let max = if active { 72 } else { 54 };
            for frame in 0..50 {
                for column in 0..18 {
                    let (r, g, b) = rgb(working_glint_bg(active, GlintFrame(frame), column, 18));
                    assert_eq!(r, g);
                    assert_eq!(g, b);
                    assert!(r <= max);
                }
            }
        }

        for active in [false, true] {
            for column in 0..18 {
                let before = rgb(working_glint_bg(active, GlintFrame(10), column, 18));
                let after = rgb(working_glint_bg(active, GlintFrame(11), column, 18));
                assert!(before.0.abs_diff(after.0) <= 7);
                assert!(before.1.abs_diff(after.1) <= 7);
                assert!(before.2.abs_diff(after.2) <= 7);
            }
        }

        assert_eq!(
            rgb(working_glint_bg(true, GlintFrame(20), 0, 5)),
            (56, 56, 56)
        );
        assert_eq!(
            rgb(working_glint_bg(false, GlintFrame(20), 0, 5)),
            (42, 42, 42)
        );
    }

    #[test]
    fn glint_timeline_collapses_the_two_second_rest() {
        assert_eq!(
            GlintFrame::for_elapsed(Duration::from_millis(79)),
            GlintFrame::REST
        );
        assert_eq!(
            GlintFrame::for_elapsed(Duration::from_millis(80)),
            GlintFrame(1)
        );
        assert_eq!(
            GlintFrame::for_elapsed(Duration::from_millis(3_920)),
            GlintFrame::REST
        );
        assert_eq!(
            GlintFrame::for_elapsed(Duration::from_millis(4_000)),
            GlintFrame::REST
        );
        assert_eq!(
            GlintFrame::for_elapsed(Duration::from_millis(5_920)),
            GlintFrame::REST
        );
        assert_eq!(
            GlintFrame::for_elapsed(Duration::from_millis(6_000)),
            GlintFrame::REST
        );
        assert_eq!(
            GlintFrame::for_elapsed(Duration::from_millis(6_080)),
            GlintFrame(1)
        );
    }

    #[test]
    fn ready_is_a_compact_right_aligned_symbol_badge() {
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
        assert!(text.ends_with(" ✓ "));
        assert_eq!(
            badge.bg,
            Color::Rgb {
                r: 55,
                g: 82,
                b: 65
            }
        );
        assert_eq!(
            badge.fg,
            Color::Rgb {
                r: 184,
                g: 219,
                b: 196
            }
        );

        for (width, expected) in [(1, "✓"), (2, " ✓"), (3, " ✓ ")] {
            let spans = ready_badge_spans("codex", width, base_bg, base_fg);
            let text: String = spans.iter().map(|span| span.text.as_str()).collect();
            assert_eq!(text, expected);
        }

        for width in [1, 2, 3, 10, 18] {
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
                key: index as u64,
                primary: format!("tab-{index}"),
                secondary: String::new(),
                agent: (index == 0).then_some(AgentStatus::single(
                    crate::agent::AgentKind::Codex,
                    AgentState::Working,
                )),
                glint_frame: (index == 0).then_some(GlintFrame(10)),
                active: index == 6,
            })
            .collect();
        let layout = Layout::new(40, 5, true, 18);
        let mut out = Vec::new();
        let mut cache = SidebarCache::default();

        let map = draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
        assert!(map.visible_glints().is_empty());
    }

    #[test]
    fn narrow_working_card_is_static_and_does_not_schedule_animation() {
        let tabs = [SidebarTab {
            key: 1,
            primary: "codex".into(),
            secondary: "~/mux".into(),
            agent: Some(AgentStatus::single(
                crate::agent::AgentKind::Codex,
                AgentState::Working,
            )),
            glint_frame: Some(GlintFrame(20)),
            active: true,
        }];
        let layout = Layout::new(25, 12, true, 5);
        let mut out = Vec::new();
        let mut cache = SidebarCache::default();

        let map = draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
        assert!(map.visible_glints().is_empty());
        assert!(cache.rows[1].spans.iter().all(|span| {
            span.bg
                == Color::Rgb {
                    r: 56,
                    g: 56,
                    b: 56,
                }
        }));
    }
}
