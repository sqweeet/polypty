use anyhow::Result;
use crossterm::event::MouseEvent;

use crate::{app::App, core::geometry::TerminalRect, input, render};

impl App {
    pub(super) fn forward_mouse_to_active(
        &mut self,
        event: MouseEvent,
        area: TerminalRect,
    ) -> Result<bool> {
        let bytes = {
            let Some(workspace) = self.book.active() else {
                return Ok(false);
            };
            let Some((col, row)) = workspace.active_cell_at(event.column, event.row, area) else {
                return Ok(false);
            };
            let screen = workspace.active_screen();
            input::encode_mouse(
                event,
                col,
                row,
                screen.mouse_protocol_mode(),
                screen.mouse_protocol_encoding(),
            )
        };
        if bytes.is_empty() {
            return Ok(false);
        }
        self.write_active(&bytes)?;
        Ok(true)
    }

    pub(super) fn handle_left_down(
        &mut self,
        event: MouseEvent,
        layout: render::Layout,
    ) -> Result<bool> {
        let area = layout.terminal_rect();
        if layout.sidebar_visible && event.column < layout.sidebar_width {
            if event.column.saturating_add(3) >= layout.sidebar_width {
                self.viewport.begin_sidebar_drag();
                return Ok(self.resize_sidebar_from(event.column));
            }
            if let Some(index) = self.presenter.sidebar_tab_at(event.column, event.row) {
                return self.select_workspace(index);
            }
            return Ok(false);
        }
        if !area.contains(event.column, event.row) {
            return Ok(false);
        }
        let changed = self
            .book
            .active_mut()
            .is_some_and(|workspace| workspace.focus_at(event.column, event.row, area));
        if changed {
            self.on_pane_focus_changed()?;
        }
        self.forward_mouse_to_active(event, area)?;
        Ok(changed)
    }

    pub(super) fn resize_sidebar_from(&mut self, column: u16) -> bool {
        if !self.viewport.set_sidebar_width(column.max(1)) {
            return false;
        }
        self.stage_workspace_resize();
        true
    }
}
