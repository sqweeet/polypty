use super::App;

#[derive(Default)]
pub(super) struct ExitDialog {
    visible: bool,
    exit_selected: bool,
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
        changed
    }

    fn close(&mut self) {
        self.visible = false;
        self.exit_selected = false;
    }

    pub(super) fn select_exit(&mut self, selected: bool) -> bool {
        let changed = self.exit_selected != selected;
        self.exit_selected = selected;
        changed
    }

    pub(super) fn toggle(&mut self) -> bool {
        self.exit_selected = !self.exit_selected;
        true
    }
}

impl App {
    pub(in crate::app) fn request_exit(&mut self) {
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
