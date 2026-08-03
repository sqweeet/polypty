mod event_loop;
mod host_terminal;
mod instance;
mod signals;

use anyhow::Result;

use host_terminal::HostTerminal;
use instance::InstanceGuard;
use signals::ShutdownLatch;

/// Owns process-level services and connects them to the application loop.
pub struct MuxRuntime {
    shutdown: ShutdownLatch,
    _instance: InstanceGuard,
}

impl MuxRuntime {
    pub fn new() -> Result<Self> {
        let instance = InstanceGuard::acquire()?;
        Ok(Self {
            shutdown: ShutdownLatch::install()?,
            _instance: instance,
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
