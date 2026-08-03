use crate::{core::geometry::TerminalRect, session::TerminalSession};

use super::{layout::compact_horizontal_extent, SplitAxis, Workspace};

impl Workspace {
    pub fn split_size(&self, axis: SplitAxis, area: TerminalRect) -> (u16, u16) {
        let rect = self.active_rect(area).unwrap_or(area);
        match axis {
            SplitAxis::Vertical if rect.cols >= 3 => {
                let available = rect.cols - 1;
                (available - available / 2, rect.rows.max(1))
            }
            SplitAxis::Horizontal if rect.rows >= 3 => {
                let available = rect.rows - 1;
                let first_rows = compact_horizontal_extent(self.active_screen(), rect.rows)
                    .unwrap_or(available / 2);
                (rect.cols.max(1), available - first_rows)
            }
            _ => (1, 1),
        }
    }

    pub(crate) fn split(
        &mut self,
        session: Box<dyn TerminalSession>,
        axis: SplitAxis,
        area: TerminalRect,
    ) -> bool {
        let active = self.focus.active();
        let new_id = session.id();
        let first_extent = if axis == SplitAxis::Horizontal {
            self.active_rect(area)
                .and_then(|rect| compact_horizontal_extent(self.active_screen(), rect.rows))
        } else {
            None
        };
        if !self.tree.split(active, new_id, axis, first_extent) {
            return false;
        }
        self.panes.push(session);
        self.focus.activate(new_id);
        true
    }

    fn active_rect(&self, area: TerminalRect) -> Option<TerminalRect> {
        self.layout(area).rect(self.focus.active())
    }
}
