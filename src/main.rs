mod agent;
mod app;
mod clip;
mod info;
mod input;
mod render;
mod tab;
mod workspace;

use std::io::{stdout, Write};
#[cfg(unix)]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyEventKind, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
#[cfg(unix)]
use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM, SIGWINCH};
#[cfg(unix)]
use signal_hook::iterator::Signals;

use crate::app::App;

#[cfg(unix)]
type ShutdownFlag = Arc<AtomicBool>;
#[cfg(not(unix))]
struct ShutdownFlag;

#[cfg(unix)]
fn install_shutdown_flag() -> Result<ShutdownFlag> {
    let requested = Arc::new(AtomicBool::new(false));
    for signal in [SIGTERM, SIGHUP, SIGINT, SIGQUIT] {
        signal_hook::flag::register(signal, Arc::clone(&requested))
            .context("register shutdown signal")?;
    }
    Ok(requested)
}

#[cfg(not(unix))]
fn install_shutdown_flag() -> Result<ShutdownFlag> {
    Ok(ShutdownFlag)
}

#[cfg(unix)]
fn shutdown_requested(flag: &ShutdownFlag) -> bool {
    flag.load(Ordering::Relaxed)
}

#[cfg(not(unix))]
fn shutdown_requested(_: &ShutdownFlag) -> bool {
    false
}

/// Restores every host-terminal mode mux can change. Kept separate from raw
/// mode so the byte-level reset can be regression-tested without a real TTY.
fn write_terminal_restore(out: &mut impl Write) -> std::io::Result<()> {
    // A draw error can leave a synchronized frame open with autowrap disabled.
    let output_modes = out.write_all(b"\x1b[?7h\x1b[?2026l");
    let screen_modes = execute!(
        out,
        DisableMouseCapture,
        DisableBracketedPaste,
        crossterm::cursor::Show,
        LeaveAlternateScreen
    );
    output_modes.and(screen_modes)
}

struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("enable raw mode")?;
        // Construct the guard before emitting any mode-changing bytes. If a
        // later setup command fails (or panics), Drop still restores the TTY.
        let guard = Self { active: true };
        let mut stdout = stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableBracketedPaste,
            EnableMouseCapture,
            crossterm::cursor::Hide
        )
        .context("enter alternate screen")?;
        stdout.flush().context("flush terminal setup")?;
        Ok(guard)
    }

    fn restore(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;

        let mut stdout = stdout();
        let _ = write_terminal_restore(&mut stdout);
        let _ = stdout.flush();
        // Keep this independent from output cleanup: a broken stdout must not
        // leave the user's terminal driver in raw mode.
        let _ = disable_raw_mode();
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        self.restore();
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("mux: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Install terminating-signal handlers before entering raw/alternate-screen
    // mode. The latch lets the ordinary unwind path restore every host mode
    // instead of allowing a terminating Unix signal to bypass
    // TerminalGuard::drop.
    let shutdown = install_shutdown_flag()?;
    // mux is a terminal emulator: child cell colors are data, not optional
    // decoration.  `NO_COLOR` must not make crossterm drop them while mux
    // serializes the emulated screen.
    render::enable_color_passthrough();
    let _terminal = TerminalGuard::enter()?;
    run_loop(&shutdown)
}

fn run_loop(shutdown: &ShutdownFlag) -> Result<()> {
    let (cols, rows) = crossterm::terminal::size().context("terminal size")?;
    let mut app = App::new(cols, rows)?;

    #[cfg(unix)]
    let mut signals = Signals::new([SIGWINCH]).context("register SIGWINCH")?;
    let mut stdout = stdout();

    // Initial draw.
    app.draw(&mut stdout)?;

    loop {
        if shutdown_requested(shutdown) {
            app.shutdown();
            break;
        }

        // POSIX SIGWINCH supplements crossterm's portable Resize events.
        // Windows uses the Event::Resize branch below.
        #[cfg(unix)]
        {
            let mut resized = false;
            for _ in signals.pending() {
                resized = true;
            }
            if resized {
                let (cols, rows) = crossterm::terminal::size()?;
                app.preview_resize(cols, rows);
            }
        }

        // A drag can produce dozens of intermediate sizes. Paint the host at
        // its latest geometry, but notify child TUIs only once the burst ends.
        app.commit_resize_if_due()?;

        // Pull PTY output from all tabs. Also repaint once a TUI output burst
        // has gone quiet so its final cursor position can be restored.
        app.poll_ptys()?;
        if app.needs_draw() {
            app.draw(&mut stdout)?;
        }

        // Input with short timeout so PTY output stays snappy.
        if event::poll(Duration::from_millis(8))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                        if app.handle_key(key)? {
                            break;
                        }
                        if app.needs_draw() {
                            app.draw(&mut stdout)?;
                        }
                    }
                }
                Event::Resize(_, _) => {
                    // Resize events can queue up while the window is being dragged.
                    // Always use the current size instead of briefly repainting stale
                    // intermediate heights, which leaves the sidebar short of the
                    // newly exposed bottom rows.
                    let (cols, rows) = crossterm::terminal::size()?;
                    app.preview_resize(cols, rows);
                    if app.needs_draw() {
                        app.draw(&mut stdout)?;
                    }
                }
                Event::Paste(text) => {
                    app.handle_paste(&text)?;
                }
                Event::Mouse(ev) => {
                    // Host terminals use Shift to bypass application mouse
                    // tracking for native selection. Ignore a shifted event if
                    // one is still reported, but never leave capture disabled.
                    if ev.modifiers.contains(KeyModifiers::SHIFT) {
                        continue;
                    }
                    if app.handle_mouse(ev)? {
                        app.draw(&mut stdout)?;
                    }
                }
                _ => {}
            }
        }

        // Reap dead tabs / exit if none left.
        if app.reap()? {
            break;
        }
    }

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
    fn terminal_restore_resets_every_output_mode() {
        let mut output = Vec::new();
        write_terminal_restore(&mut output).unwrap();

        assert!(contains(&output, b"\x1b[?7h"));
        assert!(contains(&output, b"\x1b[?2026l"));
        assert!(contains(&output, b"\x1b[?1000l"));
        assert!(contains(&output, b"\x1b[?1002l"));
        assert!(contains(&output, b"\x1b[?1003l"));
        assert!(contains(&output, b"\x1b[?1006l"));
        assert!(contains(&output, b"\x1b[?2004l"));
        assert!(contains(&output, b"\x1b[?25h"));
        assert!(contains(&output, b"\x1b[?1049l"));
    }
}
