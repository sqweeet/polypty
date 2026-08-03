mod event_loop;
mod host_terminal;
mod signals;

use anyhow::Result;

use host_terminal::HostTerminal;
use signals::ShutdownLatch;

/// Owns process-level services and connects them to the application loop.
pub struct MuxRuntime {
    shutdown: ShutdownLatch,
}

impl MuxRuntime {
    pub fn new() -> Result<Self> {
        Ok(Self {
            shutdown: ShutdownLatch::install()?,
        })
    }

    pub fn run(self) -> Result<()> {
        crate::render::enable_color_passthrough();
        let _terminal = HostTerminal::enter()?;
        event_loop::run(&self.shutdown)
    }
}

pub fn run() -> Result<()> {
    MuxRuntime::new()?.run()
}
