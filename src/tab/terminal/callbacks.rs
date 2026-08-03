const FOREGROUND_COLOR_REPLY: &[u8] = b"\x1b]10;rgb:bcbc/bcbc/bcbc\x1b\\";
const BACKGROUND_COLOR_REPLY: &[u8] = b"\x1b]11;rgb:1515/1515/1515\x1b\\";
const CURSOR_COLOR_REPLY: &[u8] = b"\x1b]12;rgb:bcbc/bcbc/bcbc\x1b\\";

#[derive(Debug, Default)]
pub(in crate::tab) struct TerminalCallbacks {
    responses: Vec<u8>,
}

impl TerminalCallbacks {
    pub fn take_responses(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.responses)
    }

    fn single_param(params: &[&[u16]]) -> Option<u16> {
        match params {
            [] | [[]] => Some(0),
            [param] if param.len() == 1 => param.first().copied(),
            _ => None,
        }
    }
}

impl vt100::Callbacks for TerminalCallbacks {
    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        i1: Option<u8>,
        i2: Option<u8>,
        params: &[&[u16]],
        c: char,
    ) {
        let response: Option<Vec<u8>> = match (i1, i2, c, Self::single_param(params)) {
            (None, None, 'n', Some(5)) => Some(b"\x1b[0n".to_vec()),
            (None, None, 'n', Some(6)) => {
                let (row, col) = screen.cursor_position();
                Some(format!("\x1b[{};{}R", row + 1, col + 1).into_bytes())
            }
            (None, None, 'c', Some(0)) => Some(b"\x1b[?1;2c".to_vec()),
            (Some(b'>'), None, 'c', Some(0)) => Some(b"\x1b[>0;0;0c".to_vec()),
            _ => None,
        };
        if let Some(response) = response {
            self.responses.extend_from_slice(&response);
        }
    }

    fn unhandled_osc(&mut self, _: &mut vt100::Screen, params: &[&[u8]]) {
        let response = match params {
            [b"10", b"?"] => Some(FOREGROUND_COLOR_REPLY),
            [b"11", b"?"] => Some(BACKGROUND_COLOR_REPLY),
            [b"12", b"?"] => Some(CURSOR_COLOR_REPLY),
            _ => None,
        };
        if let Some(response) = response {
            self.responses.extend_from_slice(response);
        }
    }
}

#[cfg(test)]
pub(in crate::tab) const TEST_COLOR_REPLIES: [&[u8]; 3] = [
    FOREGROUND_COLOR_REPLY,
    BACKGROUND_COLOR_REPLY,
    CURSOR_COLOR_REPLY,
];
