mod button;

use std::io::Write;

use anyhow::Result;
use crossterm::cursor::{Hide, MoveTo};
use crossterm::queue;
use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};

use crate::render::fade::blend;

use super::{DialogGeometry, ExitDialogButton};
use button::{paint_button, ButtonStyle};

const CARD_BG: Color = rgb(36, 36, 36);

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

pub(super) fn draw(
    out: &mut impl Write,
    geometry: DialogGeometry,
    exit_selected: bool,
    opacity: u8,
    pressed: Option<ExitDialogButton>,
    press_opacity: u8,
    selection_opacity: (u8, u8),
) -> Result<()> {
    let surface = blend(rgb(28, 28, 28), CARD_BG, opacity);
    let blank = " ".repeat(usize::from(geometry.width));
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
        paint_centered(
            out,
            geometry,
            geometry.y + 1,
            "Exit polypty?",
            true,
            opacity,
        )?;
    }
    if geometry.height >= 5 {
        paint_centered(
            out,
            geometry,
            geometry.y + 2,
            "Running tabs will be stopped.",
            false,
            opacity,
        )?;
    }
    if geometry.width >= 16 && geometry.height >= 2 {
        paint_button(
            out,
            geometry.cancel_x,
            geometry.button_y,
            " Cancel ",
            ButtonStyle {
                selected: !exit_selected,
                danger: false,
                pressed: pressed == Some(ExitDialogButton::Cancel),
                press_opacity,
                selection_opacity: selection_opacity.0,
                opacity,
            },
        )?;
        paint_button(
            out,
            geometry.exit_x,
            geometry.button_y,
            " Exit ",
            ButtonStyle {
                selected: exit_selected,
                danger: true,
                pressed: pressed == Some(ExitDialogButton::Exit),
                press_opacity,
                selection_opacity: selection_opacity.1,
                opacity,
            },
        )?;
    }
    queue!(out, ResetColor, SetAttribute(Attribute::Reset), Hide)?;
    Ok(())
}

fn paint_centered(
    out: &mut impl Write,
    geometry: DialogGeometry,
    row: u16,
    text: &str,
    bold: bool,
    opacity: u8,
) -> Result<()> {
    let surface = blend(rgb(28, 28, 28), CARD_BG, opacity);
    let text = &text[..text.len().min(usize::from(geometry.width))];
    let x = geometry.x + geometry.width.saturating_sub(text.len() as u16) / 2;
    queue!(
        out,
        MoveTo(x, row),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(blend(surface, rgb(210, 210, 210), opacity)),
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
