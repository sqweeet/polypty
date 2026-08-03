use std::time::Instant;

use anyhow::Result;

use super::App;

impl App {
    pub fn poll_ptys(&mut self) -> Result<bool> {
        let area = self.layout().terminal_rect();
        let active = self.book.active_index();
        let mut changed = false;
        let mut active_output_bytes = 0usize;
        for (index, workspace) in self.book.iter_mut().enumerate() {
            let poll = workspace.poll((index == active).then_some(area))?;
            changed |= poll.visible_changed || poll.sidebar_changed;
            if index == active {
                active_output_bytes = active_output_bytes.saturating_add(poll.active_output_bytes);
            }
        }

        let now = Instant::now();
        self.frame.record_output(active_output_bytes, now);
        if changed {
            self.frame.invalidate();
        }
        changed |= self.sync_sidebar_animation(now);
        Ok(changed)
    }

    pub(super) fn sync_sidebar_animation(&mut self, now: Instant) -> bool {
        let states = self.book.iter().map(|workspace| {
            (
                workspace.id(),
                workspace.agent_status().map(|status| status.state),
            )
        });
        let changed = self.presenter.reconcile_sidebar(states, now);
        if changed {
            self.presenter.invalidate_sidebar_content();
            self.frame.invalidate();
        }
        changed
    }
}
