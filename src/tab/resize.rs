use std::time::Instant;

use anyhow::Result;

use super::Tab;

impl Tab {
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        let cols = cols.max(1);
        let rows = rows.max(1);
        if self.terminal.size() == (cols, rows) {
            return Ok(());
        }
        self.transport.resize(cols, rows)?;
        self.terminal.resize(cols, rows);
        self.agent
            .note_resize(self.terminal.screen(), Instant::now());
        self.dirty = true;
        Ok(())
    }
}
