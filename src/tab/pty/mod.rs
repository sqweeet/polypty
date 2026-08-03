pub(super) mod input;
mod lifecycle;
pub(super) mod output;
mod spawn;

use portable_pty::{Child, MasterPty};

use input::PtyInputChannel;
use output::PtyOutputChannel;

pub(super) use input::InputQueueResult;

pub(super) struct PtyTransport {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    input: PtyInputChannel,
    output: PtyOutputChannel,
    child_exited: bool,
    shell_pid: Option<u32>,
}
