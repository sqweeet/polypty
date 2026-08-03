use std::io::{stdout, Stdout};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyEventKind, KeyModifiers};

use super::signals::{ResizeWatcher, ShutdownLatch};
use crate::app::App;

pub(super) fn run(shutdown: &ShutdownLatch) -> Result<()> {
    let (cols, rows) = crossterm::terminal::size().context("terminal size")?;
    let mut event_loop = EventLoop {
        app: App::new(cols, rows)?,
        output: stdout(),
        resizes: ResizeWatcher::install()?,
    };
    event_loop.app.draw(&mut event_loop.output)?;
    while event_loop.tick(shutdown)? {}
    Ok(())
}

struct EventLoop {
    app: App,
    output: Stdout,
    resizes: ResizeWatcher,
}

impl EventLoop {
    fn tick(&mut self, shutdown: &ShutdownLatch) -> Result<bool> {
        if shutdown.requested() {
            self.app.shutdown();
            return Ok(false);
        }
        if self.resizes.pending() {
            self.preview_current_size()?;
        }

        self.app.commit_resize_if_due()?;
        self.app.poll_ptys()?;
        self.draw_if_needed()?;

        if event::poll(Duration::from_millis(8))? {
            let event = event::read()?;
            if !self.handle_event(event)? {
                return Ok(false);
            }
        }
        Ok(!self.app.reap()?)
    }

    fn handle_event(&mut self, event: Event) -> Result<bool> {
        match event {
            Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                if self.app.handle_key(key)? {
                    return Ok(false);
                }
                self.draw_if_needed()?;
            }
            Event::Resize(_, _) => {
                self.preview_current_size()?;
                self.draw_if_needed()?;
            }
            Event::Paste(text) => self.app.handle_paste(&text)?,
            Event::Mouse(mouse)
                if !mouse.modifiers.contains(KeyModifiers::SHIFT)
                    && self.app.handle_mouse(mouse)? =>
            {
                self.app.draw(&mut self.output)?;
            }
            _ => {}
        }
        Ok(true)
    }

    fn preview_current_size(&mut self) -> Result<()> {
        let (cols, rows) = crossterm::terminal::size()?;
        self.app.preview_resize(cols, rows);
        Ok(())
    }

    fn draw_if_needed(&mut self) -> Result<()> {
        if self.app.needs_draw() {
            self.app.draw(&mut self.output)?;
        }
        Ok(())
    }
}
