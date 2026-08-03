use anyhow::Result;

#[cfg(unix)]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[cfg(unix)]
use anyhow::Context;
#[cfg(unix)]
use signal_hook::{
    consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM, SIGWINCH},
    iterator::Signals,
};

pub(super) struct ShutdownLatch {
    #[cfg(unix)]
    requested: Arc<AtomicBool>,
}

impl ShutdownLatch {
    pub(super) fn install() -> Result<Self> {
        #[cfg(unix)]
        {
            let requested = Arc::new(AtomicBool::new(false));
            for signal in [SIGTERM, SIGHUP, SIGINT, SIGQUIT] {
                signal_hook::flag::register(signal, Arc::clone(&requested))
                    .context("register shutdown signal")?;
            }
            Ok(Self { requested })
        }
        #[cfg(not(unix))]
        {
            Ok(Self {})
        }
    }

    pub(super) fn requested(&self) -> bool {
        #[cfg(unix)]
        {
            self.requested.load(Ordering::Relaxed)
        }
        #[cfg(not(unix))]
        {
            false
        }
    }
}

pub(super) struct ResizeWatcher {
    #[cfg(unix)]
    signals: Signals,
}

impl ResizeWatcher {
    pub(super) fn install() -> Result<Self> {
        #[cfg(unix)]
        {
            Ok(Self {
                signals: Signals::new([SIGWINCH]).context("register SIGWINCH")?,
            })
        }
        #[cfg(not(unix))]
        {
            Ok(Self {})
        }
    }

    pub(super) fn pending(&mut self) -> bool {
        #[cfg(unix)]
        {
            self.signals.pending().next().is_some()
        }
        #[cfg(not(unix))]
        {
            false
        }
    }
}
