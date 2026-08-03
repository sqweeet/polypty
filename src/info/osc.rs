//! Stateful OSC metadata tracking for PTY byte streams.

/// Parsed OSC 7 cwd from a stream of PTY bytes (stateful).
#[derive(Debug, Default)]
pub struct OscTracker {
    buf: Vec<u8>,
    in_osc: bool,
    title_revision: u64,
    pub cwd: Option<String>,
    /// Last OSC 0/2 window title. `Some("")` represents an explicit clear,
    /// while `None` means the child has not emitted a title yet.
    pub title: Option<String>,
}

impl OscTracker {
    pub fn title_revision(&self) -> u64 {
        self.title_revision
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if !self.in_osc {
                // ESC ]
                if b == 0x1b {
                    self.buf.clear();
                    self.buf.push(b);
                } else if self.buf == [0x1b] && b == b']' {
                    self.buf.push(b);
                    self.in_osc = true;
                } else {
                    self.buf.clear();
                }
                continue;
            }

            self.buf.push(b);
            // BEL or ST (ESC \) terminate OSC
            let done = b == 0x07
                || (b == b'\\' && self.buf.len() >= 2 && self.buf[self.buf.len() - 2] == 0x1b);
            if !done {
                // Cap runaway sequences
                if self.buf.len() > 4096 {
                    self.buf.clear();
                    self.in_osc = false;
                }
                continue;
            }

            self.in_osc = false;
            let seq = std::mem::take(&mut self.buf);
            self.handle_osc(&seq);
        }
    }

    fn handle_osc(&mut self, seq: &[u8]) {
        // ESC ] <body> BEL/ST
        if seq.len() < 4 || seq[0] != 0x1b || seq[1] != b']' {
            return;
        }
        let end = if seq.ends_with(&[0x1b, b'\\']) {
            seq.len() - 2
        } else if seq.ends_with(&[0x07]) {
            seq.len() - 1
        } else {
            return;
        };
        let body = &seq[2..end];
        let Some(separator) = body.iter().position(|byte| *byte == b';') else {
            return;
        };
        let (command, value) = (&body[..separator], &body[separator + 1..]);

        match command {
            // OSC 0 sets icon + window title; OSC 2 sets window title.
            b"0" | b"2" => {
                self.title = Some(sanitize_osc_title(value));
                self.title_revision = self.title_revision.wrapping_add(1);
            }
            // 7;file://host/path
            b"7" => {
                if let Ok(s) = std::str::from_utf8(value) {
                    if let Some(path) = parse_osc7(s) {
                        self.cwd = Some(path);
                    }
                }
            }
            _ => {}
        }
    }
}

fn sanitize_osc_title(bytes: &[u8]) -> String {
    // Never let a child smuggle terminal controls into mux's own sidebar.
    // Lossy UTF-8 keeps metadata displayable without affecting PTY parsing.
    String::from_utf8_lossy(bytes)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect()
}

fn parse_osc7(s: &str) -> Option<String> {
    // file://hostname/path or file:///path
    let s = s.trim();
    let rest = s.strip_prefix("file://")?;
    // skip host
    let path = &rest[rest.find('/')?..];
    // percent-decode lightly
    let decoded = percent_decode(path)?;
    if decoded.is_empty() {
        None
    } else {
        Some(decoded)
    }
}

fn percent_decode(s: &str) -> Option<String> {
    // Decode into bytes first. Percent escapes describe URL bytes, not
    // Unicode scalar values; converting every decoded byte directly to char
    // corrupts both percent-encoded UTF-8 and ordinary non-ASCII paths.
    let mut out = Vec::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (from_hex(b[i + 1]), from_hex(b[i + 2])) {
                out.push(h << 4 | l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).ok()
}

fn from_hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc7_file_url() {
        let mut t = OscTracker::default();
        t.feed(b"\x1b]7;file://host/home/gotlib/proj\x07");
        assert_eq!(t.cwd.as_deref(), Some("/home/gotlib/proj"));
    }

    #[test]
    fn osc7_decodes_percent_encoded_and_raw_utf8() {
        let mut t = OscTracker::default();
        t.feed(b"\x1b]7;file://host/home/%E7%95%8C%20project\x07");
        assert_eq!(t.cwd.as_deref(), Some("/home/界 project"));

        for chunk in "\x1b]7;file:///tmp/שלום\x1b\\".as_bytes().chunks(3) {
            t.feed(chunk);
        }
        assert_eq!(t.cwd.as_deref(), Some("/tmp/שלום"));

        t.feed(b"\x1b]7;file:///%FF\x07");
        assert_eq!(
            t.cwd.as_deref(),
            Some("/tmp/שלום"),
            "invalid UTF-8 must not replace the last valid cwd"
        );
    }

    #[test]
    fn osc_titles_are_stateful_sanitized_and_can_be_cleared() {
        let mut t = OscTracker::default();
        for chunk in "\x1b]0;агент\x1b\\".as_bytes().chunks(2) {
            t.feed(chunk);
        }
        assert_eq!(t.title.as_deref(), Some("агент"));

        t.feed(b"\x1b");
        t.feed(b"]2;safe\x01 title\t\x7f\x1b");
        t.feed(b"\\");
        assert_eq!(t.title.as_deref(), Some("safe title"));

        t.feed(b"\x1b]7;file://host/work/tree\x07");
        assert_eq!(t.cwd.as_deref(), Some("/work/tree"));
        assert_eq!(t.title.as_deref(), Some("safe title"));

        t.feed(b"\x1b]2;\x07");
        assert_eq!(t.title.as_deref(), Some(""));
        assert_eq!(t.cwd.as_deref(), Some("/work/tree"));
    }
}
