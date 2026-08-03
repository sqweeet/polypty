pub(super) mod callbacks;

use crate::info::OscTracker;

use callbacks::TerminalCallbacks;

pub(super) struct TerminalEmulator {
    parser: vt100::Parser<TerminalCallbacks>,
    osc: OscTracker,
}

impl TerminalEmulator {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            parser: vt100::Parser::new_with_callbacks(rows, cols, 0, TerminalCallbacks::default()),
            osc: OscTracker::default(),
        }
    }

    pub fn process(&mut self, bytes: &[u8]) -> bool {
        let revision = self.osc.title_revision();
        self.osc.feed(bytes);
        self.parser.process(bytes);
        self.osc.title_revision() > revision
    }

    pub fn take_responses(&mut self) -> Vec<u8> {
        self.parser.callbacks_mut().take_responses()
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.parser.screen_mut().set_size(rows, cols);
    }

    pub fn size(&self) -> (u16, u16) {
        let (rows, cols) = self.parser.screen().size();
        (cols, rows)
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    pub fn title(&self) -> Option<&str> {
        self.osc.title.as_deref()
    }

    pub fn cwd(&self) -> Option<&str> {
        self.osc.cwd.as_deref()
    }
}
