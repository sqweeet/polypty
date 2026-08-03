use std::time::Instant;

use anyhow::Result;

use super::{
    close_policy::{self, CloseTransition},
    App,
};

impl Drop for App {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl App {
    pub fn reap(&mut self) -> Result<bool> {
        if self.frame.quit_requested() {
            return Ok(true);
        }
        let area = self.layout().terminal_rect();
        let mut index = 0;
        let mut active_geometry_changed = false;
        while index < self.book.len() {
            let geometry_changed = self.book.get_mut(index).expect("workspace exists").reap();
            if self
                .book
                .get(index)
                .is_some_and(|workspace| workspace.is_empty())
            {
                self.remove_empty_workspace(index, area)?;
                if self.book.is_empty() {
                    return Ok(true);
                }
                continue;
            }
            if geometry_changed {
                self.book
                    .get_mut(index)
                    .expect("workspace exists")
                    .resize(area)?;
                self.presenter.invalidate_sidebar_content();
                self.frame.invalidate();
                active_geometry_changed |= index == self.book.active_index();
            }
            index += 1;
        }
        if active_geometry_changed {
            self.frame.geometry_synced();
            self.frame.clear_cursor_settle();
            self.presenter.invalidate_sidebar_content();
        }
        self.sync_sidebar_animation(Instant::now());
        Ok(false)
    }

    fn remove_empty_workspace(
        &mut self,
        index: usize,
        area: crate::core::geometry::TerminalRect,
    ) -> Result<()> {
        let was_active = index == self.book.active_index();
        let removed = self.book.remove(index).expect("workspace exists");
        self.presenter.remove_workspace(removed.id());
        if self.book.is_empty() {
            return Ok(());
        }
        if was_active {
            let workspace = self.book.active_mut().expect("active workspace exists");
            self.presenter.invalidate_workspace(workspace.id());
            workspace.resize(area)?;
            self.frame.request_full_draw();
            self.frame.clear_cursor_settle();
        }
        self.presenter.invalidate_sidebar_content();
        self.frame.invalidate();
        Ok(())
    }

    pub fn shutdown(&mut self) {
        for workspace in self.book.iter_mut() {
            workspace.kill_all();
        }
    }

    pub(super) fn close_workspace(&mut self, index: usize) -> Result<bool> {
        match close_policy::transition(self.book.len(), self.book.active_index(), index) {
            CloseTransition::Ignore => Ok(false),
            CloseTransition::Quit => {
                self.book
                    .get_mut(index)
                    .expect("workspace exists")
                    .kill_all();
                self.frame.request_quit();
                Ok(true)
            }
            CloseTransition::Active(active) => self.remove_workspace(index, active),
        }
    }

    fn remove_workspace(&mut self, index: usize, expected_active: usize) -> Result<bool> {
        let workspace = self.book.get_mut(index).expect("workspace exists");
        workspace.kill_all();
        let removed_id = workspace.id();
        self.book.remove(index);
        debug_assert_eq!(self.book.active_index(), expected_active);
        self.presenter.remove_workspace(removed_id);
        let active = self.book.active().expect("active workspace exists");
        self.presenter.invalidate_workspace(active.id());
        self.sync_active_workspace_geometry()?;
        self.frame.clear_cursor_settle();
        self.frame.request_full_draw();
        self.presenter.invalidate_sidebar_content();
        self.sync_sidebar_animation(Instant::now());
        Ok(false)
    }
}
