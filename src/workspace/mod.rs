//! Workspace domain aggregate: panes, split geometry, and focus.

mod control;
mod focus;
mod io;
mod layout;
mod lifecycle;
mod metadata;
mod navigation;
mod pane;
mod polling;
mod resizing;
pub(crate) mod snapshot;
mod splitting;
mod tree;

use focus::FocusModel;
use pane::PaneStore;
use tree::SplitTree;

use crate::session::TerminalSession;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitAxis {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WorkspacePoll {
    /// Output changed in a pane that is currently visible.
    pub visible_changed: bool,
    /// The active pane's sidebar metadata changed.
    pub sidebar_changed: bool,
    pub active_output_bytes: usize,
}

pub struct Workspace {
    /// Immutable sidebar identity; split collapse and focus never change it.
    id: u64,
    panes: PaneStore,
    tree: SplitTree,
    focus: FocusModel,
}

impl Workspace {
    pub(crate) fn new(session: Box<dyn TerminalSession>) -> Self {
        let id = session.id();
        Self {
            id,
            panes: PaneStore::new(session),
            tree: SplitTree::new(id),
            focus: FocusModel::new(id),
        }
    }
}
