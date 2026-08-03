use std::sync::mpsc::SyncSender;

use super::{ControlRequest, ControlResponse};

#[cfg(unix)]
#[path = "server/unix.rs"]
mod platform;
#[cfg(not(unix))]
#[path = "server/unsupported.rs"]
mod platform;

pub(crate) use platform::ControlServer;

pub(crate) struct PendingRequest {
    pub(crate) request: ControlRequest,
    pub(super) reply: SyncSender<ControlResponse>,
}

impl PendingRequest {
    pub(crate) fn respond_with(self, handler: impl FnOnce(ControlRequest) -> ControlResponse) {
        let response = handler(self.request);
        let _ = self.reply.send(response);
    }
}
