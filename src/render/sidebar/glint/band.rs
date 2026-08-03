use crossterm::style::Color;

use super::GlintFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render::sidebar) enum GlintRow {
    Flat,
    Upper,
    Lower,
}

impl GlintRow {
    fn offset(self, width: usize) -> f32 {
        let slope = (width as f32 / 6.0).clamp(1.5, 3.0);
        match self {
            Self::Flat => 0.0,
            Self::Upper => -slope / 2.0,
            Self::Lower => slope / 2.0,
        }
    }
}

pub(in crate::render::sidebar) fn working_glint_bg(
    active: bool,
    frame: GlintFrame,
    column: usize,
    width: usize,
    row: GlintRow,
) -> Color {
    let (base, target) = if active { (48u8, 72u8) } else { (36u8, 54u8) };
    if width < 6 {
        return gray(if active { 56 } else { 42 });
    }
    let Some(progress) = frame.progress() else {
        return gray(base);
    };
    let radius = (width as f32 / 5.5).clamp(1.5, 3.25);
    let travel = width.saturating_sub(1) as f32 + radius * 2.0;
    let center = -radius + progress * travel + row.offset(width);
    let distance = (column as f32 - center).abs();
    let feather = smooth_band(distance, radius);
    let core = smooth_band(distance, (radius * 0.42).max(0.75));
    let weight = feather * 0.65 + core * 0.35;
    gray((f32::from(base) + f32::from(target - base) * weight).round() as u8)
}

fn smooth_band(distance: f32, radius: f32) -> f32 {
    let linear = (1.0 - distance / radius).clamp(0.0, 1.0);
    linear * linear * (3.0 - 2.0 * linear)
}

const fn gray(value: u8) -> Color {
    Color::Rgb {
        r: value,
        g: value,
        b: value,
    }
}
