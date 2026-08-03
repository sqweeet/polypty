use std::time::{Duration, Instant};

use anyhow::{bail, Result};

use crate::render::{Layout, SidebarCache, SidebarMap};
use crate::tab::Tab;
use crate::workspace::{SplitAxis, Workspace};

mod draw;
mod interaction;

const SIDEBAR_DEFAULT: u16 = 18;
/// Keep the host cursor hidden until a PTY output burst settles.
const CURSOR_SETTLE_DELAY: Duration = Duration::from_millis(24);
/// Wait for the resize burst to stop before notifying child TUIs. This avoids
/// making every intermediate window size trigger a full child redraw.
const RESIZE_SETTLE_DELAY: Duration = Duration::from_millis(48);
/// Give children a moment to react to the final SIGWINCH before presenting
/// their replacement frame.
const RESIZE_CHILD_REDRAW_DELAY: Duration = Duration::from_millis(12);
/// Coalesce split PTY writes into complete-looking frames without adding more
/// than one 60 Hz frame of latency to continuously streaming output.
const OUTPUT_QUIET_DELAY: Duration = Duration::from_millis(3);
/// Ordinary shell echo must not blink the block cursor. Only suppress it for
/// substantial redraw bursts (or while a resize already owns the cursor).
const CURSOR_SUPPRESS_BURST_BYTES: usize = 512;

pub struct App {
    workspaces: Vec<Workspace>,
    active: usize,
    next_id: u64,
    cols: u16,
    rows: u16,
    sidebar_visible: bool,
    sidebar_width: u16,
    /// Force reconciliation after switching the visible workspace.
    force_draw: bool,
    /// One-shot hard clear (only first frame).
    needs_hard_clear: bool,
    /// Last painted sidebar fingerprint — skip redraw when unchanged.
    sidebar_fp: String,
    /// Row-level sidebar diff cache.
    sidebar_cache: SidebarCache,
    sidebar_glint_epoch: Instant,
    painted_glint_step: u64,
    /// Last sidebar hit map for mouse clicks.
    sidebar_map: SidebarMap,
    /// Dragging the sidebar edge to resize.
    dragging_sidebar: bool,
    /// Something changed and a paint is useful.
    dirty_ui: bool,
    /// Programs often emit a frame in several writes. Keep intermediate
    /// cursor positions hidden until the output burst is complete.
    cursor_settle_until: Option<Instant>,
    /// Host geometry changed and needs painting, but PTYs have not yet been
    /// resized. The delayed commit coalesces a whole drag into one SIGWINCH.
    viewport_dirty: bool,
    resize_commit_at: Option<Instant>,
    resize_repaint_at: Option<Instant>,
    output_burst_started: Option<Instant>,
    output_quiet_until: Option<Instant>,
    output_burst_bytes: usize,
    /// Mouse handlers report repaint requests rather than loop control. Keep
    /// an explicit quit latch so middle-clicking the final workspace can be
    /// observed by `reap` without ever leaving an empty, invalid app state.
    quit_requested: bool,
}

impl App {
    pub fn new(cols: u16, rows: u16) -> Result<Self> {
        let sidebar_glint_epoch = Instant::now();
        let mut app = Self {
            workspaces: Vec::new(),
            active: 0,
            next_id: 1,
            cols: cols.max(1),
            rows: rows.max(1),
            sidebar_visible: true,
            sidebar_width: SIDEBAR_DEFAULT,
            force_draw: true,
            needs_hard_clear: true,
            sidebar_fp: String::new(),
            sidebar_cache: SidebarCache::default(),
            sidebar_glint_epoch,
            painted_glint_step: 0,
            sidebar_map: SidebarMap::default(),
            dragging_sidebar: false,
            dirty_ui: true,
            cursor_settle_until: None,
            viewport_dirty: false,
            resize_commit_at: None,
            resize_repaint_at: None,
            output_burst_started: None,
            output_quiet_until: None,
            output_burst_bytes: 0,
            quit_requested: false,
        };
        app.spawn_workspace()?;
        Ok(app)
    }

    fn layout(&self) -> Layout {
        Layout::new(
            self.cols,
            self.rows,
            self.sidebar_visible,
            self.sidebar_width,
        )
    }

    fn spawn_session(&mut self, cols: u16, rows: u16) -> Result<Tab> {
        let id = self.next_id;
        self.next_id += 1;
        Tab::spawn(id, cols.max(1), rows.max(1))
    }

    fn spawn_workspace(&mut self) -> Result<()> {
        let area = self.layout().terminal_rect();
        let tab = self.spawn_session(area.cols, area.rows)?;
        self.workspaces.push(Workspace::new(tab));
        self.active = self.workspaces.len() - 1;
        self.cursor_settle_until = None;
        self.force_draw = true;
        self.dirty_ui = true;
        Ok(())
    }

