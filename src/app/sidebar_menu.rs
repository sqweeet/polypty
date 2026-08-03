use std::time::Instant;

use crate::{
    app::ui_animation::{fade_opacity, UiPress},
    render::{SidebarMenuAction, SidebarMenuView},
};

pub(super) struct SidebarMenu {
    anchor: Option<(u16, u16)>,
    selected: SidebarMenuAction,
    target_tab: Option<u64>,
    opened_at: Option<Instant>,
    press: UiPress<SidebarMenuAction>,
    fade_complete: bool,
}

impl Default for SidebarMenu {
    fn default() -> Self {
        Self {
            anchor: None,
            selected: SidebarMenuAction::NewTab,
            target_tab: None,
            opened_at: None,
            press: UiPress::default(),
            fade_complete: false,
        }
    }
}

impl SidebarMenu {
    pub(super) fn visible(&self) -> bool {
        self.anchor.is_some()
    }

    pub(super) fn open(&mut self, column: u16, row: u16, target_tab: Option<u64>) {
        self.anchor = Some((column, row));
        self.selected = SidebarMenuAction::NewTab;
        self.target_tab = target_tab;
        self.opened_at = Some(Instant::now());
        self.press = UiPress::default();
        self.fade_complete = false;
    }

    pub(super) fn close(&mut self) -> bool {
        let changed = self.anchor.take().is_some();
        self.selected = SidebarMenuAction::NewTab;
        self.target_tab = None;
        self.opened_at = None;
        self.press = UiPress::default();
        self.fade_complete = false;
        changed
    }

    pub(super) fn view(&self, shortcuts_visible: bool) -> Option<SidebarMenuView> {
        self.view_at(shortcuts_visible, Instant::now())
    }

    pub(super) fn view_at(&self, shortcuts_visible: bool, now: Instant) -> Option<SidebarMenuView> {
        let (anchor_column, anchor_row) = self.anchor?;
        let (pressed, press_opacity) = self
            .press
            .visual(now)
            .map_or((None, 0), |(target, opacity)| (Some(target), opacity));
        Some(SidebarMenuView {
            anchor_column,
            anchor_row,
            selected: self.selected,
            shortcuts_visible,
            can_close: self.target_tab.is_some(),
            opacity: fade_opacity(self.opened_at?, now),
            pressed,
            press_opacity,
        })
    }

    pub(super) fn animation_due(&self) -> bool {
        (self.visible() && !self.fade_complete) || self.press.animation_due()
    }

    pub(super) fn mark_animation_frame(&mut self, opacity: Option<u8>, press_opacity: Option<u8>) {
        self.fade_complete |= opacity == Some(255);
        self.press.mark_frame(press_opacity);
    }

    pub(super) fn selected_action(&self) -> SidebarMenuAction {
        self.selected
    }

    pub(super) fn target_tab(&self) -> Option<u64> {
        self.target_tab
    }

    pub(super) fn select(&mut self, action: SidebarMenuAction) -> bool {
        let changed = self.selected != action;
        self.selected = action;
        changed
    }

    pub(super) fn press(&mut self, action: SidebarMenuAction) -> bool {
        self.press.begin(action)
    }

    pub(super) fn update_press(&mut self, hit: Option<SidebarMenuAction>) -> bool {
        self.press.update(hit)
    }

    pub(super) fn release(&mut self, hit: Option<SidebarMenuAction>) -> Option<SidebarMenuAction> {
        self.press.release(hit)
    }

    pub(super) fn is_pressed(&self) -> bool {
        self.press.active()
    }

    pub(super) fn select_next(&mut self) -> bool {
        self.select_offset(1)
    }

    pub(super) fn select_previous(&mut self) -> bool {
        let actions = SidebarMenuAction::items(self.target_tab.is_some());
        self.select_offset(actions.len().saturating_sub(1))
    }

    fn select_offset(&mut self, offset: usize) -> bool {
        let actions = SidebarMenuAction::items(self.target_tab.is_some());
        let index = actions
            .iter()
            .position(|action| *action == self.selected)
            .unwrap_or(0);
        let selected = actions[(index + offset) % actions.len()];
        let changed = selected != self.selected;
        self.selected = selected;
        changed
    }
}
