use crate::core::geometry::TerminalRect;

use super::{PaneDirection, Workspace};

impl Workspace {
    pub fn focus_next(&mut self) -> bool {
        if self.panes.len() <= 1 {
            return false;
        }
        let changed = self.focus.activate_next(&self.panes);
        self.mark_focus_changed(changed)
    }

    pub fn focus_direction(&mut self, direction: PaneDirection, area: TerminalRect) -> bool {
        let layout = self.layout(area);
        let changed = self.focus.activate_direction(&layout, direction);
        self.mark_focus_changed(changed)
    }

    pub fn focus_at(&mut self, x: u16, y: u16, area: TerminalRect) -> bool {
        let layout = self.layout(area);
        let changed = self.focus.activate_at(&layout, x, y);
        self.mark_focus_changed(changed)
    }

    /// Translate a host cell into the active pane's zero-based grid.
    pub fn active_cell_at(&self, x: u16, y: u16, area: TerminalRect) -> Option<(u16, u16)> {
        self.layout(area)
            .rect(self.focus.active())
            .filter(|rect| rect.contains(x, y))
            .map(|rect| (x - rect.x, y - rect.y))
    }

    fn mark_focus_changed(&mut self, changed: bool) -> bool {
        changed
    }
}
