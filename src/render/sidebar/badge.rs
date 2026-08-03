use crossterm::style::Color;

use super::cache::SidebarPaintSpan;
use super::glint::sidebar_paint_spans;

#[derive(Clone, Copy)]
struct BadgeStyle {
    symbol: char,
    background: Color,
    foreground: Color,
}

pub(super) fn ready_badge_spans(
    label: &str,
    width: usize,
    base_bg: Color,
    base_fg: Color,
) -> Vec<SidebarPaintSpan> {
    symbol_badge_spans(
        label,
        width,
        base_bg,
        base_fg,
        BadgeStyle {
            symbol: '✓',
            background: Color::Rgb {
                r: 55,
                g: 82,
                b: 65,
            },
            foreground: Color::Rgb {
                r: 184,
                g: 219,
                b: 196,
            },
        },
    )
}

pub(super) fn blocked_badge_spans(
    label: &str,
    width: usize,
    base_bg: Color,
    base_fg: Color,
) -> Vec<SidebarPaintSpan> {
    symbol_badge_spans(
        label,
        width,
        base_bg,
        base_fg,
        BadgeStyle {
            symbol: '!',
            background: Color::Rgb {
                r: 82,
                g: 72,
                b: 42,
            },
            foreground: Color::Rgb {
                r: 238,
                g: 207,
                b: 105,
            },
        },
    )
}

fn symbol_badge_spans(
    label: &str,
    width: usize,
    base_bg: Color,
    base_fg: Color,
    style: BadgeStyle,
) -> Vec<SidebarPaintSpan> {
    const BADGE_WIDTH: usize = 3;

    if width < BADGE_WIDTH {
        let mut text = " ".repeat(width.saturating_sub(1));
        if width > 0 {
            text.push(style.symbol);
        }
        return vec![SidebarPaintSpan {
            text,
            bg: style.background,
            fg: style.foreground,
        }];
    }
    if width == BADGE_WIDTH {
        return vec![SidebarPaintSpan {
            text: format!(" {} ", style.symbol),
            bg: style.background,
            fg: style.foreground,
        }];
    }

    let mut spans = sidebar_paint_spans(label, width - BADGE_WIDTH, base_bg, base_fg, None);
    spans.push(SidebarPaintSpan {
        text: format!(" {} ", style.symbol),
        bg: style.background,
        fg: style.foreground,
    });
    spans
}
