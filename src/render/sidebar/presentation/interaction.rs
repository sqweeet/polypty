use std::time::Instant;

use super::SidebarPresentation;

impl SidebarPresentation {
    pub(in crate::render) fn set_hovered_tab(&mut self, key: Option<u64>, now: Instant) -> bool {
        self.motion.set_hover(key, now)
    }

    pub(in crate::render) fn begin_tab_press(&mut self, key: u64, now: Instant) -> bool {
        self.motion.begin_press(key, now)
    }

    pub(in crate::render) fn update_tab_press(&mut self, key: Option<u64>, now: Instant) -> bool {
        self.motion.update_press(key, now)
    }

    pub(in crate::render) fn release_tab_press(
        &mut self,
        key: Option<u64>,
        now: Instant,
    ) -> Option<u64> {
        self.motion.release(key, now)
    }

    pub(in crate::render) fn tab_press_active(&self) -> bool {
        self.motion.press_active()
    }

    pub(in crate::render) fn clear_pointer(&mut self, now: Instant) -> bool {
        let hover = self.motion.set_hover(None, now);
        let pressed = self.motion.press_active();
        self.motion.release(None, now);
        hover || pressed
    }
}
