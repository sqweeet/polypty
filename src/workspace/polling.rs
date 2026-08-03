use anyhow::Result;

use crate::core::geometry::TerminalRect;

use super::{Workspace, WorkspacePoll};

impl Workspace {
    pub fn poll(&mut self, visible_area: Option<TerminalRect>) -> Result<WorkspacePoll> {
        let mut result = WorkspacePoll::default();
        let visible_layout = visible_area.map(|area| self.layout(area));
        let sidebar_before = self.info();
        let active = self.focus.active();

        for pane in self.panes.iter_mut() {
            let changed = pane.session.poll()?;
            if changed
                && visible_layout
                    .as_ref()
                    .is_some_and(|layout| layout.contains_pane(pane.id()))
            {
                result.visible_changed = true;
            }
            if pane.id() == active {
                result.active_output_bytes = result
                    .active_output_bytes
                    .saturating_add(pane.session.last_poll_bytes());
            }
        }
        result.sidebar_changed = self.info() != sidebar_before;
        Ok(result)
    }
}
