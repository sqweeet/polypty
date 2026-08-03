mod sidebar_tabs;

use std::{collections::BTreeMap, io::Write, time::Instant};

use anyhow::Result;

use crate::{agent::AgentState, workspace::snapshot::WorkspaceSnapshot};

use super::{
    sidebar::{SidebarPresentation, SidebarShortcuts, SidebarTab},
    workspace::{self, WorkspaceRenderer},
    GlintFrame, Layout,
};

pub(crate) struct Presenter {
    sidebar: SidebarPresentation,
    workspaces: BTreeMap<u64, WorkspaceRenderer>,
}

impl Default for Presenter {
    fn default() -> Self {
        Self::new(SidebarShortcuts::default())
    }
}

impl Presenter {
    pub(crate) fn new(shortcuts: SidebarShortcuts) -> Self {
        Self {
            sidebar: SidebarPresentation::new(shortcuts),
            workspaces: BTreeMap::new(),
        }
    }

    pub(crate) fn invalidate_sidebar(&mut self) {
        self.sidebar.invalidate();
    }

    pub(crate) fn invalidate_sidebar_content(&mut self) {
        self.sidebar.invalidate_content();
    }

    pub(crate) fn set_sidebar_shortcuts_visible(&mut self, visible: bool) {
        self.sidebar.set_shortcuts_visible(visible);
    }

    pub(crate) fn reconcile_sidebar<I>(&mut self, states: I, now: Instant) -> bool
    where
        I: IntoIterator<Item = (u64, Option<AgentState>)>,
        I::IntoIter: Clone,
    {
        self.sidebar.reconcile(states, now)
    }

    pub(crate) fn sidebar_frame(&self, id: u64, now: Instant) -> Option<GlintFrame> {
        self.sidebar.frame(id, now)
    }

    pub(crate) fn sidebar_frame_due(&self, now: Instant) -> bool {
        self.sidebar.frame_due(now)
    }

    pub(crate) fn sidebar_fingerprint(&self) -> &str {
        self.sidebar.fingerprint()
    }

    pub(crate) fn sidebar_tab_at(&self, column: u16, row: u16) -> Option<usize> {
        self.sidebar.tab_at(column, row)
    }

    pub(crate) fn draw_sidebar(
        &mut self,
        output: &mut impl Write,
        layout: &Layout,
        tabs: &[SidebarTab],
        fingerprint: &str,
        hard_clear: bool,
    ) -> Result<()> {
        self.sidebar
            .draw(output, layout, tabs, fingerprint, hard_clear)
    }

    pub(crate) fn clear_sidebar(&mut self) {
        self.sidebar.clear();
    }

    pub(crate) fn invalidate_workspace(&mut self, id: u64) {
        self.workspaces.entry(id).or_default().invalidate();
    }

    pub(crate) fn remove_workspace(&mut self, id: u64) {
        self.workspaces.remove(&id);
    }

    pub(crate) fn workspace_needs_draw(&self, snapshot: &WorkspaceSnapshot<'_>) -> bool {
        self.workspaces
            .get(&snapshot.id)
            .is_none_or(|renderer| renderer.needs_draw(snapshot))
    }

    pub(crate) fn reset_workspace_blank(&mut self, snapshot: &WorkspaceSnapshot<'_>) {
        let renderer = self.workspaces.entry(snapshot.id).or_default();
        renderer.invalidate();
        renderer.sync_geometry(snapshot);
        for pane in &snapshot.panes {
            let state = renderer.pane_mut(pane.id);
            state.cache.reset_blank(pane.rect.cols, pane.rect.rows);
            state.rect = Some(pane.rect);
        }
    }

    pub(crate) fn draw_workspace(
        &mut self,
        output: &mut impl Write,
        snapshot: &WorkspaceSnapshot<'_>,
        suppress_cursor: bool,
        force: bool,
    ) -> Result<Vec<u64>> {
        let renderer = self.workspaces.entry(snapshot.id).or_default();
        workspace::paint(renderer, output, snapshot, suppress_cursor, force)
    }

    pub(crate) fn restore_workspace_cursor(
        &self,
        output: &mut impl Write,
        snapshot: &WorkspaceSnapshot<'_>,
        suppress_cursor: bool,
    ) -> Result<()> {
        workspace::restore_cursor(output, snapshot, suppress_cursor)
    }
}
