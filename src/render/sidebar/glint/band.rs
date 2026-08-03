use crossterm::style::Color;

use super::GlintFrame;

/// Where a card row sits inside the one slanted band shared by the whole card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render::sidebar) enum GlintRow {
    Flat,
    Upper,
    Lower,
}

/// Terminal cells are about twice as tall as wide, so a two-column shift per
/// text row reads as a ~45° slant on screen.
const SLANT: f32 = 2.0;
const CELL_ASPECT: f32 = 2.0;

impl GlintRow {
    fn offset(self) -> f32 {
        match self {
            Self::Flat => 0.0,
            Self::Upper => -SLANT / 2.0,
            Self::Lower => SLANT / 2.0,
        }
    }
}

/// Horizontal widening that keeps a slanted band as thick along its own normal
/// as a flat one; without it the slant reads as a thinner, weaker streak.
fn stretch() -> f32 {
    (1.0 + (SLANT / CELL_ASPECT).powi(2)).sqrt()
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
    // Clearing `reach` on both ends lands the first and last sweep frame exactly
    // on `base` for every row, so the sweep joins REST without a jump and both
    // rows enter and leave together as one continuous band.
    let reach = radius * stretch() + SLANT / 2.0;
    let travel = width.saturating_sub(1) as f32 + reach * 2.0;
    let center = -reach + progress * travel + row.offset();
    let distance = (column as f32 - center).abs() / stretch();
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
