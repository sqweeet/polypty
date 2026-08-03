use std::io::Write;

use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
};

use crate::render::fade::blend;

const CARD_BG: Color = rgb(36);
const BUTTON_BG: Color = rgb(48);
const SELECTED_BG: Color = rgb(68);
const PRESSED_BG: Color = rgb(84);
const TEXT: Color = rgb(210);
const MUTED: Color = rgb(150);

pub(super) struct ButtonStyle {
    pub(super) selected: bool,
    pub(super) pressed: bool,
    pub(super) press_opacity: u8,
    pub(super) selection_opacity: u8,
    pub(super) opacity: u8,
}

pub(super) fn paint_button(
    out: &mut impl Write,
    x: u16,
    y: u16,
    text: &str,
    style: ButtonStyle,
) -> Result<()> {
    let normal_background = blend(BUTTON_BG, SELECTED_BG, style.selection_opacity);
    let background = if style.pressed {
        blend(normal_background, PRESSED_BG, style.press_opacity)
    } else {
        normal_background
    };
    let surface = blend(rgb(28), CARD_BG, style.opacity);
    queue!(
        out,
        MoveTo(x, y),
        SetForegroundColor(blend(
            surface,
            blend(MUTED, TEXT, style.selection_opacity),
            style.opacity
        )),
        SetBackgroundColor(blend(surface, background, style.opacity)),
        SetAttribute(if style.selected {
            Attribute::Bold
        } else {
            Attribute::NormalIntensity
        }),
        Print(text),
        SetAttribute(Attribute::Reset)
    )?;
    Ok(())
}

const fn rgb(value: u8) -> Color {
    Color::Rgb {
        r: value,
        g: value,
        b: value,
    }
}
