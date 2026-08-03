use crate::core::geometry::{Divider, TerminalRect};

use super::Workspace;

pub(crate) struct PaneSnapshot<'a> {
    pub(crate) id: u64,
    pub(crate) rect: TerminalRect,
    pub(crate) screen: &'a vt100::Screen,
    pub(crate) dirty: bool,
}

pub(crate) struct WorkspaceSnapshot<'a> {
    pub(crate) id: u64,
    pub(crate) active: u64,
    pub(crate) panes: Vec<PaneSnapshot<'a>>,
    pub(crate) dividers: Vec<Divider>,
}

impl Workspace {
    pub(crate) fn snapshot(&self, area: TerminalRect) -> WorkspaceSnapshot<'_> {
        let layout = self.layout(area);
        let panes = layout
            .panes
            .iter()
            .filter_map(|(id, rect)| {
                self.panes.get(*id).map(|pane| PaneSnapshot {
                    id: *id,
                    rect: *rect,
                    screen: pane.session.screen(),
                    dirty: pane.session.is_dirty(),
                })
            })
            .collect();
        WorkspaceSnapshot {
            id: self.id,
            active: self.focus.active(),
            panes,
            dividers: layout.dividers,
        }
    }

    pub(crate) fn mark_rendered(&mut self, pane_ids: &[u64]) {
        for id in pane_ids {
            if let Some(pane) = self.panes.get_mut(*id) {
                pane.session.mark_rendered();
            }
        }
    }
}
