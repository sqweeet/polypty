use std::io::Write;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::render::{self, SidebarMap, SidebarTab};

use super::App;
#[cfg(test)]
use super::OUTPUT_QUIET_DELAY;

const OUTPUT_MAX_FRAME_DELAY: Duration = Duration::from_millis(16);
impl App {
    pub fn needs_draw(&self) -> bool {
        let now = Instant::now();
        if self.force_draw || self.needs_hard_clear || self.viewport_dirty {
            return true;
        }
        if self
            .resize_repaint_at
            .is_some_and(|deadline| now < deadline)
        {
            return false;
        }
        let workspace_dirty = self
            .workspaces
            .get(self.active)
            .is_some_and(|workspace| workspace.needs_draw(self.layout().terminal_rect()));
        if self.dirty_ui || workspace_dirty {
            return self.output_frame_ready(now);
        }
        let cursor_due = self
            .cursor_settle_until
            .is_some_and(|deadline| now >= deadline);
        cursor_due
            || (self.sidebar_visible && self.sidebar_animation.frame_due(&self.sidebar_map, now))
    }

    fn output_frame_ready(&self, now: Instant) -> bool {
        is_output_frame_ready(self.output_burst_started, self.output_quiet_until, now)
    }

    pub fn draw(&mut self, out: &mut impl Write) -> Result<()> {
        if !self.needs_draw() {
            return Ok(());
        }

        let now = Instant::now();
        let layout = self.layout();
        let area = layout.terminal_rect();
        let force = self.force_draw;
        let hard = self.needs_hard_clear;
        let active = self.active;
        let cursor_settled = self
            .cursor_settle_until
            .map(|deadline| now >= deadline)
            .unwrap_or(true);
        let need_cursor_restore = self.cursor_settle_until.is_some() && cursor_settled;
        let resize_in_progress = self.resize_commit_at.is_some();

        let side_tabs: Vec<SidebarTab> = self
            .workspaces
            .iter()
            .enumerate()
            .map(|(i, workspace)| {
                let info = workspace.info();
                let key = workspace.id();
                SidebarTab {
                    key,
                    primary: info.primary,
                    secondary: info.secondary,
                    agent: info.agent,
                    glint_frame: self.sidebar_animation.frame(key, now),
                    active: i == active,
                }
            })
            .collect();
        let glint_due = self.sidebar_animation.frame_due(&self.sidebar_map, now);
        let fp = sidebar_fingerprint(
            &side_tabs,
            layout.sidebar_visible,
            layout.sidebar_width,
            layout.rows,
        );
        let need_sidebar =
            layout.sidebar_visible && (force || hard || fp != self.sidebar_fp || glint_due);
        let need_workspace = force
            || hard
            || self.viewport_dirty
            || need_cursor_restore
            || self
                .workspaces
                .get(active)
                .is_some_and(|workspace| workspace.needs_draw(area));

        let frame_cols = if need_workspace {
            layout.cols
        } else {
            layout.sidebar_width
        };
        let frame_cells = (frame_cols as usize).saturating_mul(layout.rows as usize);
        let mut frame = Vec::with_capacity(frame_cells.saturating_mul(4));
        render::begin_sync(&mut frame)?;

        if hard {
            render::clear(&mut frame)?;
            if let Some(workspace) = self.workspaces.get_mut(active) {
                workspace.reset_blank(area);
            }
            self.sidebar_cache.invalidate();
        }

        if need_sidebar {
            self.sidebar_map = render::draw_sidebar(
                &mut frame,
                &layout,
                &side_tabs,
                &mut self.sidebar_cache,
                hard,
            )?;
            self.sidebar_fp = fp;
        } else if !layout.sidebar_visible {
            self.sidebar_map = SidebarMap::default();
            self.sidebar_cache.invalidate();
            self.sidebar_fp.clear();
        }

        if need_workspace {
            if let Some(workspace) = self.workspaces.get_mut(active) {
                workspace.draw(
                    &mut frame,
                    area,
                    !cursor_settled || resize_in_progress,
                    false,
                )?;
            }
        } else if need_sidebar {
            if let Some(workspace) = self.workspaces.get(active) {
                workspace.restore_active_cursor(
                    &mut frame,
                    area,
                    !cursor_settled || resize_in_progress,
                )?;
            }
        }

        render::end_sync(&mut frame)?;
        out.write_all(&frame).context("write frame")?;
        out.flush().context("flush stdout")?;

        self.force_draw = false;
        self.needs_hard_clear = false;
        self.viewport_dirty = false;
        self.resize_repaint_at = None;
        self.output_burst_started = None;
        self.output_quiet_until = None;
        self.output_burst_bytes = 0;
        self.dirty_ui = false;
        if need_cursor_restore {
            self.cursor_settle_until = None;
        }
        Ok(())
    }
}

