use std::{io::Write, time::Instant};

use anyhow::Result;

use crate::{agent::AgentState, render::Layout};

use super::{
    animation::SidebarAnimation, draw_sidebar, GlintFrame, SidebarCache, SidebarMap, SidebarTab,
};

#[derive(Default)]
pub(in crate::render) struct SidebarPresentation {
    fingerprint: String,
    cache: SidebarCache,
    animation: SidebarAnimation,
    map: SidebarMap,
}

impl SidebarPresentation {
    pub(in crate::render) fn invalidate(&mut self) {
        self.cache.invalidate();
        self.fingerprint.clear();
    }

    pub(in crate::render) fn invalidate_content(&mut self) {
        self.fingerprint.clear();
    }

    pub(in crate::render) fn reconcile<I>(&mut self, states: I, now: Instant) -> bool
    where
        I: IntoIterator<Item = (u64, Option<AgentState>)>,
        I::IntoIter: Clone,
    {
        self.animation.reconcile(states, now)
    }

    pub(in crate::render) fn frame(&self, id: u64, now: Instant) -> Option<GlintFrame> {
        self.animation.frame(id, now)
    }

    pub(in crate::render) fn frame_due(&self, now: Instant) -> bool {
        self.animation.frame_due(&self.map, now)
    }

    pub(in crate::render) fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub(in crate::render) fn tab_at(&self, column: u16, row: u16) -> Option<usize> {
        self.map.tab_at(column, row)
    }

    pub(in crate::render) fn draw(
        &mut self,
        output: &mut impl Write,
        layout: &Layout,
        tabs: &[SidebarTab],
        fingerprint: &str,
        hard_clear: bool,
    ) -> Result<()> {
        self.map = draw_sidebar(output, layout, tabs, &mut self.cache, hard_clear)?;
        self.fingerprint.clear();
        self.fingerprint.push_str(fingerprint);
        Ok(())
    }

    pub(in crate::render) fn clear(&mut self) {
        self.map = SidebarMap::default();
        self.invalidate();
    }
}
