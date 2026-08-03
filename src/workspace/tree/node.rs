use super::super::SplitAxis;

#[derive(Debug, Clone)]
pub(in crate::workspace) enum SplitNode {
    Leaf(u64),
    Split {
        axis: SplitAxis,
        /// Fixed first-pane height for compact horizontal splits.
        first_extent: Option<u16>,
        first: Box<SplitNode>,
        second: Box<SplitNode>,
    },
}

impl SplitNode {
    pub(in crate::workspace) fn contains(&self, id: u64) -> bool {
        match self {
            Self::Leaf(leaf) => *leaf == id,
            Self::Split { first, second, .. } => first.contains(id) || second.contains(id),
        }
    }

    pub(in crate::workspace) fn first_leaf(&self) -> u64 {
        match self {
            Self::Leaf(id) => *id,
            Self::Split { first, .. } => first.first_leaf(),
        }
    }
}
