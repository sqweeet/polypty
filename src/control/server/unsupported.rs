use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use super::PendingRequest;

pub(crate) struct ControlServer;

impl ControlServer {
    pub(crate) fn bind(_: PathBuf) -> Result<Self> {
        bail!("mux control sessions require Unix sockets")
    }

    pub(crate) fn path(&self) -> &Path {
        Path::new("")
    }

    pub(crate) fn try_recv(&self) -> Option<PendingRequest> {
        None
    }
}
