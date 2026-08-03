use crate::core::geometry::TerminalRect;

use super::{layout::WorkspaceLayout, pane::PaneStore, PaneDirection};

/// Tracks focus and owns every focus-selection policy.
pub(super) struct FocusModel {
    active: u64,
}

impl FocusModel {
    pub(super) fn new(active: u64) -> Self {
        Self { active }
    }

    pub(super) fn active(&self) -> u64 {
        self.active
    }

    pub(super) fn activate(&mut self, id: u64) -> bool {
        if id == self.active {
            return false;
        }
        self.active = id;
        true
    }

    pub(super) fn activate_next(&mut self, panes: &PaneStore) -> bool {
        panes
            .next_after(self.active)
            .is_some_and(|id| self.activate(id))
    }

    pub(super) fn activate_at(&mut self, layout: &WorkspaceLayout, x: u16, y: u16) -> bool {
        layout
            .panes
            .iter()
            .find_map(|(id, rect)| rect.contains(x, y).then_some(*id))
            .is_some_and(|id| self.activate(id))
    }

    pub(super) fn activate_direction(
        &mut self,
        layout: &WorkspaceLayout,
        direction: PaneDirection,
    ) -> bool {
        let Some(active_rect) = layout.rect(self.active) else {
            return false;
        };
        let origin = center(active_rect);
        layout
            .panes
            .iter()
            .filter(|(id, _)| *id != self.active)
            .filter_map(|(id, rect)| {
                directional_score(origin, center(*rect), direction).map(|s| (s, *id))
            })
            .min_by_key(|(score, _)| *score)
            .is_some_and(|(_, id)| self.activate(id))
    }
}

fn directional_score(
    origin: (i32, i32),
    candidate: (i32, i32),
    direction: PaneDirection,
) -> Option<i32> {
    let (in_direction, primary, secondary) = match direction {
        PaneDirection::Left => (
            candidate.0 < origin.0,
            origin.0 - candidate.0,
            (origin.1 - candidate.1).abs(),
        ),
        PaneDirection::Right => (
            candidate.0 > origin.0,
            candidate.0 - origin.0,
            (origin.1 - candidate.1).abs(),
        ),
        PaneDirection::Up => (
            candidate.1 < origin.1,
            origin.1 - candidate.1,
            (origin.0 - candidate.0).abs(),
        ),
        PaneDirection::Down => (
            candidate.1 > origin.1,
            candidate.1 - origin.1,
            (origin.0 - candidate.0).abs(),
        ),
    };
    in_direction.then(|| primary.saturating_mul(10_000).saturating_add(secondary))
}

fn center(rect: TerminalRect) -> (i32, i32) {
    (
        rect.x as i32 * 2 + rect.cols as i32,
        rect.y as i32 * 2 + rect.rows as i32,
    )
}
