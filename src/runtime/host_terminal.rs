use std::io::{stdout, Write};

use anyhow::{Context, Result};
use crossterm::{
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

pub(super) struct HostTerminal {
    active: bool,
}

impl HostTerminal {
    pub(super) fn enter() -> Result<Self> {
        enable_raw_mode().context("enable raw mode")?;
        let guard = Self { active: true };
        let mut output = stdout();
        execute!(
            output,
            EnterAlternateScreen,
            EnableBracketedPaste,
            EnableMouseCapture,
            crossterm::cursor::Hide
        )
        .context("enter alternate screen")?;
        output.flush().context("flush terminal setup")?;
        Ok(guard)
    }

    fn restore(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;
        let mut output = stdout();
        let _ = write_restore(&mut output);
        let _ = output.flush();
        let _ = disable_raw_mode();
    }
}

impl Drop for HostTerminal {
    fn drop(&mut self) {
        self.restore();
    }
}

fn write_restore(output: &mut impl Write) -> std::io::Result<()> {
    let output_modes = output.write_all(b"\x1b[?7h\x1b[?2026l");
    let screen_modes = execute!(
        output,
        DisableMouseCapture,
        DisableBracketedPaste,
        crossterm::cursor::Show,
        LeaveAlternateScreen
    );
    output_modes.and(screen_modes)
}

#[cfg(test)]
mod tests;
