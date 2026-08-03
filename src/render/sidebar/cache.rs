use crossterm::style::Color;

/// Last painted sidebar rows, retained for row-level diffs.
#[derive(Debug, Clone, Default)]
pub struct SidebarCache {
    pub(super) width: u16,
    pub(super) rows: Vec<SidebarPaintRow>,
}

impl SidebarCache {
    pub fn invalidate(&mut self) {
        self.rows.clear();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SidebarPaintRow {
    pub(super) spans: Vec<SidebarPaintSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SidebarPaintSpan {
    pub(super) text: String,
    pub(super) bg: Color,
    pub(super) fg: Color,
}
