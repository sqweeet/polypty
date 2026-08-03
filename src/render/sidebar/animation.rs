use std::{collections::BTreeMap, time::Instant};

use crate::agent::AgentState;

use super::{GlintFrame, SidebarMap};

/// Owns the independent Working epoch of every workspace card.
#[derive(Debug, Default)]
pub(super) struct SidebarAnimation {
    working_since: BTreeMap<u64, Instant>,
}

impl SidebarAnimation {
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
