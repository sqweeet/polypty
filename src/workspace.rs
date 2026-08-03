use std::io::Write;

use anyhow::Result;

use crate::agent;
use crate::info::TabInfo;
use crate::render::{self, Divider, TermCache, TerminalRect};
use crate::tab::Tab;

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

#[derive(Debug, Clone)]
enum SplitNode {
    Leaf(u64),
    Split {
        axis: SplitAxis,
        /// Horizontal splits can keep the original pane compact instead of
        /// reintroducing blank rows whenever the host window grows.
        first_extent: Option<u16>,
        first: Box<SplitNode>,
        second: Box<SplitNode>,
    },
}

impl SplitNode {
    fn contains(&self, id: u64) -> bool {
        match self {
            Self::Leaf(leaf) => *leaf == id,
            Self::Split { first, second, .. } => first.contains(id) || second.contains(id),
        }
    }

    fn first_leaf(&self) -> u64 {
        match self {
            Self::Leaf(id) => *id,
            Self::Split { first, .. } => first.first_leaf(),
        }
    }

    fn replace_leaf(
        &mut self,
        target: u64,
        new_id: u64,
        axis: SplitAxis,
        first_extent: Option<u16>,
    ) -> bool {
        match self {
            Self::Leaf(id) if *id == target => {
                *self = Self::Split {
                    axis,
                    first_extent,
                    first: Box::new(Self::Leaf(target)),
                    second: Box::new(Self::Leaf(new_id)),
                };
                true
            }
            Self::Leaf(_) => false,
            Self::Split { first, second, .. } => {
                first.replace_leaf(target, new_id, axis, first_extent)
                    || second.replace_leaf(target, new_id, axis, first_extent)
            }
        }
    }

    fn remove(self, target: u64) -> (Option<Self>, bool) {
        match self {
            Self::Leaf(id) if id == target => (None, true),
            Self::Leaf(id) => (Some(Self::Leaf(id)), false),
            Self::Split {
                axis,
                first_extent,
                first,
                second,
            } => {
                let (first, removed) = first.remove(target);
                if removed {
                    return (
                        match first {
                            Some(first) => Some(Self::Split {
                                axis,
                                first_extent,
                                first: Box::new(first),
                                second,
                            }),
                            None => Some(*second),
                        },
                        true,
                    );
                }

                let first = first.expect("unchanged split branch");
                let (second, removed) = second.remove(target);
                if removed {
                    (
                        match second {
                            Some(second) => Some(Self::Split {
                                axis,
                                first_extent,
                                first: Box::new(first),
                                second: Box::new(second),
                            }),
                            None => Some(first),
                        },
                        true,
                    )
                } else {
                    (
                        Some(Self::Split {
                            axis,
                            first_extent,
                            first: Box::new(first),
                            second: Box::new(second.expect("unchanged split branch")),
                        }),
                        false,
                    )
                }
            }
        }
    }

    fn layout(
        &self,
        rect: TerminalRect,
        active: u64,
        panes: &mut Vec<(u64, TerminalRect)>,
        dividers: &mut Vec<Divider>,
    ) {
        match self {
            Self::Leaf(id) => panes.push((*id, rect)),
            Self::Split {
                axis,
                first_extent,
                first,
                second,
            } => match axis {
                SplitAxis::Vertical if rect.cols >= 3 => {
                    let available = rect.cols - 1;
                    let first_cols = available / 2;
                    let second_cols = available - first_cols;
                    let divider_x = rect.x.saturating_add(first_cols);
                    let first_rect = TerminalRect {
                        x: rect.x,
                        y: rect.y,
                        cols: first_cols,
                        rows: rect.rows,
                    };
                    let second_rect = TerminalRect {
                        x: divider_x.saturating_add(1),
                        y: rect.y,
                        cols: second_cols,
                        rows: rect.rows,
                    };
                    dividers.push(Divider::Vertical {
                        x: divider_x,
                        y: rect.y,
                        len: rect.rows,
                    });
                    first.layout(first_rect, active, panes, dividers);
                    second.layout(second_rect, active, panes, dividers);
                }
                SplitAxis::Horizontal if rect.rows >= 3 => {
                    let available = rect.rows - 1;
                    let first_rows = first_extent
                        .unwrap_or(available / 2)
                        .clamp(1, available - 1);
                    let second_rows = available - first_rows;
                    let divider_y = rect.y.saturating_add(first_rows);
                    let first_rect = TerminalRect {
                        x: rect.x,
                        y: rect.y,
                        cols: rect.cols,
                        rows: first_rows,
                    };
                    let second_rect = TerminalRect {
                        x: rect.x,
                        y: divider_y.saturating_add(1),
                        cols: rect.cols,
                        rows: second_rows,
                    };
                    dividers.push(Divider::Horizontal {
                        x: rect.x,
                        y: divider_y,
                        len: rect.cols,
                    });
                    first.layout(first_rect, active, panes, dividers);
                    second.layout(second_rect, active, panes, dividers);
                }
                _ => {
                    // Never create zero-sized or overlapping panes. If a nested
                    // split cannot fit, show the branch containing the active pane.
                    if first.contains(active) {
                        first.layout(rect, active, panes, dividers);
                    } else {
                        second.layout(rect, active, panes, dividers);
                    }
                }
            },
        }
    }
}

