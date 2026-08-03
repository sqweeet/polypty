use crate::render::{Divider, TerminalRect};

use super::SplitAxis;

#[derive(Debug, Clone)]
pub(super) enum SplitNode {
    Leaf(u64),
    Split {
        axis: SplitAxis,
        /// Horizontal splits can keep the original pane compact instead of
        /// reintroducing blank rows whenever the host window grows.
        first_extent: Option<u16>,
        first: Box<SplitNode>,
        second: Box<SplitNode>,
    },
}

impl SplitNode {
    fn contains(&self, id: u64) -> bool {
        match self {
            Self::Leaf(leaf) => *leaf == id,
            Self::Split { first, second, .. } => first.contains(id) || second.contains(id),
        }
    }

    pub(super) fn first_leaf(&self) -> u64 {
        match self {
            Self::Leaf(id) => *id,
            Self::Split { first, .. } => first.first_leaf(),
        }
    }

    pub(super) fn replace_leaf(
        &mut self,
        target: u64,
        new_id: u64,
        axis: SplitAxis,
        first_extent: Option<u16>,
    ) -> bool {
        match self {
            Self::Leaf(id) if *id == target => {
                *self = Self::Split {
                    axis,
                    first_extent,
                    first: Box::new(Self::Leaf(target)),
                    second: Box::new(Self::Leaf(new_id)),
                };
                true
            }
            Self::Leaf(_) => false,
            Self::Split { first, second, .. } => {
                first.replace_leaf(target, new_id, axis, first_extent)
                    || second.replace_leaf(target, new_id, axis, first_extent)
            }
        }
    }

    pub(super) fn remove(self, target: u64) -> (Option<Self>, bool) {
        match self {
            Self::Leaf(id) if id == target => (None, true),
            Self::Leaf(id) => (Some(Self::Leaf(id)), false),
            Self::Split {
                axis,
                first_extent,
                first,
                second,
            } => {
                let (first, removed) = first.remove(target);
                if removed {
                    return (
                        match first {
                            Some(first) => Some(Self::Split {
                                axis,
                                first_extent,
                                first: Box::new(first),
                                second,
                            }),
                            None => Some(*second),
                        },
                        true,
                    );
                }

                let first = first.expect("unchanged split branch");
                let (second, removed) = second.remove(target);
                if removed {
                    (
                        match second {
                            Some(second) => Some(Self::Split {
                                axis,
                                first_extent,
                                first: Box::new(first),
                                second: Box::new(second),
                            }),
                            None => Some(first),
                        },
                        true,
                    )
                } else {
                    (
                        Some(Self::Split {
                            axis,
                            first_extent,
                            first: Box::new(first),
                            second: Box::new(second.expect("unchanged split branch")),
                        }),
                        false,
                    )
                }
            }
        }
    }

    pub(super) fn layout(
        &self,
        rect: TerminalRect,
        active: u64,
        panes: &mut Vec<(u64, TerminalRect)>,
        dividers: &mut Vec<Divider>,
    ) {
        match self {
            Self::Leaf(id) => panes.push((*id, rect)),
            Self::Split {
                axis,
                first_extent,
                first,
                second,
            } => match axis {
                SplitAxis::Vertical if rect.cols >= 3 => {
                    let available = rect.cols - 1;
                    let first_cols = available / 2;
                    let second_cols = available - first_cols;
                    let divider_x = rect.x.saturating_add(first_cols);
                    let first_rect = TerminalRect {
                        x: rect.x,
                        y: rect.y,
                        cols: first_cols,
                        rows: rect.rows,
                    };
                    let second_rect = TerminalRect {
                        x: divider_x.saturating_add(1),
                        y: rect.y,
                        cols: second_cols,
                        rows: rect.rows,
                    };
                    dividers.push(Divider::Vertical {
                        x: divider_x,
                        y: rect.y,
                        len: rect.rows,
                    });
                    first.layout(first_rect, active, panes, dividers);
                    second.layout(second_rect, active, panes, dividers);
                }
                SplitAxis::Horizontal if rect.rows >= 3 => {
                    let available = rect.rows - 1;
                    let first_rows = first_extent
                        .unwrap_or(available / 2)
                        .clamp(1, available - 1);
                    let second_rows = available - first_rows;
                    let divider_y = rect.y.saturating_add(first_rows);
                    let first_rect = TerminalRect {
                        x: rect.x,
                        y: rect.y,
                        cols: rect.cols,
                        rows: first_rows,
                    };
                    let second_rect = TerminalRect {
                        x: rect.x,
                        y: divider_y.saturating_add(1),
                        cols: rect.cols,
                        rows: second_rows,
                    };
                    dividers.push(Divider::Horizontal {
                        x: rect.x,
                        y: divider_y,
                        len: rect.cols,
                    });
                    first.layout(first_rect, active, panes, dividers);
                    second.layout(second_rect, active, panes, dividers);
                }
                _ => {
                    // Never create zero-sized or overlapping panes. If a nested
                    // split cannot fit, show the branch containing the active pane.
                    if first.contains(active) {
                        first.layout(rect, active, panes, dividers);
                    } else {
                        second.layout(rect, active, panes, dividers);
                    }
                }
            },
        }
    }
}

#[derive(Default)]
pub(super) struct WorkspaceLayout {
    pub(super) panes: Vec<(u64, TerminalRect)>,
    pub(super) dividers: Vec<Divider>,
}

impl WorkspaceLayout {
    pub(super) fn contains_pane(&self, id: u64) -> bool {
        self.panes.iter().any(|(pane_id, _)| *pane_id == id)
    }

