use std::collections::BTreeMap;
use std::time::Instant;

use crate::agent::AgentState;
use crate::render::{GlintFrame, SidebarMap};

/// Per-workspace animation lifecycle. Rendering remains stateless: this owns
/// only the moment each card entered Working.
#[derive(Debug, Default)]
pub(super) struct SidebarAnimation {
    working_since: BTreeMap<u64, Instant>,
}

impl SidebarAnimation {
    /// Reconcile aggregate workspace states without resetting a card whose
    /// label, pane count, focus, or geometry changed while it kept working.
    pub(super) fn reconcile<I>(&mut self, states: I, now: Instant) -> bool
    where
        I: IntoIterator<Item = (u64, Option<AgentState>)>,
        I::IntoIter: Clone,
    {
        let states = states.into_iter();
        let before = self.working_since.len();

        self.working_since.retain(|id, _| {
            states
                .clone()
                .any(|(candidate, state)| candidate == *id && state == Some(AgentState::Working))
        });
        let removed = self.working_since.len() != before;
        let mut inserted = false;
        for id in
            states.filter_map(|(id, state)| (state == Some(AgentState::Working)).then_some(id))
        {
            if let std::collections::btree_map::Entry::Vacant(entry) = self.working_since.entry(id)
            {
                entry.insert(now);
                inserted = true;
            }
        }
        removed || inserted
    }

    pub(super) fn frame(&self, id: u64, now: Instant) -> Option<GlintFrame> {
        self.working_since
            .get(&id)
            .map(|started| GlintFrame::for_elapsed(now.duration_since(*started)))
    }

    pub(super) fn frame_due(&self, map: &SidebarMap, now: Instant) -> bool {
        map.visible_glints().iter().any(|(id, painted)| {
            self.frame(*id, now)
                .is_some_and(|current| current != *painted)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn later_workspaces_keep_an_independent_phase() {
        let start = Instant::now();
        let mut animation = SidebarAnimation::default();

        assert!(animation.reconcile([(10, Some(AgentState::Working))], start));
        let later = start + Duration::from_millis(800);
        assert!(animation.reconcile(
            [
                (10, Some(AgentState::Working)),
                (20, Some(AgentState::Working)),
            ],
            later,
        ));

        let now = start + Duration::from_millis(1_600);
        assert_ne!(animation.frame(10, now), animation.frame(20, now));
    }

    #[test]
    fn working_metadata_keeps_phase_but_a_new_transition_restarts_it() {
        let start = Instant::now();
        let mut animation = SidebarAnimation::default();

        animation.reconcile([(7, Some(AgentState::Working))], start);
        let later = start + Duration::from_millis(800);
        assert!(!animation.reconcile([(7, Some(AgentState::Working))], later));
        assert_eq!(
            animation.frame(7, later),
            Some(GlintFrame::for_elapsed(Duration::from_millis(800)))
        );

        assert!(animation.reconcile([(7, Some(AgentState::Ready))], later));
        assert_eq!(animation.frame(7, later), None);
        assert!(animation.reconcile([(7, Some(AgentState::Working))], later));
        assert_eq!(
            animation.frame(7, later),
            Some(GlintFrame::for_elapsed(Duration::ZERO))
        );
    }

    #[test]
    fn removing_a_workspace_prunes_its_lifecycle() {
        let start = Instant::now();
        let mut animation = SidebarAnimation::default();
        animation.reconcile(
            [
                (1, Some(AgentState::Working)),
                (2, Some(AgentState::Working)),
            ],
            start,
        );

        assert!(animation.reconcile([(2, Some(AgentState::Working))], start));
        assert_eq!(animation.frame(1, start), None);
        assert!(animation.frame(2, start).is_some());
    }

    #[test]
    fn rest_frames_do_not_schedule_identical_repaints() {
        let start = Instant::now();
        let mut animation = SidebarAnimation::default();
        animation.reconcile([(1, Some(AgentState::Working))], start);
        let rest = GlintFrame::for_elapsed(Duration::from_millis(4_000));
        let map = SidebarMap::with_visible_glints(vec![(1, rest)]);

        assert!(!animation.frame_due(&map, start + Duration::from_millis(5_920)));
        assert!(!animation.frame_due(&map, start + Duration::from_millis(6_000)));
        assert!(animation.frame_due(&map, start + Duration::from_millis(6_080)));
    }
}
