use std::time::Instant;

mod output;
mod resize;

pub(super) struct FrameScheduler {
    force_draw: bool,
    hard_clear: bool,
    dirty_ui: bool,
    cursor_settle_until: Option<Instant>,
    viewport_dirty: bool,
    resize_commit_at: Option<Instant>,
    resize_repaint_at: Option<Instant>,
    output_burst_started: Option<Instant>,
    output_quiet_until: Option<Instant>,
    output_burst_bytes: usize,
    quit_requested: bool,
}

impl Default for FrameScheduler {
    fn default() -> Self {
        Self {
            force_draw: true,
            hard_clear: true,
            dirty_ui: true,
            cursor_settle_until: None,
            viewport_dirty: false,
            resize_commit_at: None,
            resize_repaint_at: None,
            output_burst_started: None,
            output_quiet_until: None,
            output_burst_bytes: 0,
            quit_requested: false,
        }
    }
}

impl FrameScheduler {
    pub(super) fn force_draw(&self) -> bool {
        self.force_draw
    }
    pub(super) fn hard_clear(&self) -> bool {
        self.hard_clear
    }
    pub(super) fn viewport_dirty(&self) -> bool {
        self.viewport_dirty
    }
    pub(super) fn dirty(&self) -> bool {
        self.dirty_ui
    }
    pub(super) fn quit_requested(&self) -> bool {
        self.quit_requested
    }
    pub(super) fn request_full_draw(&mut self) {
        self.force_draw = true;
        self.invalidate();
    }

    pub(super) fn request_hard_clear(&mut self) {
        self.hard_clear = true;
        self.request_full_draw();
    }

    pub(super) fn request_quit(&mut self) {
        self.quit_requested = true;
    }
    pub(super) fn clear_cursor_settle(&mut self) {
        self.cursor_settle_until = None;
    }

    pub(super) fn cursor_settled(&self, now: Instant) -> bool {
        self.cursor_settle_until
            .is_none_or(|deadline| now >= deadline)
    }

    pub(super) fn cursor_restore_due(&self, now: Instant) -> bool {
        self.cursor_settle_until.is_some() && self.cursor_settled(now)
    }

    pub(super) fn invalidate(&mut self) {
        self.dirty_ui = true;
    }

    pub(super) fn finish_frame(&mut self, restored_cursor: bool) {
        self.force_draw = false;
        self.hard_clear = false;
        self.viewport_dirty = false;
        self.resize_repaint_at = None;
        self.output_burst_started = None;
        self.output_quiet_until = None;
        self.output_burst_bytes = 0;
        self.dirty_ui = false;
        if restored_cursor {
            self.cursor_settle_until = None;
        }
    }
}
