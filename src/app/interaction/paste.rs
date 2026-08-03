use anyhow::Result;

use crate::app::App;

impl App {
    pub fn handle_paste(&mut self, text: &str) -> Result<()> {
        let bracketed = self
            .book
            .active()
            .map(|workspace| workspace.active_screen().bracketed_paste())
            .unwrap_or(false);
        if !bracketed {
            return self.write_active(text.as_bytes());
        }
        let mut bytes = Vec::with_capacity(text.len() + 16);
        bytes.extend_from_slice(b"\x1b[200~");
        bytes.extend_from_slice(text.as_bytes());
        bytes.extend_from_slice(b"\x1b[201~");
        self.write_active(&bytes)
    }
}
