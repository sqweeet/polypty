use anyhow::Result;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::app::{interaction::sidebar_menu_mouse::MenuMouseRoute, App};

impl App {
    pub fn handle_mouse(&mut self, event: MouseEvent) -> Result<bool> {
        if self.exit_dialog.visible() {
            return self.handle_exit_dialog_mouse(event);
        }
        if self.shortcut_dialog.visible() {
            return Ok(self.handle_shortcut_dialog_mouse(event));
        }
        let layout = self.layout();
        let area = layout.terminal_rect();
        let menu_changed = match self.route_sidebar_menu_mouse(event, layout)? {
            MenuMouseRoute::Handled(changed) => return Ok(changed),
            MenuMouseRoute::Continue(changed) => changed,
        };
        if let Some(changed) = self.route_sidebar_tab_press(event, layout)? {
            return Ok(menu_changed || changed);
        }
        let hover_changed = self.update_sidebar_tab_hover(event, layout);
        let changed = match event.kind {
            MouseEventKind::Down(MouseButton::Left) => self.handle_left_down(event, layout)?,
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved
                if self.viewport.is_dragging_sidebar() =>
            {
                self.resize_sidebar_from(event.column)
            }
            MouseEventKind::Up(MouseButton::Left) if self.viewport.is_dragging_sidebar() => {
                self.viewport.end_sidebar_drag();
                true
            }
            MouseEventKind::Down(MouseButton::Middle) => self.handle_middle_down(event, layout)?,
            MouseEventKind::ScrollUp
                if layout.sidebar_visible && event.column < layout.sidebar_width =>
            {
                self.select_prev()?;
                true
            }
            MouseEventKind::ScrollDown
                if layout.sidebar_visible && event.column < layout.sidebar_width =>
            {
                self.select_next()?;
                true
            }
            _ if area.contains(event.column, event.row) => {
                self.forward_mouse_to_active(event, area)?;
                false
            }
            _ => false,
        };
        Ok(menu_changed || hover_changed || changed)
    }
}
