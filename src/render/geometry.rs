/// Geometry of the mux chrome + terminal pane.
#[derive(Debug, Clone, Copy)]
pub struct Layout {
    #[allow(dead_code)]
    pub cols: u16,
    pub rows: u16,
    pub sidebar_visible: bool,
    pub sidebar_width: u16,
    pub term_x: u16,
    pub term_y: u16,
    pub term_cols: u16,
    pub term_rows: u16,
}

impl Layout {
    pub fn new(cols: u16, rows: u16, sidebar_visible: bool, sidebar_width: u16) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let sidebar_width = if sidebar_visible {
            let max_width = cols.saturating_sub(20);
            sidebar_width
                .min(max_width)
                .max(1)
                .min(cols.saturating_sub(1))
        } else {
            0
        };
        let term_x = sidebar_width;

        Self {
            cols,
            rows,
            sidebar_visible: sidebar_width > 0,
            sidebar_width,
            term_x,
            term_y: 0,
            term_cols: cols.saturating_sub(term_x).max(1),
            term_rows: rows,
        }
    }

    pub fn terminal_rect(&self) -> TerminalRect {
        TerminalRect {
            x: self.term_x,
            y: self.term_y,
            cols: self.term_cols,
            rows: self.term_rows,
        }
    }
}

use crate::core::geometry::TerminalRect;
