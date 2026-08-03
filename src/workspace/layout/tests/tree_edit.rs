use super::*;

#[test]
fn removing_leaf_collapses_parent_split() {
    let mut tree = SplitTree::new(1);
    split(&mut tree, 1, 2, SplitAxis::Vertical, None);
    assert!(tree.remove(2));
    assert!(matches!(
        tree.root,
        Some(crate::workspace::tree::SplitNode::Leaf(1))
    ));
}
