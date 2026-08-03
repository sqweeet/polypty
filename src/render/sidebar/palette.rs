use crossterm::style::Color;

use crate::agent::AgentState;

use super::badge::ready_badge_spans;
use super::cache::SidebarPaintRow;
use super::glint::{sidebar_paint_spans, GlintFrame};
use super::model::SidebarContentRow;

pub(super) struct SidebarPalette {
    background: Color,
    active_background: Color,
    active: Color,
    idle: Color,
    secondary_active: Color,
    secondary_idle: Color,
    blocked: Color,
}

impl SidebarPalette {
    pub(super) fn new() -> Self {
        Self {
            background: rgb(36),
            active_background: rgb(48),
            active: rgb(188),
            idle: rgb(120),
            secondary_active: rgb(130),
            secondary_idle: rgb(96),
            blocked: Color::Rgb {
                r: 220,
                g: 105,
                b: 105,
            },
        }
    }

    pub(super) fn paint(&self, row: &SidebarContentRow, width: usize) -> SidebarPaintRow {
        let background = if row.active && row.kind != 0 {
            self.active_background
        } else {
            self.background
        };
        let foreground = match row.kind {
            2 | 6 | 7 if row.active => self.active,
            2 | 5 | 6 | 7 => self.idle,
            3 if row.active => self.secondary_active,
            3 | 4 => self.secondary_idle,
            8 => self.blocked,
            _ => self.idle,
        };
        let spans = if row.kind == 6 {
            ready_badge_spans(&row.text, width, background, foreground)
        } else {
            let working = row.agent_state == Some(AgentState::Working);
            sidebar_paint_spans(
                &row.text,
                width,
                background,
                foreground,
                working.then_some((row.active, row.glint_frame.unwrap_or(GlintFrame::REST))),
            )
        };
        SidebarPaintRow { spans }
    }
}

fn rgb(value: u8) -> Color {
    Color::Rgb {
        r: value,
        g: value,
        b: value,
    }
}
