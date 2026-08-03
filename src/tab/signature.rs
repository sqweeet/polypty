use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Hash only emulated cell contents. Control sequences, cursor blinking and
/// identical repaint bytes deliberately leave this signature unchanged.
pub(super) fn screen_signature(screen: &vt100::Screen) -> u64 {
    let (rows, cols) = screen.size();
    let mut hash = DefaultHasher::new();
    rows.hash(&mut hash);
    cols.hash(&mut hash);
    for row in 0..rows {
        row.hash(&mut hash);
        for col in 0..cols {
            if let Some(cell) = screen.cell(row, col) {
                cell.contents().hash(&mut hash);
            }
        }
    }
    hash.finish()
}
