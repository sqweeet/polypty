use std::time::Instant;

use anyhow::Result;

use super::Tab;

impl Tab {
    /// Drain pending PTY output into the VT parser.
    pub fn poll(&mut self) -> Result<bool> {
        let now = Instant::now();
        let mut title_updated = false;
        let terminal = &mut self.terminal;
        let outcome = self.transport.drain_output(|bytes| {
            title_updated |= terminal.process(bytes);
        });
        if title_updated {
            self.agent.note_title_update(now);
        }

        let responses = self.terminal.take_responses();
        if !responses.is_empty() {
            self.queue_query_response(responses);
        }

        let mut changed = false;
        if outcome.bytes > 0 {
            self.agent.note_output(self.terminal.screen(), now);
            self.dirty = true;
            changed = true;
            self.sync_terminal_metadata();
        }
        if outcome.disconnected {
            self.alive = false;
        }

        let probed = self.metadata.needs_probe();
        if probed {
            changed |= self.refresh_info(false);
        } else if changed {
            let previous = self.info.clone();
            self.recompose();
            changed |= self.info != previous;
        }
        changed |= self.refresh_agent_status(now, outcome.bytes > 0 || probed);
        Ok(changed)
    }
}
