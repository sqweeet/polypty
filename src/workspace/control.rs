use anyhow::Result;

use super::Workspace;

impl Workspace {
    pub(crate) fn pane_ids(&self) -> Vec<u64> {
        self.panes.iter().map(|pane| pane.id()).collect()
    }

    pub(crate) fn active_pane_id(&self) -> u64 {
        self.focus.active()
    }

    pub(crate) fn pane_screen(&self, id: u64) -> Option<&vt100::Screen> {
        self.panes.get(id).map(|pane| pane.session.screen())
    }

    pub(crate) fn write_pane(&mut self, id: u64, data: &[u8]) -> Result<bool> {
        let Some(pane) = self.panes.get_mut(id) else {
            return Ok(false);
        };
        pane.session.write_all(data)?;
        Ok(true)
    }
}
