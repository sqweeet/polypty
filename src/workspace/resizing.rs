use anyhow::Result;

use crate::core::geometry::TerminalRect;

use super::Workspace;

impl Workspace {
    pub fn resize(&mut self, area: TerminalRect) -> Result<()> {
        let layout = self.layout(area);
        for pane in self.panes.iter_mut() {
            // Hidden branches retain their last usable parser geometry.
            if let Some((cols, rows)) = layout.pane_size(pane.id()) {
                pane.session.resize(cols, rows)?;
            }
        }
        Ok(())
    }
}
