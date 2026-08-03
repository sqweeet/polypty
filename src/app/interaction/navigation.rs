use anyhow::{bail, Result};

use crate::{app::App, workspace::PaneDirection};

impl App {
    pub(super) fn focus_pane(&mut self, direction: PaneDirection) -> Result<()> {
        let area = self.layout().terminal_rect();
        if self
            .book
            .active_mut()
            .is_some_and(|workspace| workspace.focus_direction(direction, area))
        {
            self.on_pane_focus_changed()?;
        }
        Ok(())
    }

    pub(in crate::app) fn on_pane_focus_changed(&mut self) -> Result<()> {
        self.sync_active_workspace_geometry()?;
        self.frame.clear_cursor_settle();
        self.presenter.invalidate_sidebar_content();
        self.frame.invalidate();
        Ok(())
    }

    pub(in crate::app) fn select_workspace(&mut self, index: usize) -> Result<bool> {
        if !self.book.select(index) {
            return Ok(false);
        }
        if let Some(workspace) = self.book.active() {
            self.presenter.invalidate_workspace(workspace.id());
        }
        self.sync_active_workspace_geometry()?;
        self.frame.clear_cursor_settle();
        self.frame.request_full_draw();
        self.presenter.invalidate_sidebar_content();
        Ok(true)
    }

    pub(super) fn select_next(&mut self) -> Result<()> {
        if let Some(index) = self.book.next_index() {
            self.select_workspace(index)?;
        }
        Ok(())
    }

    pub(super) fn select_prev(&mut self) -> Result<()> {
        if let Some(index) = self.book.previous_index() {
            self.select_workspace(index)?;
        }
        Ok(())
    }

    pub(super) fn write_active(&mut self, data: &[u8]) -> Result<()> {
        let Some(workspace) = self.book.active_mut() else {
            bail!("no active workspace");
        };
        workspace.write_active(data)
    }

    pub(super) fn adjust_sidebar(&mut self, delta: i16) {
        if self.viewport.adjust_sidebar(delta) {
            self.stage_workspace_resize();
        }
    }
}
