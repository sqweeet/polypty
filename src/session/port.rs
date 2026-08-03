use anyhow::Result;

use crate::info::TabInfo;

pub(crate) trait TerminalSession {
    fn id(&self) -> u64;
    fn info(&self) -> &TabInfo;
    fn screen(&self) -> &vt100::Screen;
    fn poll(&mut self) -> Result<bool>;
    fn last_poll_bytes(&self) -> usize;
    fn resize(&mut self, cols: u16, rows: u16) -> Result<()>;
    fn write_all(&mut self, data: &[u8]) -> Result<()>;
    fn try_reap(&mut self) -> bool;
    fn is_alive(&self) -> bool;
    fn kill(&mut self);
    fn is_dirty(&self) -> bool;
    fn mark_rendered(&mut self);
}