fn is_output_frame_ready(
    started: Option<Instant>,
    quiet_until: Option<Instant>,
    now: Instant,
) -> bool {
    let Some(started) = started else {
        return true;
    };
    let quiet = quiet_until.is_some_and(|deadline| now >= deadline);
    quiet || now.duration_since(started) >= OUTPUT_MAX_FRAME_DELAY
}

fn sidebar_fingerprint(tabs: &[SidebarTab], visible: bool, width: u16, rows: u16) -> String {
    let mut fingerprint = format!("v{visible}|w{width}|r{rows}|");
    for (i, tab) in tabs.iter().enumerate() {
        fingerprint.push_str(&format!(
            "{i}:{}:{}:{}:{}|",
            tab.primary,
            tab.secondary,
            tab.agent
                .map(|status| {
                    format!(
                        "{}:{}:{}:{}",
                        status.kind.label(),
                        status.state.label(),
                        status.panes,
                        status.mixed_kinds as u8
                    )
                })
                .unwrap_or_default(),
            tab.active as u8
        ));
    }
    fingerprint
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_frames_wait_for_quiet_but_never_starve() {
        let started = Instant::now();
        let quiet_until = started + OUTPUT_QUIET_DELAY;

        assert!(!is_output_frame_ready(
            Some(started),
            Some(quiet_until),
            started + Duration::from_millis(2),
        ));
        assert!(is_output_frame_ready(
            Some(started),
            Some(quiet_until),
            quiet_until,
        ));
        assert!(is_output_frame_ready(
            Some(started),
            Some(started + Duration::from_secs(1)),
            started + OUTPUT_MAX_FRAME_DELAY,
        ));
        assert!(is_output_frame_ready(None, None, started));
    }

    #[test]
    fn agent_state_is_part_of_sidebar_fingerprint() {
        let mut tabs = vec![SidebarTab {
            key: 1,
            primary: "codex".into(),
            secondary: "~/projects/mux".into(),
            agent: Some(crate::agent::AgentStatus::single(
                crate::agent::AgentKind::Codex,
                crate::agent::AgentState::Working,
            )),
            glint_frame: Some(crate::render::GlintFrame::for_elapsed(Duration::ZERO)),
            active: true,
        }];
        let working = sidebar_fingerprint(&tabs, true, 18, 24);
        tabs[0].agent.as_mut().unwrap().state = crate::agent::AgentState::Blocked;
        let blocked = sidebar_fingerprint(&tabs, true, 18, 24);
        tabs[0].agent.as_mut().unwrap().panes = 2;
        let split = sidebar_fingerprint(&tabs, true, 18, 24);

        assert_ne!(working, blocked);
        assert_ne!(blocked, split);

        let structural = sidebar_fingerprint(&tabs, true, 18, 24);
        tabs[0].glint_frame = Some(crate::render::GlintFrame::for_elapsed(
            Duration::from_millis(800),
        ));
        assert_eq!(structural, sidebar_fingerprint(&tabs, true, 18, 24));
    }
}
