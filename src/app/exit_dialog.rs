use std::time::Instant;

use crate::{
    app::ui_animation::{fade_opacity, UiPress, UiSelection},
    render::ExitDialogButton,
};

use super::App;

#[derive(Default)]
pub(super) struct ExitDialog {
    visible: bool,
    exit_selected: bool,
    opened_at: Option<Instant>,
    press: UiPress<ExitDialogButton>,
    selection: UiSelection<ExitDialogButton>,
    fade_complete: bool,
}

impl ExitDialog {
    pub(super) fn visible(&self) -> bool {
        self.visible
    }

    pub(super) fn exit_selected(&self) -> bool {
        self.exit_selected
    }

    fn open(&mut self) -> bool {
        let changed = !self.visible;
        self.visible = true;
        self.exit_selected = false;
        self.opened_at = Some(Instant::now());
        self.press = UiPress::default();
        self.selection.reset(ExitDialogButton::Cancel);
        self.fade_complete = false;
        changed
    }

    fn close(&mut self) {
        self.visible = false;
        self.exit_selected = false;
        self.opened_at = None;
        self.press = UiPress::default();
        self.selection = UiSelection::default();
        self.fade_complete = false;
    }

    pub(super) fn select_exit(&mut self, selected: bool) -> bool {
        let changed = self.exit_selected != selected;
        self.exit_selected = selected;
        if changed {
            self.selection.select(if selected {
                ExitDialogButton::Exit
            } else {
                ExitDialogButton::Cancel
            });
        }
        changed
    }

    pub(super) fn toggle(&mut self) -> bool {
        self.select_exit(!self.exit_selected)
    }

    pub(super) fn opacity(&self, now: Instant) -> u8 {
        self.opened_at
            .map_or(255, |opened_at| fade_opacity(opened_at, now))
    }

    pub(super) fn animation_due(&self) -> bool {
        (self.visible && !self.fade_complete)
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
        if let Some((cancel, exit)) = selection {
            self.selection.mark_frame(ExitDialogButton::Cancel, cancel);
            self.selection.mark_frame(ExitDialogButton::Exit, exit);
        }
    }

    pub(super) fn press(&mut self, button: ExitDialogButton) -> bool {
        self.press.begin(button)
    }

    pub(super) fn update_press(&mut self, hit: Option<ExitDialogButton>) -> bool {
        self.press.update(hit)
    }

    pub(super) fn release(&mut self, hit: Option<ExitDialogButton>) -> Option<ExitDialogButton> {
        self.press.release(hit)
    }

    pub(super) fn press_visual(&self, now: Instant) -> (Option<ExitDialogButton>, u8) {
        self.press
            .visual(now)
            .map_or((None, 0), |(target, opacity)| (Some(target), opacity))
    }

    pub(super) fn selection_visual(&self, now: Instant) -> (u8, u8) {
        (
            self.selection.opacity(ExitDialogButton::Cancel, now),
            self.selection.opacity(ExitDialogButton::Exit, now),
        )
    }

    pub(super) fn is_pressed(&self) -> bool {
        self.press.active()
    }
}

impl App {
    pub(in crate::app) fn request_exit(&mut self) {
        self.clear_sidebar_pointer();
        self.close_sidebar_menu();
        self.cancel_shortcut_dialog();
        if self.exit_dialog.open() {
            self.frame.request_full_draw();
        }
    }

    pub(in crate::app) fn cancel_exit(&mut self) {
        self.exit_dialog.close();
        self.frame.request_hard_clear();
    }

    pub(in crate::app) fn confirm_exit(&mut self) {
        self.shutdown();
        self.frame.request_quit();
    }
}
