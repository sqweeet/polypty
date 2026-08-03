use std::time::Instant;

use crate::app::{
    timing::{CURSOR_SETTLE_DELAY, CURSOR_SUPPRESS_BURST_BYTES, OUTPUT_QUIET_DELAY},
    FrameScheduler,
};

impl FrameScheduler {
    pub(in crate::app) fn output_window(&self) -> (Option<Instant>, Option<Instant>) {
        (self.output_burst_started, self.output_quiet_until)
    }

    pub(in crate::app) fn record_output(&mut self, bytes: usize, now: Instant) {
        if bytes == 0 {
            return;
        }
        self.output_burst_started.get_or_insert(now);
        self.output_quiet_until = Some(now + OUTPUT_QUIET_DELAY);
        self.output_burst_bytes = self.output_burst_bytes.saturating_add(bytes);
        if self.cursor_settle_until.is_some()
            || self.output_burst_bytes >= CURSOR_SUPPRESS_BURST_BYTES
        {
            self.cursor_settle_until = Some(now + CURSOR_SETTLE_DELAY);
        }
    }
}
