mod event_loop;
mod host_liveness;
mod host_terminal;
mod instance;
mod signals;

use anyhow::Result;

use crate::{
    config::Config,
    control::{self, ControlServer},
};

use host_terminal::HostTerminal;
use instance::InstanceGuard;
use signals::ShutdownLatch;

/// Owns process-level services and connects them to the application loop.
pub struct PolyptyRuntime {
    shutdown: ShutdownLatch,
    config: Config,
    control: ControlServer,
    _instance: InstanceGuard,
}

impl PolyptyRuntime {
    pub fn new() -> Result<Self> {
        let instance = InstanceGuard::acquire()?;
        let config = Config::load()?;
        let control = ControlServer::bind(control::socket_path()?)?;
        Ok(Self {
            shutdown: ShutdownLatch::install()?,
            config,
            control,
            _instance: instance,
        })
    }

    pub fn run(self) -> Result<()> {
        crate::render::enable_color_passthrough();
        let _terminal = HostTerminal::enter()?;
        event_loop::run(&self.shutdown, &self.config, &self.control)
    }
}

pub fn run() -> Result<()> {
    if control::dispatch_cli()? {
        return Ok(());
    }
    PolyptyRuntime::new()?.run()
}
