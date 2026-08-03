use std::io::Write;

use anyhow::Result;
use crossterm::cursor::{Hide, MoveTo};
use crossterm::queue;
use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};

use super::DialogGeometry;

const CARD_BG: Color = rgb(36, 36, 36);
const BUTTON_BG: Color = rgb(48, 48, 48);
const SELECTED_BG: Color = rgb(68, 68, 68);
const DANGER: Color = rgb(220, 105, 105);
const DANGER_BG: Color = rgb(54, 34, 34);
const DANGER_SELECTED_BG: Color = rgb(104, 48, 48);

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

pub(super) fn draw(
    out: &mut impl Write,
    geometry: DialogGeometry,
    exit_selected: bool,
) -> Result<()> {
    let blank = " ".repeat(usize::from(geometry.width));
    queue!(out, Hide, SetAttribute(Attribute::Reset))?;
    for offset in 0..geometry.height {
        queue!(
            out,
            MoveTo(geometry.x, geometry.y + offset),
            SetBackgroundColor(CARD_BG),
            Print(&blank)
        )?;
    }
    if geometry.height >= 3 {
        paint_centered(out, geometry, geometry.y + 1, "Exit mux?", true)?;
    }
    if geometry.height >= 5 {
        paint_centered(
            out,
            geometry,
            geometry.y + 2,
            "Running tabs will be stopped.",
            false,
        )?;
    }
    if geometry.width >= 16 && geometry.height >= 2 {
        paint_button(
            out,
            geometry.cancel_x,
            geometry.button_y,
            " Cancel ",
            !exit_selected,
            false,
        )?;
        paint_button(
            out,
            geometry.exit_x,
            geometry.button_y,
            " Exit ",
            exit_selected,
            true,
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
) -> Result<()> {
    let text = &text[..text.len().min(usize::from(geometry.width))];
    let x = geometry.x + geometry.width.saturating_sub(text.len() as u16) / 2;
    queue!(
        out,
        MoveTo(x, row),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(Color::Rgb {
            r: 210,
            g: 210,
            b: 210
        }),
        SetBackgroundColor(CARD_BG),
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

fn paint_button(
    out: &mut impl Write,
    x: u16,
    y: u16,
    text: &str,
    selected: bool,
    danger: bool,
) -> Result<()> {
    let foreground = if selected {
        Color::Rgb {
            r: 225,
            g: 225,
            b: 225,
        }
    } else if danger {
        DANGER
    } else {
        Color::Rgb {
            r: 145,
            g: 145,
            b: 145,
        }
    };
    let background = if selected && danger {
        DANGER_SELECTED_BG
    } else if selected {
        SELECTED_BG
    } else if danger {
        DANGER_BG
    } else {
        BUTTON_BG
    };
    queue!(
        out,
        MoveTo(x, y),
        SetForegroundColor(foreground),
        SetBackgroundColor(background),
        SetAttribute(if selected {
            Attribute::Bold
        } else {
            Attribute::NormalIntensity
        }),
        Print(text),
        SetAttribute(Attribute::Reset)
    )?;
    Ok(())
}
