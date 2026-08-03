use std::time::Instant;

use crate::app::{
    timing::{CURSOR_SETTLE_DELAY, RESIZE_CHILD_REDRAW_DELAY, RESIZE_SETTLE_DELAY},
    FrameScheduler,
};

impl FrameScheduler {
    pub(in crate::app) fn resize_in_progress(&self) -> bool {
        self.resize_commit_at.is_some()
    }

    pub(in crate::app) fn waiting_for_resize_repaint(&self, now: Instant) -> bool {
        self.resize_repaint_at
            .is_some_and(|deadline| now < deadline)
    }

    pub(in crate::app) fn stage_resize(&mut self, now: Instant) {
        self.viewport_dirty = true;
        self.resize_commit_at = Some(now + RESIZE_SETTLE_DELAY);
        self.resize_repaint_at = None;
        self.invalidate();
    }

    pub(in crate::app) fn resize_due(&self, now: Instant) -> bool {
        self.resize_commit_at
            .is_some_and(|deadline| now >= deadline)
    }

    pub(in crate::app) fn complete_resize(&mut self, now: Instant) {
        self.resize_commit_at = None;
        self.resize_repaint_at = Some(now + RESIZE_CHILD_REDRAW_DELAY);
        self.cursor_settle_until = Some(now + CURSOR_SETTLE_DELAY);
        self.invalidate();
    }

    pub(in crate::app) fn geometry_synced(&mut self) {
        self.resize_repaint_at = None;
        self.invalidate();
    }
}
