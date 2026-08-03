use std::time::Instant;

use anyhow::{bail, Result};

use super::App;

impl App {
    pub fn preview_resize(&mut self, cols: u16, rows: u16) -> bool {
        self.invalidate_host_render_state();
        if !self.viewport.resize(cols, rows) {
            return false;
        }
        self.stage_workspace_resize();
        true
    }

    fn invalidate_host_render_state(&mut self) {
        self.presenter.invalidate_sidebar();
        if let Some(workspace) = self.book.active() {
            self.presenter.invalidate_workspace(workspace.id());
        }
        self.frame.invalidate();
    }

    pub fn commit_resize_if_due(&mut self) -> Result<bool> {
        let now = Instant::now();
        if !self.frame.resize_due(now) {
            return Ok(false);
        }
        let area = self.layout().terminal_rect();
        for workspace in self.book.iter_mut() {
            workspace.resize(area)?;
        }
        self.frame.complete_resize(now);
        Ok(true)
    }

    pub(super) fn sync_active_workspace_geometry(&mut self) -> Result<()> {
        let area = self.layout().terminal_rect();
        let Some(workspace) = self.book.active_mut() else {
            bail!("no active workspace");
        };
        workspace.resize(area)?;
        self.frame.geometry_synced();
        Ok(())
    }

    pub(super) fn stage_workspace_resize(&mut self) {
        self.frame.stage_resize(Instant::now());
        self.presenter.invalidate_sidebar_content();
    }
}
