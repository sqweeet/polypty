use std::time::Instant;

use crate::{
    app::ui_animation::{fade_opacity, UiPress, UiSelection},
    render::{ShortcutDialogView, ShortcutScope},
};

pub(super) struct ShortcutDialog {
    desired_visible: Option<bool>,
    selected: ShortcutScope,
    save_failed: bool,
    opened_at: Option<Instant>,
    press: UiPress<ShortcutScope>,
    selection: UiSelection<ShortcutScope>,
    fade_complete: bool,
}

impl Default for ShortcutDialog {
    fn default() -> Self {
        Self {
            desired_visible: None,
            selected: ShortcutScope::Session,
            save_failed: false,
            opened_at: None,
            press: UiPress::default(),
            selection: UiSelection::default(),
            fade_complete: false,
        }
    }
}

impl ShortcutDialog {
    pub(super) fn visible(&self) -> bool {
        self.desired_visible.is_some()
    }

    pub(super) fn open(&mut self, desired_visible: bool) {
        self.desired_visible = Some(desired_visible);
        self.selected = ShortcutScope::Session;
        self.save_failed = false;
        self.opened_at = Some(Instant::now());
        self.press = UiPress::default();
        self.selection.reset(ShortcutScope::Session);
        self.fade_complete = false;
    }

    pub(super) fn close(&mut self) -> bool {
        let changed = self.desired_visible.take().is_some();
        self.selected = ShortcutScope::Session;
        self.save_failed = false;
        self.opened_at = None;
        self.press = UiPress::default();
        self.selection = UiSelection::default();
        self.fade_complete = false;
        changed
    }

    #[cfg(test)]
    pub(super) fn view(&self) -> Option<ShortcutDialogView> {
        self.view_at(Instant::now())
    }

    pub(super) fn view_at(&self, now: Instant) -> Option<ShortcutDialogView> {
        let (pressed, press_opacity) = self
            .press
            .visual(now)
            .map_or((None, 0), |(target, opacity)| (Some(target), opacity));
        Some(ShortcutDialogView {
            desired_visible: self.desired_visible?,
            selected: self.selected,
            save_failed: self.save_failed,
            opacity: fade_opacity(self.opened_at?, now),
            pressed,
            press_opacity,
            session_opacity: self.selection.opacity(ShortcutScope::Session, now),
            always_opacity: self.selection.opacity(ShortcutScope::Always, now),
        })
    }

    pub(super) fn animation_due(&self) -> bool {
        (self.visible() && !self.fade_complete)
            || self.press.animation_due()
            || self.selection.animation_due()
    }

    pub(super) fn mark_animation_frame(
        &mut self,
        opacity: Option<u8>,
        press_opacity: Option<u8>,
        selection: Option<(u8, u8)>,
    ) {
        self.fade_complete |= opacity == Some(255);
        self.press.mark_frame(press_opacity);
        if let Some((session, always)) = selection {
            self.selection.mark_frame(ShortcutScope::Session, session);
            self.selection.mark_frame(ShortcutScope::Always, always);
        }
    }

    pub(super) fn desired_visible(&self) -> Option<bool> {
        self.desired_visible
    }

    pub(super) fn selected(&self) -> ShortcutScope {
        self.selected
    }

    pub(super) fn select(&mut self, selected: ShortcutScope) -> bool {
        let changed = self.selected != selected;
        self.selected = selected;
        if changed {
            self.selection.select(selected);
        }
        changed
    }

    pub(super) fn toggle(&mut self) -> bool {
        let selected = match self.selected {
            ShortcutScope::Session => ShortcutScope::Always,
            ShortcutScope::Always => ShortcutScope::Session,
        };
        self.select(selected)
    }

    pub(super) fn press(&mut self, scope: ShortcutScope) -> bool {
        self.press.begin(scope)
    }

    pub(super) fn update_press(&mut self, hit: Option<ShortcutScope>) -> bool {
        self.press.update(hit)
    }

    pub(super) fn release(&mut self, hit: Option<ShortcutScope>) -> Option<ShortcutScope> {
        self.press.release(hit)
    }

    pub(super) fn is_pressed(&self) -> bool {
        self.press.active()
    }

    pub(super) fn mark_save_failed(&mut self) {
        self.save_failed = true;
    }
}
