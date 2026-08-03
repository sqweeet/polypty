use super::SplitNode;
use crate::workspace::SplitAxis;

impl SplitNode {
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
            } => remove_from_split(target, axis, first_extent, *first, *second),
        }
    }
}

fn remove_from_split(
    target: u64,
    axis: SplitAxis,
    first_extent: Option<u16>,
    first: SplitNode,
    second: SplitNode,
) -> (Option<SplitNode>, bool) {
    let (new_first, removed) = first.remove(target);
    if removed {
        let root = match new_first {
            Some(first) => split(axis, first_extent, first, second),
            None => second,
        };
        return (Some(root), true);
    }
    let first = new_first.expect("unchanged first split branch");
    let (new_second, removed) = second.remove(target);
    if removed {
        let root = match new_second {
            Some(second) => split(axis, first_extent, first, second),
            None => first,
        };
        (Some(root), true)
    } else {
        (
            Some(split(
                axis,
                first_extent,
                first,
                new_second.expect("unchanged second split branch"),
            )),
            false,
        )
    }
}

fn split(
    axis: SplitAxis,
    first_extent: Option<u16>,
    first: SplitNode,
    second: SplitNode,
) -> SplitNode {
    SplitNode::Split {
        axis,
        first_extent,
        first: Box::new(first),
        second: Box::new(second),
    }
}