    fn split_active(&mut self, axis: SplitAxis) -> Result<()> {
        let area = self.layout().terminal_rect();
        let (cols, rows) = self
            .workspaces
            .get(self.active)
            .map(|workspace| workspace.split_size(axis, area))
            .ok_or_else(|| anyhow::anyhow!("no active workspace"))?;
        let tab = self.spawn_session(cols, rows)?;
        if !self.workspaces[self.active].split(tab, axis, area) {
            bail!("active pane is missing from split tree");
        }
        self.on_pane_focus_changed()?;
        Ok(())
    }

    /// Update visible host geometry immediately without flooding every child
    /// PTY with intermediate sizes while the window is being dragged.
    pub fn preview_resize(&mut self, cols: u16, rows: u16) -> bool {
        let cols = cols.max(1);
        let rows = rows.max(1);

        // A host terminal is allowed to reflow or discard physical cells when
        // its width changes. Our diff caches describe the pre-resize host
        // screen, so preserving their overlap can permanently skip cells that
        // no longer exist (tmux does this during resize bursts). Invalidate on
        // every observed resize event, even if a burst returned to the same
        // final dimensions before we sampled it.
        self.invalidate_host_render_state();

        if self.cols == cols && self.rows == rows {
            return false;
        }

        self.cols = cols;
        self.rows = rows;
        // Layout clamps the effective sidebar width for small windows. Keep
        // the preferred width so it returns when the window grows again.
        self.stage_workspace_resize();
        true
    }

    fn invalidate_host_render_state(&mut self) {
        self.sidebar_cache.invalidate();
        self.sidebar_fp.clear();
        if let Some(workspace) = self.workspaces.get_mut(self.active) {
            workspace.invalidate_render();
        }
        self.dirty_ui = true;
    }

    /// Send one final resize to all pane PTYs after host geometry settles.
    pub fn commit_resize_if_due(&mut self) -> Result<bool> {
        let Some(deadline) = self.resize_commit_at else {
            return Ok(false);
        };
        let now = Instant::now();
        if now < deadline {
            return Ok(false);
        }

        let area = self.layout().terminal_rect();
        for workspace in &mut self.workspaces {
            workspace.resize(area)?;
        }
        self.resize_commit_at = None;
        self.resize_repaint_at = Some(now + RESIZE_CHILD_REDRAW_DELAY);
        self.cursor_settle_until = Some(now + CURSOR_SETTLE_DELAY);
        self.dirty_ui = true;
        Ok(true)
    }

    pub fn poll_ptys(&mut self) -> Result<bool> {
        let area = self.layout().terminal_rect();
        let mut any = false;
        let mut active_output_bytes = 0usize;
        for (i, workspace) in self.workspaces.iter_mut().enumerate() {
            let poll = workspace.poll((i == self.active).then_some(area))?;
            any |= poll.visible_changed || poll.sidebar_changed;
            if i == self.active {
                active_output_bytes = active_output_bytes.saturating_add(poll.active_output_bytes);
            }
        }

        if active_output_bytes > 0 {
            let now = Instant::now();
            self.output_burst_started.get_or_insert(now);
            self.output_quiet_until = Some(now + OUTPUT_QUIET_DELAY);
            self.output_burst_bytes = self.output_burst_bytes.saturating_add(active_output_bytes);
            if self.cursor_settle_until.is_some()
                || self.output_burst_bytes >= CURSOR_SUPPRESS_BURST_BYTES
            {
                self.cursor_settle_until = Some(now + CURSOR_SETTLE_DELAY);
            }
        }
        if any {
            self.dirty_ui = true;
        }
        Ok(any)
    }

