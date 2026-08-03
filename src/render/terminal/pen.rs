use std::io::Write;

use anyhow::Result;
use crossterm::cursor::MoveTo;
use crossterm::queue;
use crossterm::style::{
    Attribute, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};

use super::cell::{PaintCell, ATTR_BOLD, ATTR_DIM, ATTR_INVERSE, ATTR_ITALIC, ATTR_UNDERLINE};
use super::color::PackedColor;

pub(super) struct PaintPen {
    fg: PackedColor,
    bg: PackedColor,
    attrs: u8,
    x: i32,
    y: i32,
}

impl PaintPen {
    pub(super) fn new() -> Self {
        Self {
            fg: PackedColor::INVALID,
            bg: PackedColor::INVALID,
            attrs: u8::MAX,
            x: -1,
            y: -1,
        }
    }

    pub(super) fn paint(
        &mut self,
        out: &mut impl Write,
        x: u16,
        y: u16,
        cell: &PaintCell,
    ) -> Result<()> {
        if self.x != i32::from(x) || self.y != i32::from(y) {
            queue!(out, MoveTo(x, y))?;
        }
        if cell.fg != self.fg || cell.bg != self.bg || cell.attrs != self.attrs {
            self.apply_style(out, cell)?;
        }

        let text = if cell.text.is_empty() {
            " "
        } else {
            cell.text.as_str()
        };
        queue!(out, Print(text))?;
        self.x = i32::from(x) + i32::from(cell.width);
        self.y = i32::from(y);
        Ok(())
    }

    fn apply_style(&mut self, out: &mut impl Write, cell: &PaintCell) -> Result<()> {
        queue!(out, ResetColor, SetAttribute(Attribute::Reset))?;
        if let Some(color) = cell.fg.to_crossterm() {
            queue!(out, SetForegroundColor(color))?;
        }
        if let Some(color) = cell.bg.to_crossterm() {
            queue!(out, SetBackgroundColor(color))?;
        }
        for (mask, attribute) in [
            (ATTR_BOLD, Attribute::Bold),
            (ATTR_DIM, Attribute::Dim),
            (ATTR_ITALIC, Attribute::Italic),
            (ATTR_UNDERLINE, Attribute::Underlined),
            (ATTR_INVERSE, Attribute::Reverse),
        ] {
            if cell.attrs & mask != 0 {
                queue!(out, SetAttribute(attribute))?;
            }
        }
        self.fg = cell.fg;
        self.bg = cell.bg;
        self.attrs = cell.attrs;
        Ok(())
    }
}
