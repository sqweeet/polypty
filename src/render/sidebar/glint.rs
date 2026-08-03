use std::time::Duration;

use crossterm::style::Color;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::cache::SidebarPaintSpan;
use super::text::pad_fit;

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

pub(super) fn working_glint_bg(
    active: bool,
    frame: GlintFrame,
    column: usize,
    width: usize,
) -> Color {
    let (base, target) = if active { (48u8, 72u8) } else { (36u8, 54u8) };
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

pub(super) fn sidebar_paint_spans(
    label: &str,
    width: usize,
    base_bg: Color,
    base_fg: Color,
    glint: Option<(bool, GlintFrame)>,
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
