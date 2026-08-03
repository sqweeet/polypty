use anyhow::Result;

use crate::tab::Tab;

use super::TerminalSession;

pub(crate) trait SessionFactory {
    fn spawn(&mut self, id: u64, cols: u16, rows: u16) -> Result<Box<dyn TerminalSession>>;
}

#[derive(Debug, Default)]
pub(crate) struct PtySessionFactory;

impl SessionFactory for PtySessionFactory {
    fn spawn(&mut self, id: u64, cols: u16, rows: u16) -> Result<Box<dyn TerminalSession>> {
        Ok(Box::new(Tab::spawn(id, cols, rows)?))
    }
}
