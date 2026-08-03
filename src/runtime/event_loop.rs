use std::io::{stdout, Stdout};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyEventKind, KeyModifiers};

use super::{
    host_liveness::HostLiveness,
    signals::{ResizeWatcher, ShutdownLatch},
};
use crate::{app::App, config::Config, control::ControlServer};

pub(super) fn run(
    shutdown: &ShutdownLatch,
    config: &Config,
    control: &ControlServer,
) -> Result<()> {
    let (cols, rows) = crossterm::terminal::size().context("terminal size")?;
    let mut event_loop = EventLoop {
        app: App::configured(
            cols,
            rows,
            config.keymap.clone(),
            (
                config.sidebar.visible,
                config.sidebar.width,
                config.sidebar.shortcuts,
            ),
            config.shell.clone(),
            Some(control.path().to_owned()),
            config.source_path.clone(),
        )?,
        output: stdout(),
        resizes: ResizeWatcher::install()?,
    };
    if let Err(error) = event_loop.app.draw(&mut event_loop.output) {
        return event_loop.recover_host_close(error);
    }
    loop {
        match event_loop.tick(shutdown, control) {
            Ok(true) => {}
            Ok(false) => break,
            Err(error) => return event_loop.recover_host_close(error),
        }
    }
    Ok(())
}

struct EventLoop {
    app: App,
    output: Stdout,
    resizes: ResizeWatcher,
}

impl EventLoop {
    fn recover_host_close(&mut self, error: anyhow::Error) -> Result<()> {
        if HostLiveness::attached() {
            return Err(error);
        }
        self.app.shutdown();
        Ok(())
    }

    fn tick(&mut self, shutdown: &ShutdownLatch, control: &ControlServer) -> Result<bool> {
        if shutdown.requested() || !HostLiveness::attached() {
            self.app.shutdown();
            return Ok(false);
        }
        if self.resizes.pending() {
            self.preview_current_size()?;
        }

        self.handle_control(control);
        self.app.commit_resize_if_due()?;
        self.app.poll_ptys()?;
        self.draw_if_needed()?;

        if event::poll(Duration::from_millis(8))? {
            if !HostLiveness::attached() {
                self.app.shutdown();
                return Ok(false);
            }
            let event = event::read()?;
            if !self.handle_event(event)? {
                return Ok(false);
            }
        }
        Ok(!self.app.reap()?)
    }

    fn handle_control(&mut self, control: &ControlServer) {
        while let Some(pending) = control.try_recv() {
            pending.respond_with(|request| self.app.handle_control(request));
        }
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