#[derive(Default)]
struct WorkspaceLayout {
    panes: Vec<(u64, TerminalRect)>,
    dividers: Vec<Divider>,
}

impl WorkspaceLayout {
    fn contains_pane(&self, id: u64) -> bool {
        self.panes.iter().any(|(pane_id, _)| *pane_id == id)
    }

    fn pane_size(&self, id: u64) -> Option<(u16, u16)> {
        self.panes.iter().find_map(|(pane_id, rect)| {
            (*pane_id == id).then_some((rect.cols.max(1), rect.rows.max(1)))
        })
    }

    fn has_visible_dirty(&self, panes: impl IntoIterator<Item = (u64, bool)>) -> bool {
        panes
            .into_iter()
            .any(|(id, dirty)| dirty && self.contains_pane(id))
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

fn compact_horizontal_extent(screen: &vt100::Screen, total_rows: u16) -> Option<u16> {
    let available = total_rows.saturating_sub(1);
    let half = (available / 2).max(1);
    const MIN_ROWS: u16 = 5;
    if available < MIN_ROWS.saturating_mul(2) {
        return None;
    }

    let (screen_rows, screen_cols) = screen.size();
    let mut used = screen.cursor_position().0.saturating_add(1);
    for row in 0..screen_rows.min(total_rows) {
        let has_text = (0..screen_cols).any(|col| {
            screen.cell(row, col).is_some_and(|cell| {
                cell.contents()
                    .chars()
                    .any(|character| !character.is_whitespace())
            })
        });
        if has_text {
            used = used.max(row.saturating_add(1));
        }
    }

    // Return a fixed extent only when it removes real blank rows. Otherwise
    // leave the node dynamic so an ordinary 50/50 split keeps balancing on
    // later host resizes.
    (MIN_ROWS..half).contains(&used).then_some(used)
}

fn center(rect: TerminalRect) -> (i32, i32) {
    (
        rect.x as i32 * 2 + rect.cols as i32,
        rect.y as i32 * 2 + rect.rows as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_layout(root: &SplitNode, area: TerminalRect, active: u64) -> WorkspaceLayout {
        let mut layout = WorkspaceLayout::default();
        root.layout(area, active, &mut layout.panes, &mut layout.dividers);
        layout
    }

    #[test]
    fn nested_split_covers_area_without_overlap() {
        let mut root = SplitNode::Leaf(1);
        assert!(root.replace_leaf(1, 2, SplitAxis::Vertical, None));
        assert!(root.replace_leaf(2, 3, SplitAxis::Horizontal, Some(8)));
        let area = TerminalRect {
            x: 18,
            y: 0,
            cols: 81,
            rows: 30,
        };
        let layout = node_layout(&root, area, 3);

        assert_eq!(layout.panes.len(), 3);
        assert_eq!(layout.dividers.len(), 2);
        for (i, (_, left)) in layout.panes.iter().enumerate() {
            for (_, right) in layout.panes.iter().skip(i + 1) {
                let overlap_x = left.x < right.x + right.cols && right.x < left.x + left.cols;
                let overlap_y = left.y < right.y + right.rows && right.y < left.y + left.rows;
                assert!(!(overlap_x && overlap_y));
            }
        }
    }

    #[test]
    fn tiny_layout_shows_only_active_branch() {
        let mut root = SplitNode::Leaf(1);
        assert!(root.replace_leaf(1, 2, SplitAxis::Vertical, None));
        let layout = node_layout(
            &root,
            TerminalRect {
                x: 0,
                y: 0,
                cols: 1,
                rows: 1,
            },
            2,
        );
        assert_eq!(
            layout.panes,
            vec![(
                2,
                TerminalRect {
                    x: 0,
                    y: 0,
                    cols: 1,
                    rows: 1
                }
            )]
        );
        assert!(layout.dividers.is_empty());
    }

    #[test]
    fn collapsed_layout_ignores_hidden_dirty_panes_and_preserves_their_geometry() {
        let mut root = SplitNode::Leaf(1);
        assert!(root.replace_leaf(1, 2, SplitAxis::Vertical, None));
        let area = TerminalRect {
            x: 7,
            y: 3,
            cols: 2,
            rows: 9,
        };

        let first = node_layout(&root, area, 1);
        assert_eq!(first.panes, vec![(1, area)]);
        assert!(!first.has_visible_dirty([(2, true)]));
        assert!(first.has_visible_dirty([(1, true), (2, false)]));
        assert_eq!(first.pane_size(1), Some((2, 9)));
        // No resize is planned for a hidden pane, so its last usable parser
        // geometry is retained until focus makes it visible again.
        assert_eq!(first.pane_size(2), None);

        let second = node_layout(&root, area, 2);
        assert_eq!(second.panes, vec![(2, area)]);
        assert!(!second.has_visible_dirty([(1, true)]));
        assert!(second.has_visible_dirty([(2, true)]));
        assert_eq!(second.pane_size(2), Some((2, 9)));
        assert_eq!(second.pane_size(1), None);
    }

    #[test]
    fn nested_layout_converges_after_tiny_resize_round_trip() {
        let mut root = SplitNode::Leaf(1);
        assert!(root.replace_leaf(1, 2, SplitAxis::Vertical, None));
        assert!(root.replace_leaf(2, 3, SplitAxis::Horizontal, Some(7)));
        let large = TerminalRect {
            x: 18,
            y: 0,
            cols: 82,
            rows: 31,
        };

        let before = node_layout(&root, large, 3);
        let tiny = node_layout(
            &root,
            TerminalRect {
                cols: 2,
                rows: 2,
                ..large
            },
            3,
        );
        assert_eq!(tiny.panes.len(), 1);
        assert!(tiny.dividers.is_empty());

        let after = node_layout(&root, large, 3);
        assert_eq!(after.panes, before.panes);
        assert_eq!(after.dividers, before.dividers);
    }

    #[test]
    fn removing_leaf_collapses_parent_split() {
        let mut root = SplitNode::Leaf(1);
        assert!(root.replace_leaf(1, 2, SplitAxis::Vertical, None));
        let (root, removed) = root.remove(2);
        assert!(removed);
        assert!(matches!(root, Some(SplitNode::Leaf(1))));
    }

    #[test]
    fn horizontal_split_hugs_sparse_content() {
        let mut parser = vt100::Parser::new(38, 49, 0);
        parser.process(b"one\r\ntwo\x1b[11;1H");

        assert_eq!(compact_horizontal_extent(parser.screen(), 38), Some(11));
    }

    #[test]
    fn horizontal_split_keeps_shells_and_busy_screens_half() {
        let sparse = vt100::Parser::new(38, 49, 0);
        assert_eq!(compact_horizontal_extent(sparse.screen(), 38), None);

        let mut busy = vt100::Parser::new(38, 49, 0);
        busy.process(b"\x1b[30;1Hbusy");
        assert_eq!(compact_horizontal_extent(busy.screen(), 38), None);
    }

    #[test]
    fn compact_extent_survives_growth_and_clamps_without_zero_panes() {
        let mut root = SplitNode::Leaf(1);
        assert!(root.replace_leaf(1, 2, SplitAxis::Horizontal, Some(11)));
        let area = TerminalRect {
            x: 0,
            y: 0,
            cols: 20,
            rows: 60,
        };
        let grown = node_layout(&root, area, 2);
        assert_eq!(grown.panes[0].1.rows, 11);
        assert_eq!(grown.panes[1].1.rows, 48);

        let tiny = node_layout(&root, TerminalRect { rows: 8, ..area }, 2);
        assert_eq!(tiny.panes[0].1.rows, 6);
        assert_eq!(tiny.panes[1].1.rows, 1);
    }
}
