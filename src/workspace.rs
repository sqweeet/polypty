use std::io::Write;

use anyhow::Result;

use crate::agent;
use crate::info::TabInfo;
use crate::render::{self, TermCache, TerminalRect};
use crate::tab::Tab;

mod layout;

use layout::{center, compact_horizontal_extent, SplitNode, WorkspaceLayout};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitAxis {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WorkspacePoll {
    /// Output changed in a pane that is currently visible.
    pub visible_changed: bool,
    /// The active pane's sidebar metadata changed.
    pub sidebar_changed: bool,
    pub active_output_bytes: usize,
}

struct Pane {
    tab: Tab,
    cache: TermCache,
    last_rect: Option<TerminalRect>,
}

impl Pane {
    fn new(tab: Tab) -> Self {
        Self {
            tab,
            cache: TermCache::default(),
            last_rect: None,
        }
    }
}

pub struct Workspace {
    panes: Vec<Pane>,
    root: Option<SplitNode>,
    active: u64,
    chrome_dirty: bool,
    focus_dirty: bool,
}

impl Workspace {
    pub fn new(tab: Tab) -> Self {
        let active = tab.id;
        Self {
            panes: vec![Pane::new(tab)],
            root: Some(SplitNode::Leaf(active)),
            active,
            chrome_dirty: true,
            focus_dirty: true,
        }
    }

    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    pub fn active_tab(&self) -> &Tab {
        &self
            .panes
            .iter()
            .find(|pane| pane.tab.id == self.active)
            .expect("active pane exists")
            .tab
    }

    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self
            .panes
            .iter_mut()
            .find(|pane| pane.tab.id == self.active)
            .expect("active pane exists")
            .tab
    }

    pub fn info(&self) -> TabInfo {
        let mut info = self.active_tab().info.clone();
        let active = self.active;
        info.agent = agent::rollup(
            self.panes
                .iter()
                .filter(|pane| pane.tab.id == active)
                .chain(self.panes.iter().filter(|pane| pane.tab.id != active))
                .filter_map(|pane| pane.tab.info.agent),
        );
        info
    }

    pub fn active_screen(&self) -> &vt100::Screen {
        self.active_tab().screen()
    }

    pub fn split_size(&self, axis: SplitAxis, area: TerminalRect) -> (u16, u16) {
        let rect = self.active_rect(area).unwrap_or(area);
        match axis {
            SplitAxis::Vertical if rect.cols >= 3 => {
                let available = rect.cols - 1;
                (available - available / 2, rect.rows.max(1))
            }
            SplitAxis::Horizontal if rect.rows >= 3 => {
                let available = rect.rows - 1;
                let first_rows = compact_horizontal_extent(self.active_screen(), rect.rows)
                    .unwrap_or(available / 2);
                (rect.cols.max(1), available - first_rows)
            }
            _ => (1, 1),
        }
    }

    pub fn split(&mut self, tab: Tab, axis: SplitAxis, area: TerminalRect) -> bool {
        let new_id = tab.id;
        let first_extent = if axis == SplitAxis::Horizontal {
            self.active_rect(area)
                .and_then(|rect| compact_horizontal_extent(self.active_screen(), rect.rows))
        } else {
            None
        };
        let Some(root) = self.root.as_mut() else {
            return false;
        };
        if !root.replace_leaf(self.active, new_id, axis, first_extent) {
            return false;
        }
        self.panes.push(Pane::new(tab));
        self.active = new_id;
        self.chrome_dirty = true;
        self.focus_dirty = true;
        true
    }

    /// Close only the active pane. Returns false when it is the last pane.
    pub fn close_active_pane(&mut self) -> bool {
        if self.panes.len() <= 1 {
            return false;
        }
        let id = self.active;
        if let Some(pane) = self.panes.iter_mut().find(|pane| pane.tab.id == id) {
            pane.tab.kill();
        }
        self.remove_pane(id)
    }

    pub fn focus_next(&mut self) -> bool {
        if self.panes.len() <= 1 {
            return false;
        }
        let current = self
            .panes
            .iter()
            .position(|pane| pane.tab.id == self.active)
            .unwrap_or(0);
        self.active = self.panes[(current + 1) % self.panes.len()].tab.id;
        self.focus_dirty = true;
        true
    }

    pub fn focus_direction(&mut self, direction: PaneDirection, area: TerminalRect) -> bool {
        let layout = self.layout(area);
        let Some((_, active_rect)) = layout
            .panes
            .iter()
            .find(|(id, _)| *id == self.active)
            .copied()
        else {
            return false;
        };

        let active_center = center(active_rect);
        let mut best: Option<(i32, u64)> = None;
        for (id, rect) in layout.panes {
            if id == self.active {
                continue;
            }
            let candidate_center = center(rect);
            let (in_direction, primary, secondary) = match direction {
                PaneDirection::Left => (
                    candidate_center.0 < active_center.0,
                    active_center.0 - candidate_center.0,
                    (active_center.1 - candidate_center.1).abs(),
                ),
                PaneDirection::Right => (
                    candidate_center.0 > active_center.0,
                    candidate_center.0 - active_center.0,
                    (active_center.1 - candidate_center.1).abs(),
                ),
                PaneDirection::Up => (
                    candidate_center.1 < active_center.1,
                    active_center.1 - candidate_center.1,
                    (active_center.0 - candidate_center.0).abs(),
                ),
                PaneDirection::Down => (
                    candidate_center.1 > active_center.1,
                    candidate_center.1 - active_center.1,
                    (active_center.0 - candidate_center.0).abs(),
                ),
            };
            if !in_direction {
                continue;
            }
            let score = primary.saturating_mul(10_000).saturating_add(secondary);
            if best.is_none_or(|(best_score, _)| score < best_score) {
                best = Some((score, id));
            }
        }

        if let Some((_, id)) = best {
            self.active = id;
            self.focus_dirty = true;
            true
        } else {
            false
        }
    }

    pub fn focus_at(&mut self, x: u16, y: u16, area: TerminalRect) -> bool {
        let layout = self.layout(area);
        let Some(id) = layout
            .panes
            .iter()
            .find_map(|(id, rect)| rect.contains(x, y).then_some(*id))
        else {
            return false;
        };
        if id == self.active {
            return false;
        }
        self.active = id;
        self.focus_dirty = true;
        true
    }

    /// Translate a host cell into the active pane's own zero-based grid.
    pub fn active_cell_at(&self, x: u16, y: u16, area: TerminalRect) -> Option<(u16, u16)> {
        self.layout(area)
            .panes
            .into_iter()
            .find(|(id, rect)| *id == self.active && rect.contains(x, y))
            .map(|(_, rect)| (x - rect.x, y - rect.y))
    }

    pub fn poll(&mut self, visible_area: Option<TerminalRect>) -> Result<WorkspacePoll> {
        let mut result = WorkspacePoll::default();
        let visible_layout = visible_area.map(|area| self.layout(area));
        let sidebar_before = self.info();
        for pane in &mut self.panes {
            let changed = pane.tab.poll()?;
            if changed
                && visible_layout
                    .as_ref()
                    .is_some_and(|layout| layout.contains_pane(pane.tab.id))
            {
                result.visible_changed = true;
            }
            if pane.tab.id == self.active {
                result.active_output_bytes = result
                    .active_output_bytes
                    .saturating_add(pane.tab.last_poll_bytes());
            }
        }
        result.sidebar_changed = self.info() != sidebar_before;
        Ok(result)
    }

    /// Reap exited panes. Returns true when split geometry changed.
    pub fn reap(&mut self) -> bool {
        let dead: Vec<u64> = self
            .panes
            .iter_mut()
            .filter_map(|pane| (pane.tab.try_reap() && !pane.tab.alive).then_some(pane.tab.id))
            .collect();
        let mut changed = false;
        for id in dead {
            if let Some(pane) = self.panes.iter_mut().find(|pane| pane.tab.id == id) {
                let _ = pane.tab.poll();
            }
            changed |= self.remove_pane(id);
        }
        changed
    }

    pub fn is_empty(&self) -> bool {
        self.panes.is_empty()
    }

    pub fn resize(&mut self, area: TerminalRect) -> Result<()> {
        let layout = self.layout(area);
        for pane in &mut self.panes {
            // A split can be temporarily collapsed when the host is tiny.
            // Preserve hidden TUIs at their last usable size instead of
            // making them destructively reflow into a 1x1 grid. Focus changes
            // call this method again after making the selected branch visible.
            if let Some((cols, rows)) = layout.pane_size(pane.tab.id) {
                pane.tab.resize(cols, rows)?;
            }
        }
        Ok(())
    }

    pub fn mark_geometry_dirty(&mut self) {
        self.chrome_dirty = true;
    }

    pub fn invalidate_render(&mut self) {
        for pane in &mut self.panes {
            pane.cache.invalidate();
            pane.last_rect = None;
        }
        self.chrome_dirty = true;
        self.focus_dirty = true;
    }

    pub fn reset_blank(&mut self, area: TerminalRect) {
        let layout = self.layout(area);
        for pane in &mut self.panes {
            if let Some(rect) = layout
                .panes
                .iter()
                .find_map(|(id, rect)| (pane.tab.id == *id).then_some(*rect))
            {
                pane.cache.reset_blank(rect.cols, rect.rows);
                pane.last_rect = Some(rect);
            } else {
                pane.cache.invalidate();
                pane.last_rect = None;
            }
        }
        self.chrome_dirty = true;
        self.focus_dirty = true;
    }

    pub fn needs_draw(&self, area: TerminalRect) -> bool {
        if self.chrome_dirty || self.focus_dirty {
            return true;
        }

        let layout = self.layout(area);
        layout.has_visible_dirty(self.panes.iter().map(|pane| (pane.tab.id, pane.tab.dirty)))
    }

    pub fn draw(
        &mut self,
        out: &mut impl Write,
        area: TerminalRect,
        suppress_active_cursor: bool,
        force: bool,
    ) -> Result<()> {
        if force {
            self.invalidate_render();
        }

        let layout = self.layout(area);
        let mut geometry_changed = false;
        for pane in &mut self.panes {
            let rect = layout
                .panes
                .iter()
                .find_map(|(id, rect)| (pane.tab.id == *id).then_some(*rect));
            match rect {
                Some(rect) => {
                    if pane.last_rect != Some(rect) {
                        if pane
                            .last_rect
                            .is_none_or(|old| old.x != rect.x || old.y != rect.y)
                        {
                            pane.cache.invalidate();
                        }
                        pane.last_rect = Some(rect);
                        geometry_changed = true;
                    }
                }
                None => {
                    if pane.last_rect.take().is_some() {
                        pane.cache.invalidate();
                        geometry_changed = true;
                    }
                }
            }
        }

        if self.chrome_dirty || geometry_changed {
            render::draw_dividers(out, &layout.dividers)?;
        }

        let active = self.active;
        for (id, rect) in &layout.panes {
            if *id == active {
                continue;
            }
            let pane = self
                .panes
                .iter_mut()
                .find(|pane| pane.tab.id == *id)
                .expect("layout pane exists");
            if pane.tab.dirty || geometry_changed || force {
                render::draw_terminal_rect(
                    out,
                    *rect,
                    pane.tab.screen(),
                    &mut pane.cache,
                    false,
                    true,
                )?;
                pane.tab.dirty = false;
            }
        }

        // Draw the active pane last so it owns the one real hardware cursor.
        if let Some((_, rect)) = layout.panes.iter().find(|(id, _)| *id == active) {
            let pane = self
                .panes
                .iter_mut()
                .find(|pane| pane.tab.id == active)
                .expect("active pane exists");
            render::draw_terminal_rect(
                out,
                *rect,
                pane.tab.screen(),
                &mut pane.cache,
                false,
                suppress_active_cursor,
            )?;
            pane.tab.dirty = false;
        }

        self.chrome_dirty = false;
        self.focus_dirty = false;
        Ok(())
    }

    pub fn restore_active_cursor(
        &self,
        out: &mut impl Write,
        area: TerminalRect,
        suppress_cursor: bool,
    ) -> Result<()> {
        let layout = self.layout(area);
        let Some((_, rect)) = layout.panes.iter().find(|(id, _)| *id == self.active) else {
            return Ok(());
        };
        let pane = self
            .panes
            .iter()
            .find(|pane| pane.tab.id == self.active)
            .expect("active pane exists");
        render::restore_terminal_cursor(out, *rect, pane.tab.screen(), suppress_cursor)
    }

    pub fn write_active(&mut self, data: &[u8]) -> Result<()> {
        self.active_tab_mut().write_all(data)
    }

    pub fn kill_all(&mut self) {
        for pane in &mut self.panes {
            pane.tab.kill();
        }
    }

    fn active_rect(&self, area: TerminalRect) -> Option<TerminalRect> {
        self.layout(area)
            .panes
            .into_iter()
            .find_map(|(id, rect)| (id == self.active).then_some(rect))
    }

    fn layout(&self, area: TerminalRect) -> WorkspaceLayout {
        let mut layout = WorkspaceLayout::default();
        if let Some(root) = &self.root {
            root.layout(
                TerminalRect {
                    cols: area.cols.max(1),
                    rows: area.rows.max(1),
                    ..area
                },
                self.active,
                &mut layout.panes,
                &mut layout.dividers,
            );
        }
        layout
    }

    fn remove_pane(&mut self, id: u64) -> bool {
        let Some(index) = self.panes.iter().position(|pane| pane.tab.id == id) else {
            return false;
        };
        self.panes.remove(index);

        if let Some(root) = self.root.take() {
            let (root, removed) = root.remove(id);
            debug_assert!(removed);
            self.root = root;
        }
        if !self.panes.iter().any(|pane| pane.tab.id == self.active) {
            if let Some(root) = &self.root {
                self.active = root.first_leaf();
            }
        }
        self.chrome_dirty = true;
        self.focus_dirty = true;
        true
    }
}
