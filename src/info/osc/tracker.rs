use super::parser::{parse, OscUpdate};

/// Parsed OSC title and cwd from a stream of PTY bytes.
#[derive(Debug, Default)]
pub struct OscTracker {
    buffer: Vec<u8>,
    in_osc: bool,
    title_revision: u64,
    pub cwd: Option<String>,
    /// `Some("")` means explicit clear; `None` means no title was emitted.
    pub title: Option<String>,
}

impl OscTracker {
    pub fn title_revision(&self) -> u64 {
        self.title_revision
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if !self.in_osc {
                self.seek_start(byte);
                continue;
            }
            self.buffer.push(byte);
            if sequence_done(&self.buffer, byte) {
                self.in_osc = false;
                let sequence = std::mem::take(&mut self.buffer);
                self.apply(parse(&sequence));
            } else if self.buffer.len() > 4096 {
                self.buffer.clear();
                self.in_osc = false;
            }
        }
    }

    fn seek_start(&mut self, byte: u8) {
        if byte == 0x1b {
            self.buffer.clear();
            self.buffer.push(byte);
        } else if self.buffer == [0x1b] && byte == b']' {
            self.buffer.push(byte);
            self.in_osc = true;
        } else {
            self.buffer.clear();
        }
    }

    fn apply(&mut self, update: Option<OscUpdate>) {
        match update {
            Some(OscUpdate::Title(title)) => {
                self.title = Some(title);
                self.title_revision = self.title_revision.wrapping_add(1);
            }
            Some(OscUpdate::Cwd(cwd)) => self.cwd = Some(cwd),
            None => {}
        }
    }
}

fn sequence_done(buffer: &[u8], byte: u8) -> bool {
    byte == 0x07 || (byte == b'\\' && buffer.len() >= 2 && buffer[buffer.len() - 2] == 0x1b)
}
