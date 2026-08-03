//! One shell tab, composed from PTY, terminal, metadata, and agent state.

mod activity;
mod activity_text;
mod agent_tracker;
mod environment;
mod input;
mod lifecycle;
mod metadata;
mod poll;
mod pty;
mod resize;
mod session;
mod signature;
mod spawn;
mod terminal;

use agent_tracker::AgentTracker;
use metadata::SessionMetadata;
use pty::PtyTransport;
use terminal::TerminalEmulator;

use crate::info::TabInfo;

/// Stable facade for one shell session.
pub struct Tab {
    id: u64,
    /// OSC window title (raw).
    title: String,
    custom_title: bool,
    /// Live sidebar info (process + cwd).
    info: TabInfo,
    alive: bool,
    dirty: bool,
    transport: PtyTransport,
    terminal: TerminalEmulator,
    metadata: SessionMetadata,
    agent: AgentTracker,
}

impl Tab {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn info(&self) -> &TabInfo {
        &self.info
    }

    pub fn is_alive(&self) -> bool {
        self.alive
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(crate) fn mark_rendered(&mut self) {
        self.dirty = false;
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.terminal.screen()
    }

    pub fn last_poll_bytes(&self) -> usize {
        self.transport.last_poll_bytes()
    }
}

#[cfg(test)]
mod tests;
