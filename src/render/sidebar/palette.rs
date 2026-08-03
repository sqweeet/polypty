use std::borrow::Cow;

use crossterm::style::Color;

use crate::agent::AgentState;
use crate::render::fade::blend;

use super::badge::{blocked_badge_spans, ready_badge_spans};
use super::cache::SidebarPaintRow;
use super::glint::{sidebar_paint_spans, GlintFrame};
use super::model::SidebarContentRow;

pub(super) struct SidebarPalette {
    background: Color,
    active_background: Color,
    hover_background: Color,
    pressed_background: Color,
    active: Color,
    idle: Color,
    secondary_active: Color,
    secondary_idle: Color,
}

impl SidebarPalette {
    pub(super) fn new() -> Self {
        Self {
            background: rgb(36),
            active_background: rgb(48),
            hover_background: rgb(56),
            pressed_background: rgb(68),
            active: rgb(188),
            idle: rgb(120),
            secondary_active: rgb(130),
            secondary_idle: rgb(96),
        }
    }

    pub(super) fn paint(&self, row: &SidebarContentRow, width: usize) -> SidebarPaintRow {
        let label = if row.tab_idx.is_some() && width > 1 {
            Cow::Owned(format!(" {}", row.text))
        } else {
            Cow::Borrowed(row.text.as_str())
        };
        let background = self.background(row);
        let emphasis = row
            .visual
            .active
            .max(row.visual.hover / 2)
            .max(row.visual.press.saturating_mul(3) / 4);
        let foreground = match row.kind {
            2 | 6 | 7 | 8 => blend(self.idle, self.active, emphasis),
            5 => self.idle,
            3 => blend(self.secondary_idle, self.secondary_active, emphasis),
            4 => self.secondary_idle,
            _ => self.idle,
        };
        let spans = if row.kind == 6 {
            ready_badge_spans(&label, width, background, foreground)
        } else if row.kind == 8 {
            blocked_badge_spans(&label, width, background, foreground)
        } else {
            let working = row.agent_state == Some(AgentState::Working);
            sidebar_paint_spans(
                &label,
                width,
                background,
                foreground,
                working.then_some((
                    row.active,
                    row.glint_frame.unwrap_or(GlintFrame::REST),
                    row.glint_row,
                )),
            )
        };
        SidebarPaintRow { spans }
    }

    fn background(&self, row: &SidebarContentRow) -> Color {
        if row.kind == 0 {
            return self.background;
        }
        let active = blend(self.background, self.active_background, row.visual.active);
        let hovered = blend(active, self.hover_background, row.visual.hover);
        blend(hovered, self.pressed_background, row.visual.press)
    }
}

fn rgb(value: u8) -> Color {
    Color::Rgb {
        r: value,
        g: value,
        b: value,
    }
}
