//! Stateless split-tree layout engine and its result model.

mod compact;
mod engine;

pub(in crate::workspace) use compact::compact_horizontal_extent;

use crate::core::geometry::{Divider, TerminalRect};

#[cfg(test)]
use super::tree::SplitTree;
use super::Workspace;
use engine::LayoutEngine;

#[derive(Default)]
pub(super) struct WorkspaceLayout {
    pub(super) panes: Vec<(u64, TerminalRect)>,
    pub(super) dividers: Vec<Divider>,
}

impl WorkspaceLayout {
    pub(super) fn contains_pane(&self, id: u64) -> bool {
        self.panes.iter().any(|(pane_id, _)| *pane_id == id)
    }

    pub(super) fn rect(&self, id: u64) -> Option<TerminalRect> {
        self.panes
            .iter()
            .find_map(|(pane_id, rect)| (*pane_id == id).then_some(*rect))
    }

    pub(super) fn pane_size(&self, id: u64) -> Option<(u16, u16)> {
        self.rect(id)
            .map(|rect| (rect.cols.max(1), rect.rows.max(1)))
    }

    #[cfg(test)]
    pub(super) fn has_visible_dirty(&self, panes: impl IntoIterator<Item = (u64, bool)>) -> bool {
        panes
            .into_iter()
            .any(|(id, dirty)| dirty && self.contains_pane(id))
    }
}

impl Workspace {
    pub(super) fn layout(&self, area: TerminalRect) -> WorkspaceLayout {
        LayoutEngine::build(&self.tree, self.focus.active(), area)
    }
}

#[cfg(test)]
pub(super) fn tree_layout(tree: &SplitTree, area: TerminalRect, active: u64) -> WorkspaceLayout {
    LayoutEngine::build(tree, active, area)
}

#[cfg(test)]
mod tests;
