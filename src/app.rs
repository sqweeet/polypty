use std::io::Write;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};

use crate::clip;
use crate::input::{self, Action};
use crate::render::{self, Layout, SidebarCache, SidebarMap, SidebarTab};
use crate::tab::Tab;
use crate::workspace::{PaneDirection, SplitAxis, Workspace};

const SIDEBAR_MIN: u16 = 10;
const SIDEBAR_DEFAULT: u16 = 18;
/// Leave at least this many columns for the terminal pane.
const TERM_MIN_COLS: u16 = 20;
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
const OUTPUT_MAX_FRAME_DELAY: Duration = Duration::from_millis(16);
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

    /// Returns true if the app should quit.
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        match input::map_key(key) {
            Action::Quit => {
                self.shutdown();
                return Ok(true);
            }
            Action::NewTab => self.spawn_workspace()?,
            Action::CloseTab => {
                if self.close_workspace(self.active)? {
                    return Ok(true);
                }
            }
            Action::NextTab => self.select_next()?,
            Action::PrevTab => self.select_prev()?,
            Action::Tab(n) => {
                let idx = (n as usize).saturating_sub(1);
                self.select_workspace(idx)?;
            }
            Action::SplitVertical => self.split_active(SplitAxis::Vertical)?,
            Action::SplitHorizontal => self.split_active(SplitAxis::Horizontal)?,
            Action::ClosePane => {
                let close_workspace = self
                    .workspaces
                    .get(self.active)
                    .is_none_or(|workspace| workspace.pane_count() <= 1);
                if close_workspace {
                    if self.close_workspace(self.active)? {
                        return Ok(true);
                    }
                } else if self.workspaces[self.active].close_active_pane() {
                    self.on_pane_focus_changed()?;
                }
            }
            Action::NextPane => {
                if self.workspaces[self.active].focus_next() {
                    self.on_pane_focus_changed()?;
                }
            }
            Action::PaneLeft => self.focus_pane(PaneDirection::Left)?,
            Action::PaneRight => self.focus_pane(PaneDirection::Right)?,
            Action::PaneUp => self.focus_pane(PaneDirection::Up)?,
            Action::PaneDown => self.focus_pane(PaneDirection::Down)?,
            Action::ToggleSidebar => {
                self.sidebar_visible = !self.sidebar_visible;
                self.stage_workspace_resize();
            }
            Action::SidebarWider => {
                if self.sidebar_visible {
                    self.adjust_sidebar(2);
                } else {
                    self.sidebar_visible = true;
                    let max = self.sidebar_max();
                    self.sidebar_width =
                        self.sidebar_width.saturating_add(2).clamp(SIDEBAR_MIN, max);
                    self.stage_workspace_resize();
                }
            }
            Action::SidebarNarrower => {
                if self.sidebar_visible {
                    self.adjust_sidebar(-2);
                }
            }
            Action::PasteClipboard => {
                if let Some(text) = clip::read_clipboard() {
                    self.handle_paste(&text)?;
                }
                return Ok(false);
            }
            Action::Forward => {
                let (app_cursor, app_keypad) = self
                    .workspaces
                    .get(self.active)
                    .map(|workspace| {
                        let screen = workspace.active_screen();
                        (screen.application_cursor(), screen.application_keypad())
                    })
                    .unwrap_or((false, false));
                let bytes = input::encode_key(key, app_cursor, app_keypad);
                if !bytes.is_empty() {
                    self.write_active(&bytes)?;
                }
                return Ok(false);
            }
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

    /// Mouse: click tabs/panes, scroll tabs, drag sidebar edge, and pass
    /// application mouse events to TUIs using pane-local coordinates.
    /// Returns true if mux chrome/focus should repaint immediately.
    pub fn handle_mouse(&mut self, ev: MouseEvent) -> Result<bool> {
        let layout = self.layout();
        let area = layout.terminal_rect();
        let grip = 3u16;

        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if layout.sidebar_visible {
                    let on_sidebar = ev.column < layout.sidebar_width;
                    let on_grip =
                        on_sidebar && ev.column.saturating_add(grip) >= layout.sidebar_width;

                    // Never let the resize hitbox spill into the pane. The
                    // first terminal columns must remain clickable by TUIs.
                    if on_grip {
                        self.dragging_sidebar = true;
                        return Ok(self.set_sidebar_width(ev.column.max(1)));
                    }
                    if on_sidebar {
                        if let Some(idx) = self.sidebar_map.tab_at(ev.column, ev.row) {
                            return self.select_workspace(idx);
                        }
                        return Ok(false);
                    }
                }

                if area.contains(ev.column, ev.row) {
                    let focus_changed =
                        self.workspaces[self.active].focus_at(ev.column, ev.row, area);
                    if focus_changed {
                        self.on_pane_focus_changed()?;
                    }
                    self.forward_mouse_to_active(ev, area)?;
                    return Ok(focus_changed);
                }
            }
            MouseEventKind::Drag(MouseButton::Left) if self.dragging_sidebar => {
                return Ok(self.set_sidebar_width(ev.column.max(1)));
            }
            MouseEventKind::Moved if self.dragging_sidebar => {
                return Ok(self.set_sidebar_width(ev.column.max(1)));
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if self.dragging_sidebar {
                    self.dragging_sidebar = false;
                    return Ok(true);
                }
            }
            MouseEventKind::Down(MouseButton::Middle) => {
                if layout.sidebar_visible && ev.column < layout.sidebar_width {
                    if let Some(idx) = self.sidebar_map.tab_at(ev.column, ev.row) {
                        self.close_workspace(idx)?;
                        return Ok(true);
                    }
                    return Ok(false);
                }
                if area.contains(ev.column, ev.row) {
                    let focus_changed =
                        self.workspaces[self.active].focus_at(ev.column, ev.row, area);
                    if focus_changed {
                        self.on_pane_focus_changed()?;
                    }
                    // A child that requested mouse tracking owns middle-click.
                    // Otherwise retain the traditional PRIMARY paste behavior.
                    if self.forward_mouse_to_active(ev, area)? {
                        return Ok(focus_changed);
                    }
                    if let Some(text) = clip::read_primary() {
                        self.handle_paste(&text)?;
                    }
                    return Ok(true);
                }
            }
            MouseEventKind::ScrollUp
                if layout.sidebar_visible && ev.column < layout.sidebar_width =>
            {
                self.select_prev()?;
                return Ok(true);
            }
            MouseEventKind::ScrollDown
                if layout.sidebar_visible && ev.column < layout.sidebar_width =>
            {
                self.select_next()?;
                return Ok(true);
            }
            _ => {}
        }

        if area.contains(ev.column, ev.row) {
            self.forward_mouse_to_active(ev, area)?;
        }
        Ok(false)
    }

    fn forward_mouse_to_active(
        &mut self,
        ev: MouseEvent,
        area: render::TerminalRect,
    ) -> Result<bool> {
        let bytes = {
            let Some(workspace) = self.workspaces.get(self.active) else {
                return Ok(false);
            };
            let Some((col, row)) = workspace.active_cell_at(ev.column, ev.row, area) else {
                return Ok(false);
            };
            let screen = workspace.active_screen();
            input::encode_mouse(
                ev,
                col,
                row,
                screen.mouse_protocol_mode(),
                screen.mouse_protocol_encoding(),
            )
        };
        if bytes.is_empty() {
            return Ok(false);
        }
        self.write_active(&bytes)?;
        Ok(true)
    }

    fn focus_pane(&mut self, direction: PaneDirection) -> Result<()> {
        let area = self.layout().terminal_rect();
        if self.workspaces[self.active].focus_direction(direction, area) {
            self.on_pane_focus_changed()?;
        }
        Ok(())
    }

    fn on_pane_focus_changed(&mut self) -> Result<()> {
        // In a tiny viewport, layout hides whole split branches. As soon as
        // focus selects one of those panes, synchronize both the PTY and the
        // parser before forwarding input or painting its newly visible frame.
        self.sync_active_workspace_geometry()?;
        self.cursor_settle_until = None;
        self.sidebar_fp.clear();
        self.dirty_ui = true;
        Ok(())
    }

    fn select_workspace(&mut self, idx: usize) -> Result<bool> {
        if idx < self.workspaces.len() && idx != self.active {
            self.active = idx;
            self.workspaces[self.active].invalidate_render();
            self.sync_active_workspace_geometry()?;
            self.cursor_settle_until = None;
            self.force_draw = true;
            self.sidebar_fp.clear();
            self.dirty_ui = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn select_next(&mut self) -> Result<()> {
        if !self.workspaces.is_empty() {
            let idx = (self.active + 1) % self.workspaces.len();
            self.select_workspace(idx)?;
        }
        Ok(())
    }

    fn select_prev(&mut self) -> Result<()> {
        if !self.workspaces.is_empty() {
            let idx = if self.active == 0 {
                self.workspaces.len() - 1
            } else {
                self.active - 1
            };
            self.select_workspace(idx)?;
        }
        Ok(())
    }

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
        self.cursor_settle_until
            .is_some_and(|deadline| now >= deadline)
    }

    fn output_frame_ready(&self, now: Instant) -> bool {
        is_output_frame_ready(self.output_burst_started, self.output_quiet_until, now)
    }

    pub fn handle_paste(&mut self, text: &str) -> Result<()> {
        let bracketed = self
            .workspaces
            .get(self.active)
            .map(|workspace| workspace.active_screen().bracketed_paste())
            .unwrap_or(false);
        if bracketed {
            let mut buf = Vec::with_capacity(text.len() + 16);
            buf.extend_from_slice(b"\x1b[200~");
            buf.extend_from_slice(text.as_bytes());
            buf.extend_from_slice(b"\x1b[201~");
            self.write_active(&buf)
        } else {
            self.write_active(text.as_bytes())
        }
    }

    fn write_active(&mut self, data: &[u8]) -> Result<()> {
        let Some(workspace) = self.workspaces.get_mut(self.active) else {
            bail!("no active workspace");
        };
        workspace.write_active(data)
    }

    fn sidebar_max(&self) -> u16 {
        self.cols.saturating_sub(TERM_MIN_COLS).max(SIDEBAR_MIN)
    }

    fn adjust_sidebar(&mut self, delta: i16) {
        let max = self.sidebar_max();
        let next = (self.sidebar_width as i16 + delta).clamp(SIDEBAR_MIN as i16, max as i16) as u16;
        if next != self.sidebar_width {
            self.sidebar_width = next;
            self.stage_workspace_resize();
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

    fn set_sidebar_width(&mut self, width: u16) -> bool {
        let max = self.sidebar_max();
        let width = width.clamp(SIDEBAR_MIN, max);
        if width == self.sidebar_width && self.sidebar_visible {
            return false;
        }
        self.sidebar_width = width;
        self.sidebar_visible = true;
        self.stage_workspace_resize();
        true
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
                SidebarTab {
                    primary: info.primary,
                    secondary: info.secondary,
                    agent: info.agent,
                    active: i == active,
                }
            })
            .collect();
        let fp = sidebar_fingerprint(
            &side_tabs,
            layout.sidebar_visible,
            layout.sidebar_width,
            layout.rows,
        );
        let need_sidebar = layout.sidebar_visible && (force || hard || fp != self.sidebar_fp);
        let need_workspace = need_sidebar
            || force
            || hard
            || self.viewport_dirty
            || need_cursor_restore
            || self
                .workspaces
                .get(active)
                .is_some_and(|workspace| workspace.needs_draw(area));

        let frame_cells = (layout.cols as usize).saturating_mul(layout.rows as usize);
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
                .map(|status| format!("{}:{}", status.kind.label(), status.state.label()))
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
            primary: "codex".into(),
            secondary: "~/projects/mux".into(),
            agent: Some(crate::agent::AgentStatus {
                kind: crate::agent::AgentKind::Codex,
                state: crate::agent::AgentState::Working,
            }),
            active: true,
        }];
        let working = sidebar_fingerprint(&tabs, true, 18, 24);
        tabs[0].agent.as_mut().unwrap().state = crate::agent::AgentState::Blocked;
        let blocked = sidebar_fingerprint(&tabs, true, 18, 24);

        assert_ne!(working, blocked);
    }
}
