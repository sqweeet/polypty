use anyhow::Result;

use crate::{info::TabInfo, session::TerminalSession};

use super::Tab;

impl TerminalSession for Tab {
    fn id(&self) -> u64 {
        self.id()
    }
    fn info(&self) -> &TabInfo {
        self.info()
    }
    fn screen(&self) -> &vt100::Screen {
        self.screen()
    }
    fn poll(&mut self) -> Result<bool> {
        self.poll()
    }
    fn last_poll_bytes(&self) -> usize {
        self.last_poll_bytes()
    }
    fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.resize(cols, rows)
    }
    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        self.write_all(data)
    }
    fn try_reap(&mut self) -> bool {
        self.try_reap()
    }
    fn is_alive(&self) -> bool {
        self.is_alive()
    }
    fn kill(&mut self) {
        self.kill();
    }
    fn is_dirty(&self) -> bool {
        self.is_dirty()
    }
    fn mark_rendered(&mut self) {
        self.mark_rendered();
    }
}
