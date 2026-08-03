use crossterm::style::Color;

use super::GlintFrame;

/// Which row of a card is being painted. Every row uses the same horizontal
/// profile so the highlight stays vertical across the whole card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render::sidebar) enum GlintRow {
    Flat,
    Upper,
    Lower,
}

pub(in crate::render::sidebar) fn working_glint_bg(
    active: bool,
    frame: GlintFrame,
    column: usize,
    width: usize,
    _row: GlintRow,
) -> Color {
    let (base, target) = if active { (48u8, 68u8) } else { (36u8, 50u8) };
    if width < 6 {
        return gray(if active { 56 } else { 42 });
    }
    let Some(progress) = frame.progress() else {
        return gray(base);
    };
    // A broad radius gives the low-contrast highlight enough room to fade over
    // several terminal cells instead of reading as a hard moving stripe.
    let radius = (width as f32 / 3.75).clamp(2.75, 5.0);
    // Clearing one full radius at both ends makes the sweep meet REST at base.
    let travel = width.saturating_sub(1) as f32 + radius * 2.0;
    let center = -radius + progress * travel;
    let distance = (column as f32 - center).abs();
    let weight = smooth_band(distance, radius);
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
