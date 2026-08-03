use std::path::PathBuf;

use anyhow::Result;

use crate::tab::Tab;

use super::TerminalSession;

pub(crate) trait SessionFactory {
    fn spawn(
        &mut self,
        id: u64,
        tab_id: u64,
        cols: u16,
        rows: u16,
    ) -> Result<Box<dyn TerminalSession>>;
}

#[derive(Debug, Default)]
pub(crate) struct PtySessionFactory {
    shell: Option<String>,
    control_socket: Option<PathBuf>,
}

impl PtySessionFactory {
    pub(crate) fn new(shell: Option<String>, control_socket: Option<PathBuf>) -> Self {
        Self {
            shell,
            control_socket,
        }
    }
}

impl SessionFactory for PtySessionFactory {
    fn spawn(
        &mut self,
        id: u64,
        tab_id: u64,
        cols: u16,
        rows: u16,
    ) -> Result<Box<dyn TerminalSession>> {
        Ok(Box::new(Tab::spawn(
            id,
            tab_id,
            cols,
            rows,
            self.shell.as_deref(),
            self.control_socket.as_deref(),
        )?))
    }
}