    /// Returns true when the app should exit.
    pub fn reap(&mut self) -> Result<bool> {
        if self.quit_requested {
            return Ok(true);
        }

        let area = self.layout().terminal_rect();
        let mut i = 0;
        let mut active_geometry_changed = false;
        while i < self.workspaces.len() {
            let geometry_changed = self.workspaces[i].reap();
            if self.workspaces[i].is_empty() {
                self.workspaces.remove(i);
                if self.workspaces.is_empty() {
                    return Ok(true);
                }
                if i < self.active {
                    self.active -= 1;
                } else if i == self.active {
                    self.active = self.active.min(self.workspaces.len() - 1);
                    self.workspaces[self.active].invalidate_render();
                    self.workspaces[self.active].resize(area)?;
                    self.force_draw = true;
                    self.cursor_settle_until = None;
                }
                self.sidebar_fp.clear();
                self.dirty_ui = true;
                continue;
            }
            if geometry_changed {
                // Inactive workspaces are reaped too. Resize their surviving
                // panes now so selecting one later never exposes stale PTY
                // geometry from the collapsed split.
                self.workspaces[i].resize(area)?;
                // Removing the active pane of a background workspace changes
                // the process/title shown by that workspace's sidebar card.
                // This transition happens after Workspace::poll, so schedule
                // the metadata repaint explicitly instead of leaving a stale
                // card until some unrelated event occurs.
                self.sidebar_fp.clear();
                self.dirty_ui = true;
                if i == self.active {
                    active_geometry_changed = true;
                }
            }
            i += 1;
        }

        if active_geometry_changed {
            self.resize_repaint_at = None;
            self.cursor_settle_until = None;
            self.sidebar_fp.clear();
            self.dirty_ui = true;
        }
        Ok(false)
    }

    /// Stop every child before the terminal guard restores the host. Used by
    /// both keyboard quit and the graceful Unix signal path.
    pub fn shutdown(&mut self) {
        for workspace in &mut self.workspaces {
            workspace.kill_all();
        }
    }

    fn sync_active_workspace_geometry(&mut self) -> Result<()> {
        let area = self.layout().terminal_rect();
        let Some(workspace) = self.workspaces.get_mut(self.active) else {
            bail!("no active workspace");
        };
        workspace.resize(area)?;
        self.resize_repaint_at = None;
        self.dirty_ui = true;
        Ok(())
    }

    fn stage_workspace_resize(&mut self) {
        if let Some(workspace) = self.workspaces.get_mut(self.active) {
            workspace.mark_geometry_dirty();
        }
        self.viewport_dirty = true;
        self.resize_commit_at = Some(Instant::now() + RESIZE_SETTLE_DELAY);
        self.resize_repaint_at = None;
        self.sidebar_fp.clear();
        self.dirty_ui = true;
    }

    /// Close a workspace by sidebar index. Returns true when closing the last
    /// workspace requests application exit.
    fn close_workspace(&mut self, idx: usize) -> Result<bool> {
        match workspace_close_transition(self.workspaces.len(), self.active, idx) {
            WorkspaceCloseTransition::Ignore => Ok(false),
            WorkspaceCloseTransition::Quit => {
                // Keep the final workspace structurally valid until the event
                // loop observes the quit latch. Mouse events cannot directly
                // return the loop-control boolean used by keyboard actions.
                self.workspaces[idx].kill_all();
                self.quit_requested = true;
                Ok(true)
            }
            WorkspaceCloseTransition::Active(next_active) => {
                self.workspaces[idx].kill_all();
                self.workspaces.remove(idx);
                self.active = next_active;
                self.workspaces[self.active].invalidate_render();
                self.sync_active_workspace_geometry()?;
                self.cursor_settle_until = None;
                self.force_draw = true;
                self.sidebar_fp.clear();
                self.dirty_ui = true;
                Ok(false)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceCloseTransition {
    Ignore,
    Quit,
    Active(usize),
}

fn workspace_close_transition(
    workspace_count: usize,
    active: usize,
    closing: usize,
) -> WorkspaceCloseTransition {
    if workspace_count == 0 || active >= workspace_count || closing >= workspace_count {
        return WorkspaceCloseTransition::Ignore;
    }
    if workspace_count == 1 {
        return WorkspaceCloseTransition::Quit;
    }

    let active = if closing < active {
        active - 1
    } else if closing == active {
        active.min(workspace_count - 2)
    } else {
        active
    };
    WorkspaceCloseTransition::Active(active)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_final_workspace_requests_quit_without_empty_state() {
        assert_eq!(
            workspace_close_transition(1, 0, 0),
            WorkspaceCloseTransition::Quit
        );
        assert_eq!(
            workspace_close_transition(0, 0, 0),
            WorkspaceCloseTransition::Ignore
        );
    }

    #[test]
    fn middle_close_preserves_a_valid_active_workspace() {
        // Closing a background workspace before the active one preserves the
        // same logical workspace at its shifted index.
        assert_eq!(
            workspace_close_transition(4, 2, 0),
            WorkspaceCloseTransition::Active(1)
        );
        assert_eq!(
            workspace_close_transition(4, 1, 3),
            WorkspaceCloseTransition::Active(1)
        );
        // Closing the active final item selects the preceding workspace.
        assert_eq!(
            workspace_close_transition(4, 3, 3),
            WorkspaceCloseTransition::Active(2)
        );
        assert_eq!(
            workspace_close_transition(4, 1, 1),
            WorkspaceCloseTransition::Active(1)
        );
    }
}
