use vt100::Cell;

use super::color::PackedColor;

pub(super) const ATTR_BOLD: u8 = 1;
pub(super) const ATTR_ITALIC: u8 = 2;
pub(super) const ATTR_UNDERLINE: u8 = 4;
pub(super) const ATTR_INVERSE: u8 = 8;
pub(super) const ATTR_DIM: u8 = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PaintCell {
    pub(super) text: String,
    pub(super) width: u8,
    pub(super) fg: PackedColor,
    pub(super) bg: PackedColor,
    pub(super) attrs: u8,
}

impl PaintCell {
    pub(super) fn blank() -> Self {
        Self {
            text: " ".into(),
            width: 1,
            fg: PackedColor::DEFAULT,
            bg: PackedColor::DEFAULT,
            attrs: 0,
        }
    }

    pub(super) fn continuation(fg: PackedColor, bg: PackedColor, attrs: u8) -> Self {
        Self {
            text: String::new(),
            width: 0,
            fg,
            bg,
            attrs,
        }
    }
}

pub(super) fn cell_to_paint(cell: Option<&Cell>) -> PaintCell {
    let Some(cell) = cell else {
        return PaintCell::blank();
    };
    if cell.is_wide_continuation() {
        return PaintCell::continuation(PackedColor::DEFAULT, PackedColor::DEFAULT, 0);
    }

    let contents = cell.contents();
    let text = if contents.is_empty() {
        " ".to_string()
    } else {
        contents.to_owned()
    };
    let width = if cell.is_wide() { 2 } else { 1 };
    let mut attrs = 0;
    if cell.bold() {
        attrs |= ATTR_BOLD;
    }
    if cell.dim() {
        attrs |= ATTR_DIM;
    }
    if cell.italic() {
        attrs |= ATTR_ITALIC;
    }
    if cell.underline() {
        attrs |= ATTR_UNDERLINE;
    }
    if cell.inverse() {
        attrs |= ATTR_INVERSE;
    }

    PaintCell {
        text,
        width,
        fg: PackedColor::from_vt(cell.fgcolor()),
        bg: PackedColor::from_vt(cell.bgcolor()),
        attrs,
    }
}
