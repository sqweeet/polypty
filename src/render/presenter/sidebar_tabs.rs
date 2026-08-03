use std::time::Instant;

use super::Presenter;

impl Presenter {
    pub(crate) fn set_hovered_sidebar_tab(&mut self, key: Option<u64>, now: Instant) -> bool {
        self.sidebar.set_hovered_tab(key, now)
    }

    pub(crate) fn begin_sidebar_tab_press(&mut self, key: u64, now: Instant) -> bool {
        self.sidebar.begin_tab_press(key, now)
    }

    pub(crate) fn update_sidebar_tab_press(&mut self, key: Option<u64>, now: Instant) -> bool {
        self.sidebar.update_tab_press(key, now)
    }

    pub(crate) fn release_sidebar_tab_press(
        &mut self,
        key: Option<u64>,
        now: Instant,
    ) -> Option<u64> {
        self.sidebar.release_tab_press(key, now)
    }

    pub(crate) fn sidebar_tab_press_active(&self) -> bool {
        self.sidebar.tab_press_active()
    }

    pub(crate) fn clear_sidebar_pointer(&mut self, now: Instant) -> bool {
        self.sidebar.clear_pointer(now)
    }
}
