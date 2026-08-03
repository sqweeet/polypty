//! Split-tree ownership and structural editing.

mod edit;
mod node;

pub(in crate::workspace) use node::SplitNode;

use super::SplitAxis;

pub(super) struct SplitTree {
    pub(super) root: Option<SplitNode>,
}

impl SplitTree {
    pub(super) fn new(id: u64) -> Self {
        Self {
            root: Some(SplitNode::Leaf(id)),
        }
    }

    pub(super) fn split(
        &mut self,
        target: u64,
        new_id: u64,
        axis: SplitAxis,
        first_extent: Option<u16>,
    ) -> bool {
        self.root
            .as_mut()
            .is_some_and(|root| root.replace_leaf(target, new_id, axis, first_extent))
    }

    pub(super) fn remove(&mut self, id: u64) -> bool {
        let Some(root) = self.root.take() else {
            return false;
        };
        let (root, removed) = root.remove(id);
        self.root = root;
        removed
    }

    pub(super) fn first_leaf(&self) -> Option<u64> {
        self.root.as_ref().map(SplitNode::first_leaf)
    }
}