    pub(super) fn pane_size(&self, id: u64) -> Option<(u16, u16)> {
        self.panes.iter().find_map(|(pane_id, rect)| {
            (*pane_id == id).then_some((rect.cols.max(1), rect.rows.max(1)))
        })
    }

    pub(super) fn has_visible_dirty(&self, panes: impl IntoIterator<Item = (u64, bool)>) -> bool {
        panes
            .into_iter()
            .any(|(id, dirty)| dirty && self.contains_pane(id))
    }
}

pub(super) fn compact_horizontal_extent(screen: &vt100::Screen, total_rows: u16) -> Option<u16> {
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

    // Return a fixed extent only when it removes real blank rows. Otherwise
    // leave the node dynamic so an ordinary 50/50 split keeps balancing on
    // later host resizes.
    (MIN_ROWS..half).contains(&used).then_some(used)
}

pub(super) fn center(rect: TerminalRect) -> (i32, i32) {
    (
        rect.x as i32 * 2 + rect.cols as i32,
        rect.y as i32 * 2 + rect.rows as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_layout(root: &SplitNode, area: TerminalRect, active: u64) -> WorkspaceLayout {
        let mut layout = WorkspaceLayout::default();
        root.layout(area, active, &mut layout.panes, &mut layout.dividers);
        layout
    }

    #[test]
    fn nested_split_covers_area_without_overlap() {
        let mut root = SplitNode::Leaf(1);
        assert!(root.replace_leaf(1, 2, SplitAxis::Vertical, None));
        assert!(root.replace_leaf(2, 3, SplitAxis::Horizontal, Some(8)));
        let area = TerminalRect {
            x: 18,
            y: 0,
            cols: 81,
            rows: 30,
        };
        let layout = node_layout(&root, area, 3);

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
    fn tiny_layout_shows_only_active_branch() {
        let mut root = SplitNode::Leaf(1);
        assert!(root.replace_leaf(1, 2, SplitAxis::Vertical, None));
        let layout = node_layout(
            &root,
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
        let mut root = SplitNode::Leaf(1);
        assert!(root.replace_leaf(1, 2, SplitAxis::Vertical, None));
        let area = TerminalRect {
            x: 7,
            y: 3,
            cols: 2,
            rows: 9,
        };

        let first = node_layout(&root, area, 1);
        assert_eq!(first.panes, vec![(1, area)]);
        assert!(!first.has_visible_dirty([(2, true)]));
        assert!(first.has_visible_dirty([(1, true), (2, false)]));
        assert_eq!(first.pane_size(1), Some((2, 9)));
        // No resize is planned for a hidden pane, so its last usable parser
        // geometry is retained until focus makes it visible again.
        assert_eq!(first.pane_size(2), None);

        let second = node_layout(&root, area, 2);
        assert_eq!(second.panes, vec![(2, area)]);
        assert!(!second.has_visible_dirty([(1, true)]));
        assert!(second.has_visible_dirty([(2, true)]));
        assert_eq!(second.pane_size(2), Some((2, 9)));
        assert_eq!(second.pane_size(1), None);
    }

    #[test]
    fn nested_layout_converges_after_tiny_resize_round_trip() {
        let mut root = SplitNode::Leaf(1);
        assert!(root.replace_leaf(1, 2, SplitAxis::Vertical, None));
        assert!(root.replace_leaf(2, 3, SplitAxis::Horizontal, Some(7)));
        let large = TerminalRect {
            x: 18,
            y: 0,
            cols: 82,
            rows: 31,
        };

        let before = node_layout(&root, large, 3);
        let tiny = node_layout(
            &root,
            TerminalRect {
                cols: 2,
                rows: 2,
                ..large
            },
            3,
        );
        assert_eq!(tiny.panes.len(), 1);
        assert!(tiny.dividers.is_empty());

        let after = node_layout(&root, large, 3);
        assert_eq!(after.panes, before.panes);
        assert_eq!(after.dividers, before.dividers);
    }

    #[test]
    fn removing_leaf_collapses_parent_split() {
        let mut root = SplitNode::Leaf(1);
        assert!(root.replace_leaf(1, 2, SplitAxis::Vertical, None));
        let (root, removed) = root.remove(2);
        assert!(removed);
        assert!(matches!(root, Some(SplitNode::Leaf(1))));
    }

    #[test]
    fn horizontal_split_hugs_sparse_content() {
        let mut parser = vt100::Parser::new(38, 49, 0);
        parser.process(b"one\r\ntwo\x1b[11;1H");

        assert_eq!(compact_horizontal_extent(parser.screen(), 38), Some(11));
    }

    #[test]
    fn horizontal_split_keeps_shells_and_busy_screens_half() {
        let sparse = vt100::Parser::new(38, 49, 0);
        assert_eq!(compact_horizontal_extent(sparse.screen(), 38), None);

        let mut busy = vt100::Parser::new(38, 49, 0);
        busy.process(b"\x1b[30;1Hbusy");
        assert_eq!(compact_horizontal_extent(busy.screen(), 38), None);
    }

    #[test]
    fn compact_extent_survives_growth_and_clamps_without_zero_panes() {
        let mut root = SplitNode::Leaf(1);
        assert!(root.replace_leaf(1, 2, SplitAxis::Horizontal, Some(11)));
        let area = TerminalRect {
            x: 0,
            y: 0,
            cols: 20,
            rows: 60,
        };
        let grown = node_layout(&root, area, 2);
        assert_eq!(grown.panes[0].1.rows, 11);
        assert_eq!(grown.panes[1].1.rows, 48);

        let tiny = node_layout(&root, TerminalRect { rows: 8, ..area }, 2);
        assert_eq!(tiny.panes[0].1.rows, 6);
        assert_eq!(tiny.panes[1].1.rows, 1);
    }
}
