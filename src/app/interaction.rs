use std::time::Instant;

use anyhow::{bail, Result};
use crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};

use crate::clip;
use crate::input::{self, Action};
use crate::render;
use crate::workspace::{PaneDirection, SplitAxis};

use super::App;

const SIDEBAR_MIN: u16 = 10;
/// Leave at least this many columns for the terminal pane.
const TERM_MIN_COLS: u16 = 20;

impl App {
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
        self.sync_sidebar_animation(Instant::now());
        Ok(false)
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

    pub(super) fn on_pane_focus_changed(&mut self) -> Result<()> {
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
}
