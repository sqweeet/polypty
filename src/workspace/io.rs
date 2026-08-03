use anyhow::Result;

use super::Workspace;

impl Workspace {
    pub fn write_active(&mut self, data: &[u8]) -> Result<()> {
        self.active_session_mut().write_all(data)
    }
}
