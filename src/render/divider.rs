use std::collections::BTreeMap;
use std::io::Write;

use crate::core::geometry::Divider;
use anyhow::Result;
use crossterm::cursor::{Hide, MoveTo};
use crossterm::queue;
use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};

const UP: u8 = 1;
const DOWN: u8 = 2;
const LEFT: u8 = 4;
const RIGHT: u8 = 8;

pub fn draw_dividers(out: &mut impl Write, dividers: &[Divider]) -> Result<()> {
    if dividers.is_empty() {
        return Ok(());
    }

    let cells = connected_cells(dividers);
    let fg = Color::Rgb {
        r: 76,
        g: 76,
        b: 76,
    };
    let bg = Color::Rgb {
        r: 21,
        g: 21,
        b: 21,
    };
    queue!(out, Hide, SetForegroundColor(fg), SetBackgroundColor(bg))?;
    for ((x, y), links) in cells {
        queue!(out, MoveTo(x, y), Print(glyph(links)))?;
    }
    queue!(out, ResetColor, SetAttribute(Attribute::Reset))?;
    Ok(())
}

fn connected_cells(dividers: &[Divider]) -> BTreeMap<(u16, u16), u8> {
    let mut cells = BTreeMap::new();
    for divider in dividers {
        match *divider {
            Divider::Vertical { x, y, len } => {
                for offset in 0..len {
                    *cells.entry((x, y.saturating_add(offset))).or_default() |= UP | DOWN;
                }
            }
            Divider::Horizontal { x, y, len } => {
                for offset in 0..len {
                    *cells.entry((x.saturating_add(offset), y)).or_default() |= LEFT | RIGHT;
                }
            }
        }
    }

    let positions: Vec<_> = cells.keys().copied().collect();
    for (x, y) in positions {
        let mut links = cells[&(x, y)];
        if x > 0 && cells.contains_key(&(x - 1, y)) {
            links |= LEFT;
        }
        if cells.contains_key(&(x.saturating_add(1), y)) {
            links |= RIGHT;
        }
        if y > 0 && cells.contains_key(&(x, y - 1)) {
            links |= UP;
        }
        if cells.contains_key(&(x, y.saturating_add(1))) {
            links |= DOWN;
        }
        cells.insert((x, y), links);
    }
    cells
}

fn glyph(links: u8) -> char {
    match links {
        bits if bits == UP | DOWN | LEFT | RIGHT => '┼',
        bits if bits == UP | DOWN | RIGHT => '├',
        bits if bits == UP | DOWN | LEFT => '┤',
        bits if bits == LEFT | RIGHT | DOWN => '┬',
        bits if bits == LEFT | RIGHT | UP => '┴',
        bits if bits == DOWN | RIGHT => '┌',
        bits if bits == DOWN | LEFT => '┐',
        bits if bits == UP | RIGHT => '└',
        bits if bits == UP | LEFT => '┘',
        bits if bits & (LEFT | RIGHT) != 0 && bits & (UP | DOWN) == 0 => '─',
        bits if bits & (UP | DOWN) != 0 && bits & (LEFT | RIGHT) == 0 => '│',
        bits if bits & (LEFT | RIGHT) != 0 => '─',
        _ => '│',
    }
}

#[cfg(test)]
mod tests;
