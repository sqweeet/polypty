use vt100::Screen;

use super::cell::{cell_to_paint, PaintCell};

pub(super) fn desired_frame(screen: &Screen, cols: u16, rows: u16) -> Vec<PaintCell> {
    let (screen_rows, screen_cols) = screen.size();
    let mut frame = vec![PaintCell::blank(); usize::from(cols) * usize::from(rows)];

    for row in 0..rows {
        let mut col = 0;
        while col < cols {
            let index = usize::from(row) * usize::from(cols) + usize::from(col);
            if row >= screen_rows || col >= screen_cols {
                frame[index] = PaintCell::blank();
                col += 1;
                continue;
            }

            let painted = cell_to_paint(screen.cell(row, col));
            if painted.width == 0 {
                frame[index] = painted;
                col += 1;
                continue;
            }
            if usize::from(col) + usize::from(painted.width) > usize::from(cols) {
                for clipped_col in col..cols {
                    let clipped = usize::from(row) * usize::from(cols) + usize::from(clipped_col);
                    frame[clipped] = PaintCell::blank();
                }
                break;
            }

            frame[index] = painted.clone();
            for offset in 1..painted.width {
                let continuation =
                    usize::from(row) * usize::from(cols) + usize::from(col + u16::from(offset));
                frame[continuation] =
                    PaintCell::continuation(painted.fg, painted.bg, painted.attrs);
            }
            col += u16::from(painted.width);
        }
    }
    frame
}
