use anyhow::Result;
use crossterm::event::MouseEvent;

use crate::{app::App, platform::clipboard::ClipboardKind, render};

impl App {
    pub(super) fn handle_middle_down(
        &mut self,
        event: MouseEvent,
        layout: render::Layout,
    ) -> Result<bool> {
        let area = layout.terminal_rect();
        if layout.sidebar_visible && event.column < layout.sidebar_width {
            if let Some(index) = self.presenter.sidebar_tab_at(event.column, event.row) {
                self.close_workspace(index)?;
                return Ok(true);
            }
            return Ok(false);
        }
        if !area.contains(event.column, event.row) {
            return Ok(false);
        }
        let focus_changed = self
            .book
            .active_mut()
            .is_some_and(|workspace| workspace.focus_at(event.column, event.row, area));
        if focus_changed {
            self.on_pane_focus_changed()?;
        }
        if self.forward_mouse_to_active(event, area)? {
            return Ok(focus_changed);
        }
        if let Some(text) = self.clipboard.read(ClipboardKind::Primary) {
            self.handle_paste(&text)?;
        }
        Ok(true)
    }
}
