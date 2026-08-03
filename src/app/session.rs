use anyhow::{bail, Result};

use crate::{
    session::TerminalSession,
    workspace::{SplitAxis, Workspace},
};

use super::App;

impl App {
    pub(super) fn layout(&self) -> crate::render::Layout {
        self.viewport.layout()
    }

    fn spawn_session(&mut self, cols: u16, rows: u16) -> Result<Box<dyn TerminalSession>> {
        let id = self.book.allocate_pane_id();
        self.sessions.spawn(id, cols.max(1), rows.max(1))
    }

    pub(super) fn spawn_workspace(&mut self) -> Result<()> {
        let area = self.layout().terminal_rect();
        let tab = self.spawn_session(area.cols, area.rows)?;
        self.book.push_and_select(Workspace::new(tab));
        self.frame.clear_cursor_settle();
        self.frame.request_full_draw();
        Ok(())
    }

    pub(super) fn split_active(&mut self, axis: SplitAxis) -> Result<()> {
        let area = self.layout().terminal_rect();
        let (cols, rows) = self
            .book
            .active()
            .map(|workspace| workspace.split_size(axis, area))
            .ok_or_else(|| anyhow::anyhow!("no active workspace"))?;
        let tab = self.spawn_session(cols, rows)?;
        if !self
            .book
            .active_mut()
            .is_some_and(|workspace| workspace.split(tab, axis, area))
        {
            bail!("active pane is missing from split tree");
        }
        self.on_pane_focus_changed()?;
        Ok(())
    }
}
