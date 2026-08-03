use crossterm::style::Color;

use super::cache::SidebarPaintSpan;
use super::glint::sidebar_paint_spans;

pub(super) fn ready_badge_spans(
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
