use std::time::Duration;

pub(super) const CURSOR_SETTLE_DELAY: Duration = Duration::from_millis(24);
pub(super) const RESIZE_SETTLE_DELAY: Duration = Duration::from_millis(48);
pub(super) const RESIZE_CHILD_REDRAW_DELAY: Duration = Duration::from_millis(12);
pub(super) const OUTPUT_QUIET_DELAY: Duration = Duration::from_millis(3);
pub(super) const OUTPUT_MAX_FRAME_DELAY: Duration = Duration::from_millis(16);
pub(super) const CURSOR_SUPPRESS_BURST_BYTES: usize = 512;
