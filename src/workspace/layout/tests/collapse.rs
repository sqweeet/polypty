use super::*;

#[test]
fn tiny_layout_shows_only_active_branch() {
    let mut tree = SplitTree::new(1);
    split(&mut tree, 1, 2, SplitAxis::Vertical, None);
    let layout = layout(
        &tree,
        TerminalRect {
            x: 0,
            y: 0,
            cols: 1,
            rows: 1,
        },
        2,
    );
    assert_eq!(
        layout.panes,
        vec![(
            2,
            TerminalRect {
                x: 0,
                y: 0,
                cols: 1,
                rows: 1
            }
        )]
    );
    assert!(layout.dividers.is_empty());
}

#[test]
fn collapsed_layout_ignores_hidden_dirty_panes_and_preserves_their_geometry() {
    let mut tree = SplitTree::new(1);
    split(&mut tree, 1, 2, SplitAxis::Vertical, None);
    let area = TerminalRect {
        x: 7,
        y: 3,
        cols: 2,
        rows: 9,
    };

    let first = layout(&tree, area, 1);
    assert_eq!(first.panes, vec![(1, area)]);
    assert!(!first.has_visible_dirty([(2, true)]));
    assert!(first.has_visible_dirty([(1, true), (2, false)]));
    assert_eq!(first.pane_size(1), Some((2, 9)));
    // Hidden panes retain their last usable parser geometry.
    assert_eq!(first.pane_size(2), None);

    let second = layout(&tree, area, 2);
    assert_eq!(second.panes, vec![(2, area)]);
    assert!(!second.has_visible_dirty([(1, true)]));
    assert!(second.has_visible_dirty([(2, true)]));
    assert_eq!(second.pane_size(2), Some((2, 9)));
    assert_eq!(second.pane_size(1), None);
}
