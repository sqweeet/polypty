mod button;

use std::io::Write;

use anyhow::Result;
use crossterm::{
    cursor::{Hide, MoveTo},
    queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
};

use crate::render::fade::blend;

use super::{ShortcutDialogGeometry, ShortcutDialogView, ShortcutScope};
use button::{paint_button, ButtonStyle};

const CARD_BG: Color = rgb(36, 36, 36);
const TEXT: Color = rgb(210, 210, 210);
const MUTED: Color = rgb(150, 150, 150);
const ERROR: Color = rgb(220, 105, 105);

pub(super) fn draw(
    out: &mut impl Write,
    geometry: ShortcutDialogGeometry,
    view: ShortcutDialogView,
) -> Result<()> {
    let blank = " ".repeat(usize::from(geometry.width));
    let surface = blend(rgb(28, 28, 28), CARD_BG, view.opacity);
    queue!(out, Hide, SetAttribute(Attribute::Reset))?;
    for offset in 0..geometry.height {
        queue!(
            out,
            MoveTo(geometry.x, geometry.y + offset),
            SetBackgroundColor(surface),
            Print(&blank)
        )?;
    }
    if geometry.height >= 3 {
        let title = if view.desired_visible {
            "Show shortcuts?"
        } else {
            "Hide shortcuts?"
        };
        paint_centered(
            out,
            geometry,
            geometry.y + 1,
            title,
            TEXT,
            true,
            view.opacity,
        )?;
    }
    if geometry.height >= 5 {
        let (message, color) = if view.save_failed {
            ("Could not save config.", ERROR)
        } else {
            ("Choose Session or Always.", MUTED)
        };
        paint_centered(
            out,
            geometry,
            geometry.y + 2,
            message,
            color,
            false,
            view.opacity,
        )?;
    }
    if geometry.width >= 19 && geometry.height >= 2 {
        paint_button(
            out,
            geometry.session_x,
            geometry.button_y,
            " Session ",
            ButtonStyle {
                selected: view.selected == ShortcutScope::Session,
                pressed: view.pressed == Some(ShortcutScope::Session),
                press_opacity: view.press_opacity,
                selection_opacity: view.session_opacity,
                opacity: view.opacity,
            },
        )?;
        paint_button(
            out,
            geometry.always_x,
            geometry.button_y,
            " Always ",
            ButtonStyle {
                selected: view.selected == ShortcutScope::Always,
                pressed: view.pressed == Some(ShortcutScope::Always),
                press_opacity: view.press_opacity,
                selection_opacity: view.always_opacity,
                opacity: view.opacity,
            },
        )?;
    }
    queue!(out, ResetColor, SetAttribute(Attribute::Reset), Hide)?;
    Ok(())
}

fn paint_centered(
    out: &mut impl Write,
    geometry: ShortcutDialogGeometry,
    row: u16,
    text: &str,
    color: Color,
    bold: bool,
    opacity: u8,
) -> Result<()> {
    let surface = blend(rgb(28, 28, 28), CARD_BG, opacity);
    let text = &text[..text.len().min(usize::from(geometry.width))];
    let x = geometry.x + geometry.width.saturating_sub(text.len() as u16) / 2;
    queue!(
        out,
        MoveTo(x, row),
        SetForegroundColor(blend(surface, color, opacity)),
        SetBackgroundColor(surface),
        SetAttribute(if bold {
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
