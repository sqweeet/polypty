use std::io::Write;

use anyhow::{Context, Result};
use crossterm::cursor::{Hide, MoveTo};
use crossterm::queue;
use crossterm::style::{Attribute, ResetColor, SetAttribute};
use crossterm::terminal::{Clear, ClearType};

pub fn clear(out: &mut impl Write) -> Result<()> {
    queue!(
        out,
        ResetColor,
        SetAttribute(Attribute::Reset),
        Clear(ClearType::All),
        Hide,
        MoveTo(0, 0)
    )
    .context("clear")?;
    Ok(())
}

/// Preserve colors emitted by child terminals even when polypty itself inherits
/// `NO_COLOR`.
pub fn enable_color_passthrough() {
    crossterm::style::Colored::set_ansi_color_disabled(false);
}

/// Bracket a paint batch so supporting terminals apply it atomically.
pub fn begin_sync(out: &mut impl Write) -> Result<()> {
    out.write_all(b"\x1b[?2026h\x1b[?7l")
        .context("sync begin")?;
    Ok(())
}

pub fn end_sync(out: &mut impl Write) -> Result<()> {
    out.write_all(b"\x1b[?7h\x1b[?2026l").context("sync end")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    #[test]
    fn frame_guard_prevents_bottom_row_scroll() {
        let mut out = Vec::new();
        begin_sync(&mut out).unwrap();
        end_sync(&mut out).unwrap();

        assert!(contains(&out, b"\x1b[?7l"));
        assert!(contains(&out, b"\x1b[?7h"));
        assert!(contains(&out, b"\x1b[?2026h"));
        assert!(contains(&out, b"\x1b[?2026l"));
    }
}
