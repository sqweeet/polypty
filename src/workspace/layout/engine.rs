use crate::core::geometry::{Divider, TerminalRect};

use super::WorkspaceLayout;
use crate::workspace::{tree::SplitNode, tree::SplitTree, SplitAxis};

pub(super) struct LayoutEngine;

impl LayoutEngine {
    pub(super) fn build(tree: &SplitTree, active: u64, area: TerminalRect) -> WorkspaceLayout {
        let mut output = WorkspaceLayout::default();
        if let Some(root) = &tree.root {
            let safe_area = TerminalRect {
                cols: area.cols.max(1),
                rows: area.rows.max(1),
                ..area
            };
            layout_node(root, safe_area, active, &mut output);
        }
        output
    }
}

fn layout_node(node: &SplitNode, rect: TerminalRect, active: u64, output: &mut WorkspaceLayout) {
    match node {
        SplitNode::Leaf(id) => output.panes.push((*id, rect)),
        SplitNode::Split {
            axis,
            first_extent,
            first,
            second,
        } => match axis {
            SplitAxis::Vertical if rect.cols >= 3 => {
                layout_vertical(first, second, rect, active, output);
            }
            SplitAxis::Horizontal if rect.rows >= 3 => {
                layout_horizontal(first, second, *first_extent, rect, active, output);
            }
            _ if first.contains(active) => layout_node(first, rect, active, output),
            _ => layout_node(second, rect, active, output),
        },
    }
}

fn layout_vertical(
    first: &SplitNode,
    second: &SplitNode,
    rect: TerminalRect,
    active: u64,
    output: &mut WorkspaceLayout,
) {
    let available = rect.cols - 1;
    let first_cols = available / 2;
    let divider_x = rect.x.saturating_add(first_cols);
    let first_rect = TerminalRect {
        cols: first_cols,
        ..rect
    };
    let second_rect = TerminalRect {
        x: divider_x.saturating_add(1),
        cols: available - first_cols,
        ..rect
    };
    output.dividers.push(Divider::Vertical {
        x: divider_x,
        y: rect.y,
        len: rect.rows,
    });
    layout_node(first, first_rect, active, output);
    layout_node(second, second_rect, active, output);
}

fn layout_horizontal(
    first: &SplitNode,
    second: &SplitNode,
    first_extent: Option<u16>,
    rect: TerminalRect,
    active: u64,
    output: &mut WorkspaceLayout,
) {
    let available = rect.rows - 1;
    let first_rows = first_extent
        .unwrap_or(available / 2)
        .clamp(1, available - 1);
    let divider_y = rect.y.saturating_add(first_rows);
    let first_rect = TerminalRect {
        rows: first_rows,
        ..rect
    };
    let second_rect = TerminalRect {
        y: divider_y.saturating_add(1),
        rows: available - first_rows,
        ..rect
    };
    output.dividers.push(Divider::Horizontal {
        x: rect.x,
        y: divider_y,
        len: rect.cols,
    });
    layout_node(first, first_rect, active, output);
    layout_node(second, second_rect, active, output);
}
