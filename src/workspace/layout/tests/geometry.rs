use super::*;

#[test]
fn nested_split_covers_area_without_overlap() {
    let mut tree = SplitTree::new(1);
    split(&mut tree, 1, 2, SplitAxis::Vertical, None);
    split(&mut tree, 2, 3, SplitAxis::Horizontal, Some(8));
    let area = TerminalRect {
        x: 18,
        y: 0,
        cols: 81,
        rows: 30,
    };
    let layout = layout(&tree, area, 3);

    assert_eq!(layout.panes.len(), 3);
    assert_eq!(layout.dividers.len(), 2);
    for (i, (_, left)) in layout.panes.iter().enumerate() {
        for (_, right) in layout.panes.iter().skip(i + 1) {
            let overlap_x = left.x < right.x + right.cols && right.x < left.x + left.cols;
            let overlap_y = left.y < right.y + right.rows && right.y < left.y + left.rows;
            assert!(!(overlap_x && overlap_y));
        }
    }
}

#[test]
fn nested_layout_converges_after_tiny_resize_round_trip() {
    let mut tree = SplitTree::new(1);
    split(&mut tree, 1, 2, SplitAxis::Vertical, None);
    split(&mut tree, 2, 3, SplitAxis::Horizontal, Some(7));
    let large = TerminalRect {
        x: 18,
        y: 0,
        cols: 82,
        rows: 31,
    };

    let before = layout(&tree, large, 3);
    let tiny = layout(
        &tree,
        TerminalRect {
            cols: 2,
            rows: 2,
            ..large
        },
        3,
    );
    assert_eq!(tiny.panes.len(), 1);
    assert!(tiny.dividers.is_empty());

    let after = layout(&tree, large, 3);
    assert_eq!(after.panes, before.panes);
    assert_eq!(after.dividers, before.dividers);
}
