use std::time::Instant;

use crate::app::{timing::OUTPUT_MAX_FRAME_DELAY, App};

impl App {
    pub fn needs_draw(&self) -> bool {
        let now = Instant::now();
        if self.frame.force_draw() || self.frame.hard_clear() || self.frame.viewport_dirty() {
            return true;
        }
        if self.sidebar_menu.animation_due()
            || self.shortcut_dialog.animation_due()
            || self.exit_dialog.animation_due()
        {
            return true;
        }
        if self.frame.waiting_for_resize_repaint(now) {
            return false;
        }
        let workspace_dirty = self.book.active().is_some_and(|workspace| {
            let snapshot = workspace.snapshot(self.layout().terminal_rect());
            self.presenter.workspace_needs_draw(&snapshot)
        });
        if self.frame.dirty() || workspace_dirty {
            let (started, quiet) = self.frame.output_window();
            return output_frame_ready(started, quiet, now);
        }
        let cursor_due = self.frame.cursor_restore_due(now);
        cursor_due || (self.viewport.sidebar_visible() && self.presenter.sidebar_frame_due(now))
    }
}

fn output_frame_ready(started: Option<Instant>, quiet: Option<Instant>, now: Instant) -> bool {
    let Some(started) = started else {
        return true;
    };
    quiet.is_some_and(|deadline| now >= deadline)
        || now.duration_since(started) >= OUTPUT_MAX_FRAME_DELAY
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::app::timing::OUTPUT_QUIET_DELAY;

    #[test]
    fn output_frames_wait_for_quiet_but_never_starve() {
        let started = Instant::now();
        let quiet = started + OUTPUT_QUIET_DELAY;
        assert!(!output_frame_ready(
            Some(started),
            Some(quiet),
            started + Duration::from_millis(2)
        ));
        assert!(output_frame_ready(Some(started), Some(quiet), quiet));
        assert!(output_frame_ready(
            Some(started),
            Some(started + Duration::from_secs(1)),
            started + OUTPUT_MAX_FRAME_DELAY
        ));
        assert!(output_frame_ready(None, None, started));
    }
}
