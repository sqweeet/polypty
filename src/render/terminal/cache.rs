use super::cell::PaintCell;

/// Last painted terminal pane, retained for cell-level diffs.
#[derive(Debug, Clone)]
pub struct TermCache {
    pub(super) cols: u16,
    pub(super) rows: u16,
    pub(super) cells: Vec<PaintCell>,
    pub(super) cursor: (u16, u16),
    pub(super) cursor_hidden: bool,
    pub(super) valid_cells: Vec<bool>,
}

impl Default for TermCache {
    fn default() -> Self {
        Self {
            cols: 0,
            rows: 0,
            cells: Vec::new(),
            cursor: (0, 0),
            cursor_hidden: true,
            valid_cells: Vec::new(),
        }
    }
}

impl TermCache {
    pub fn invalidate(&mut self) {
        self.valid_cells.fill(false);
    }

    /// Establish the known screen state after a reset-color + full clear.
    pub fn reset_blank(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
        let len = usize::from(cols) * usize::from(rows);
        self.cells = vec![PaintCell::blank(); len];
        self.valid_cells = vec![true; len];
    }

    pub(super) fn ensure(&mut self, cols: u16, rows: u16) {
        let len = usize::from(cols) * usize::from(rows);
        if self.cols == cols
            && self.rows == rows
            && self.cells.len() == len
            && self.valid_cells.len() == len
        {
            return;
        }

        let old_cols = self.cols;
        let old_rows = self.rows;
        let old_cells = std::mem::take(&mut self.cells);
        let old_valid = std::mem::take(&mut self.valid_cells);
        self.cols = cols;
        self.rows = rows;
        self.cells = vec![PaintCell::blank(); len];
        self.valid_cells = vec![false; len];

        let copy_cols = old_cols.min(cols);
        let copy_rows = old_rows.min(rows);
        for row in 0..copy_rows {
            for col in 0..copy_cols {
                let old_idx = usize::from(row) * usize::from(old_cols) + usize::from(col);
                let new_idx = usize::from(row) * usize::from(cols) + usize::from(col);
                if let Some(cell) = old_cells.get(old_idx) {
                    self.cells[new_idx] = cell.clone();
                    self.valid_cells[new_idx] = old_valid.get(old_idx).copied().unwrap_or(false);
                }
            }
        }
    }

    pub(super) fn idx(&self, row: u16, col: u16) -> usize {
        usize::from(row) * usize::from(self.cols) + usize::from(col)
    }
}
