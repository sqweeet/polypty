use std::time::Instant;

use super::{TabMotion, HOVER_DURATION, PRESS_DURATION};

impl TabMotion {
    pub(in crate::render::sidebar) fn set_hover(&mut self, key: Option<u64>, now: Instant) -> bool {
        if key == self.hovered {
            return false;
        }
        if let Some(previous) = self.hovered {
            self.levels
                .entry(previous)
                .or_default()
                .hover
                .transition(0, now, HOVER_DURATION, 0);
        }
        if let Some(key) = key {
            self.levels
                .entry(key)
                .or_default()
                .hover
                .transition(255, now, HOVER_DURATION, 32);
        }
        self.hovered = key;
        true
    }

    pub(in crate::render::sidebar) fn begin_press(&mut self, key: u64, now: Instant) -> bool {
        let changed = self.press_origin != Some(key);
        if let Some(previous) = self.press_origin.filter(|previous| *previous != key) {
            self.fade_press(previous, now, 0, 0);
        }
        self.press_origin = Some(key);
        self.fade_press(key, now, 255, 96);
        changed
    }

    pub(in crate::render::sidebar) fn update_press(
        &mut self,
        hit: Option<u64>,
        now: Instant,
    ) -> bool {
        let Some(origin) = self.press_origin else {
            return false;
        };
        let inside = hit == Some(origin);
        self.fade_press(
            origin,
            now,
            if inside { 255 } else { 0 },
            if inside { 96 } else { 0 },
        )
    }

    pub(in crate::render::sidebar) fn release(
        &mut self,
        hit: Option<u64>,
        now: Instant,
    ) -> Option<u64> {
        let origin = self.press_origin.take()?;
        self.fade_press(origin, now, 0, 0);
        (hit == Some(origin)).then_some(origin)
    }

    pub(in crate::render::sidebar) fn press_active(&self) -> bool {
        self.press_origin.is_some()
    }

    fn fade_press(&mut self, key: u64, now: Instant, target: u8, floor: u8) -> bool {
        self.levels
            .entry(key)
            .or_default()
            .press
            .transition(target, now, PRESS_DURATION, floor)
    }
}
