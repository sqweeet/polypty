mod interaction;

use std::{io::Write, time::Instant};

use anyhow::Result;

use crate::{agent::AgentState, render::Layout};

use super::tab_motion::TabMotion;
use super::{
    animation::SidebarAnimation, footer::SidebarShortcuts, painter::draw_sidebar_with_shortcuts,
    GlintFrame, SidebarCache, SidebarMap, SidebarTab,
};

pub(in crate::render) struct SidebarPresentation {
    fingerprint: String,
    cache: SidebarCache,
    animation: SidebarAnimation,
    map: SidebarMap,
    motion: TabMotion,
    shortcuts: SidebarShortcuts,
}

impl Default for SidebarPresentation {
    fn default() -> Self {
        Self::new(SidebarShortcuts::default())
    }
}

impl SidebarPresentation {
    pub(in crate::render) fn new(shortcuts: SidebarShortcuts) -> Self {
        Self {
            fingerprint: String::new(),
            cache: SidebarCache::default(),
            animation: SidebarAnimation::default(),
            map: SidebarMap::default(),
            motion: TabMotion::default(),
            shortcuts,
        }
    }

    pub(in crate::render) fn invalidate(&mut self) {
        self.cache.invalidate();
        self.fingerprint.clear();
    }

    pub(in crate::render) fn invalidate_content(&mut self) {
        self.fingerprint.clear();
    }

    pub(in crate::render) fn set_shortcuts_visible(&mut self, visible: bool) {
        if self.shortcuts.visible != visible {
            self.shortcuts.visible = visible;
            self.invalidate();
        }
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
            || self.motion.frame_due(self.map.visible_tabs(), now)
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
        let now = Instant::now();
        self.motion.reconcile(tabs, now);
        self.map = draw_sidebar_with_shortcuts(
            output,
            layout,
            tabs,
            &mut self.cache,
            hard_clear,
            &self.shortcuts,
            (&self.motion, now),
        )?;
        self.motion.mark_frame(self.map.visible_tabs(), now);
        self.fingerprint.clear();
        self.fingerprint.push_str(fingerprint);
        Ok(())
    }

    pub(in crate::render) fn clear(&mut self) {
        self.map = SidebarMap::default();
        self.clear_pointer(Instant::now());
        self.invalidate();
    }
}
