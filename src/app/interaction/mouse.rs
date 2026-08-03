use anyhow::Result;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::app::App;

impl App {
    pub fn handle_mouse(&mut self, event: MouseEvent) -> Result<bool> {
        if self.exit_dialog.visible() {
            return self.handle_exit_dialog_mouse(event);
        }
        let layout = self.layout();
        let area = layout.terminal_rect();
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => self.handle_left_down(event, layout),
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved
                if self.viewport.is_dragging_sidebar() =>
            {
                Ok(self.resize_sidebar_from(event.column))
            }
            MouseEventKind::Up(MouseButton::Left) if self.viewport.is_dragging_sidebar() => {
                self.viewport.end_sidebar_drag();
                Ok(true)
            }
            MouseEventKind::Down(MouseButton::Middle) => self.handle_middle_down(event, layout),
            MouseEventKind::ScrollUp
                if layout.sidebar_visible && event.column < layout.sidebar_width =>
            {
                self.select_prev()?;
                Ok(true)
            }
            MouseEventKind::ScrollDown
                if layout.sidebar_visible && event.column < layout.sidebar_width =>
            {
                self.select_next()?;
                Ok(true)
            }
            _ if area.contains(event.column, event.row) => {
                self.forward_mouse_to_active(event, area)?;
                Ok(false)
            }
            _ => Ok(false),
        }
    }
}
