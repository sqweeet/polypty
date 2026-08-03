use std::time::Instant;

use anyhow::{bail, Result};

use super::{pty::InputQueueResult, Tab};

impl Tab {
    pub fn write_all(&mut self, data: &[u8]) -> Result<()> {
        if !self.alive {
            bail!("tab {} is dead", self.id);
        }
        if data.is_empty() {
            return Ok(());
        }
        match self.transport.queue_user(data.to_vec()) {
            InputQueueResult::Queued => self.agent.note_input(data, Instant::now()),
            InputQueueResult::Full => {}
            InputQueueResult::Disconnected => self.kill(),
        }
        Ok(())
    }

    pub(super) fn queue_query_response(&mut self, response: Vec<u8>) {
        if self.transport.queue_query(response) == InputQueueResult::Disconnected {
            self.kill();
        }
    }
}
