mod level;
mod pointer;

use std::{collections::BTreeMap, time::Duration, time::Instant};

use super::model::SidebarTab;
use level::Level;

const ACTIVE_DURATION: Duration = Duration::from_millis(150);
const HOVER_DURATION: Duration = Duration::from_millis(110);
const PRESS_DURATION: Duration = Duration::from_millis(70);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct TabVisual {
    pub(super) active: u8,
    pub(super) hover: u8,
    pub(super) press: u8,
}

impl TabVisual {
    pub(super) fn settled(active: bool) -> Self {
        Self {
            active: if active { 255 } else { 0 },
            ..Self::default()
        }
    }
}

#[derive(Debug, Default)]
struct TabLevels {
    active: Level,
    hover: Level,
    press: Level,
}

#[derive(Debug, Default)]
pub(super) struct TabMotion {
    levels: BTreeMap<u64, TabLevels>,
    active: Option<u64>,
    hovered: Option<u64>,
    press_origin: Option<u64>,
}

impl TabMotion {
    pub(super) fn reconcile(&mut self, tabs: &[SidebarTab], now: Instant) {
        self.levels
            .retain(|key, _| tabs.iter().any(|tab| tab.key == *key));
        let next = tabs.iter().find(|tab| tab.active).map(|tab| tab.key);
        if next == self.active {
            return;
        }
        if let Some(previous) = self.active {
            self.levels
                .entry(previous)
                .or_default()
                .active
                .transition(0, now, ACTIVE_DURATION, 0);
        }
        if let Some(key) = next {
            let active = &mut self.levels.entry(key).or_default().active;
            if self.active.is_none() {
                active.jump(255);
            } else {
                active.transition(255, now, ACTIVE_DURATION, 0);
            }
        }
        self.active = next;
    }

    pub(super) fn visual(&self, key: u64, active: bool, now: Instant) -> TabVisual {
        self.levels.get(&key).map_or_else(
            || TabVisual::settled(active),
            |levels| TabVisual {
                active: levels.active.value(now),
                hover: levels.hover.value(now),
                press: levels.press.value(now),
            },
        )
    }

    pub(super) fn frame_due(&self, visible: &[u64], now: Instant) -> bool {
        visible.iter().any(|key| {
            self.levels.get(key).is_some_and(|levels| {
                levels.active.frame_due(now)
                    || levels.hover.frame_due(now)
                    || levels.press.frame_due(now)
            })
        })
    }

    pub(super) fn mark_frame(&mut self, visible: &[u64], now: Instant) {
        for key in visible {
            if let Some(levels) = self.levels.get_mut(key) {
                levels.active.mark_frame(now);
                levels.hover.mark_frame(now);
                levels.press.mark_frame(now);
            }
        }
    }
}
