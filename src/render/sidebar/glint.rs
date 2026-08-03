mod band;

use std::time::Duration;

use crossterm::style::Color;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::cache::SidebarPaintSpan;
use super::text::pad_fit;
pub(super) use band::{working_glint_bg, GlintRow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlintFrame(pub(super) u8);

impl GlintFrame {
    const FRAME_MS: u128 = 80;
    const SWEEP_FRAMES: u128 = 50;
    const CYCLE_FRAMES: u128 = 75;
    pub(super) const REST: Self = Self(Self::SWEEP_FRAMES as u8);

    pub(crate) fn for_elapsed(elapsed: Duration) -> Self {
        let frame = (elapsed.as_millis() / Self::FRAME_MS) % Self::CYCLE_FRAMES;
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

pub(super) fn sidebar_paint_spans(
    label: &str,
    width: usize,
    base_bg: Color,
    base_fg: Color,
    glint: Option<(bool, GlintFrame, GlintRow)>,
) -> Vec<SidebarPaintSpan> {
    let padded = pad_fit(label, width);
    let mut spans: Vec<SidebarPaintSpan> = Vec::new();
    let mut column = 0;
    for grapheme in UnicodeSegmentation::graphemes(padded.as_str(), true) {
        let grapheme_width = UnicodeWidthStr::width(grapheme);
        if grapheme_width == 0 {
            if let Some(span) = spans.last_mut() {
                span.text.push_str(grapheme);
            }
            continue;
        }
        let bg = glint
            .map(|(active, frame, row)| {
                shifted_glint(
                    base_bg,
                    working_glint_bg(active, frame, column, width, row),
                    active,
                )
            })
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

fn shifted_glint(base: Color, glint: Color, active: bool) -> Color {
    let standard = if active { 48 } else { 36 };
    let (
        Color::Rgb { r, g, b },
        Color::Rgb {
            r: gr,
            g: gg,
            b: gb,
        },
    ) = (base, glint)
    else {
        return glint;
    };
    Color::Rgb {
        r: shift_channel(r, gr, standard),
        g: shift_channel(g, gg, standard),
        b: shift_channel(b, gb, standard),
    }
}

fn shift_channel(base: u8, glint: u8, standard: u8) -> u8 {
    if glint >= standard {
        base.saturating_add(glint - standard)
    } else {
        base.saturating_sub(standard - glint)
    }
}
