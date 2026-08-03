mod collapse;
mod compact;
mod geometry;
mod tree_edit;

use crate::core::geometry::TerminalRect;

use super::{tree_layout, WorkspaceLayout};
use crate::workspace::{tree::SplitTree, SplitAxis};

fn split(tree: &mut SplitTree, target: u64, new_id: u64, axis: SplitAxis, extent: Option<u16>) {
    assert!(tree.split(target, new_id, axis, extent));
}

fn layout(tree: &SplitTree, area: TerminalRect, active: u64) -> WorkspaceLayout {
    tree_layout(tree, area, active)
}
