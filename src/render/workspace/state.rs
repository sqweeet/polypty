use std::collections::{BTreeMap, BTreeSet};

use crate::{
    core::geometry::{Divider, TerminalRect},
    render::TermCache,
    workspace::snapshot::WorkspaceSnapshot,
};

pub(in crate::render) struct PaneRenderState {
    pub(in crate::render) cache: TermCache,
    pub(in crate::render) rect: Option<TerminalRect>,
}

#[derive(Default)]
pub(in crate::render) struct WorkspaceRenderer {
    panes: BTreeMap<u64, PaneRenderState>,
    active: Option<u64>,
    dividers: Vec<Divider>,
    invalidated: bool,
}

impl WorkspaceRenderer {
    pub(in crate::render) fn invalidate(&mut self) {
        for pane in self.panes.values_mut() {
            pane.cache.invalidate();
            pane.rect = None;
        }
        self.invalidated = true;
    }

    pub(in crate::render) fn needs_draw(&self, snapshot: &WorkspaceSnapshot<'_>) -> bool {
        self.invalidated
            || self.active != Some(snapshot.active)
            || self.dividers != snapshot.dividers
            || snapshot.panes.iter().any(|pane| {
                pane.dirty
                    || self
                        .panes
                        .get(&pane.id)
                        .is_none_or(|state| state.rect != Some(pane.rect))
            })
    }

    pub(in crate::render) fn chrome_changed(&self, snapshot: &WorkspaceSnapshot<'_>) -> bool {
        self.invalidated || self.dividers != snapshot.dividers
    }

    pub(in crate::render) fn sync_geometry(&mut self, snapshot: &WorkspaceSnapshot<'_>) -> bool {
        let visible: BTreeSet<_> = snapshot.panes.iter().map(|pane| pane.id).collect();
        let removed = self.panes.keys().any(|id| !visible.contains(id));
        self.panes.retain(|id, _| visible.contains(id));
        let mut changed = removed;
        for pane in &snapshot.panes {
            let state = self
                .panes
                .entry(pane.id)
                .or_insert_with(|| PaneRenderState {
                    cache: TermCache::default(),
                    rect: None,
                });
            if state.rect != Some(pane.rect) {
                if state
                    .rect
                    .is_none_or(|old| old.x != pane.rect.x || old.y != pane.rect.y)
                {
                    state.cache.invalidate();
                }
                state.rect = Some(pane.rect);
                changed = true;
            }
        }
        changed
    }

    pub(in crate::render) fn pane_mut(&mut self, id: u64) -> &mut PaneRenderState {
        self.panes.get_mut(&id).expect("visible pane cache exists")
    }

    pub(in crate::render) fn redraw_all(&self, force: bool) -> bool {
        force || self.invalidated
    }

    pub(in crate::render) fn finish(&mut self, snapshot: &WorkspaceSnapshot<'_>) {
        self.active = Some(snapshot.active);
        self.dividers.clone_from(&snapshot.dividers);
        self.invalidated = false;
    }
}
