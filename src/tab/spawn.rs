use std::{path::Path, time::Instant};

use anyhow::Result;

use super::{
    agent_tracker::AgentTracker, metadata, metadata::SessionMetadata, pty::PtyTransport,
    terminal::TerminalEmulator, Tab,
};

impl Tab {
    pub fn spawn(
        id: u64,
        tab_id: u64,
        cols: u16,
        rows: u16,
        shell: Option<&str>,
        control_socket: Option<&Path>,
    ) -> Result<Self> {
        let transport = PtyTransport::spawn(id, tab_id, cols, rows, shell, control_socket)?;
        let now = Instant::now();
        let mut tab = Self {
            id,
            title: String::new(),
            custom_title: false,
            info: metadata::initial_info(),
            alive: true,
            dirty: true,
            transport,
            terminal: TerminalEmulator::new(cols, rows),
            metadata: SessionMetadata::new(now),
            agent: AgentTracker::new(now),
        };
        tab.refresh_info(true);
        tab.refresh_agent_status(Instant::now(), true);
        tab.recompose();
        Ok(tab)
    }
}
