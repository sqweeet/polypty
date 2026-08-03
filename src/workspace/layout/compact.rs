/// Choose a stable first-pane height when sparse content would waste half the screen.
pub(in crate::workspace) fn compact_horizontal_extent(
    screen: &vt100::Screen,
    total_rows: u16,
) -> Option<u16> {
    let available = total_rows.saturating_sub(1);
    let half = (available / 2).max(1);
    const MIN_ROWS: u16 = 5;
    if available < MIN_ROWS.saturating_mul(2) {
        return None;
    }

    let (screen_rows, screen_cols) = screen.size();
    let mut used = screen.cursor_position().0.saturating_add(1);
    for row in 0..screen_rows.min(total_rows) {
        let has_text = (0..screen_cols).any(|col| {
            screen.cell(row, col).is_some_and(|cell| {
                cell.contents()
                    .chars()
                    .any(|character| !character.is_whitespace())
            })
        });
        if has_text {
            used = used.max(row.saturating_add(1));
        }
    }
    (MIN_ROWS..half).contains(&used).then_some(used)
}
