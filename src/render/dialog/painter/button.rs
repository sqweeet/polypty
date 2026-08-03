use std::io::Write;

use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
};

use crate::render::fade::blend;

const CARD_BG: Color = rgb(36, 36, 36);
const BUTTON_BG: Color = rgb(48, 48, 48);
const SELECTED_BG: Color = rgb(68, 68, 68);
const PRESSED_BG: Color = rgb(84, 84, 84);
const DANGER: Color = rgb(220, 105, 105);
const DANGER_BG: Color = rgb(54, 34, 34);
const DANGER_SELECTED_BG: Color = rgb(104, 48, 48);
const DANGER_PRESSED_BG: Color = rgb(124, 54, 54);

pub(super) struct ButtonStyle {
    pub(super) selected: bool,
    pub(super) danger: bool,
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
    let idle_foreground = if style.danger {
        DANGER
    } else {
        rgb(145, 145, 145)
    };
    let foreground = blend(idle_foreground, rgb(225, 225, 225), style.selection_opacity);
    let idle_background = if style.danger { DANGER_BG } else { BUTTON_BG };
    let selected_background = if style.danger {
        DANGER_SELECTED_BG
    } else {
        SELECTED_BG
    };
    let normal_background = blend(
        idle_background,
        selected_background,
        style.selection_opacity,
    );
    let pressed_background = if style.danger {
        DANGER_PRESSED_BG
    } else {
        PRESSED_BG
    };
    let background = if style.pressed {
        blend(normal_background, pressed_background, style.press_opacity)
    } else {
        normal_background
    };
    let surface = blend(rgb(28, 28, 28), CARD_BG, style.opacity);
    queue!(
        out,
        MoveTo(x, y),
        SetForegroundColor(blend(surface, foreground, style.opacity)),
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

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}
