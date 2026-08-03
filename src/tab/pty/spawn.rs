use anyhow::{Context, Result};
use portable_pty::{native_pty_system, PtySize};

use crate::tab::environment::child_command;

use super::{input::PtyInputChannel, output::PtyOutputChannel, PtyTransport};

impl PtyTransport {
    pub fn spawn(id: u64, cols: u16, rows: u16) -> Result<Self> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("open pty")?;
        let child = pair
            .slave
            .spawn_command(child_command(id))
            .context("spawn shell")?;
        drop(pair.slave);

        let shell_pid = child.process_id();
        let reader = pair.master.try_clone_reader().context("clone pty reader")?;
        let writer = pair.master.take_writer().context("take pty writer")?;
        let input = PtyInputChannel::spawn(writer, id)?;
        let output = PtyOutputChannel::spawn(reader, id)?;

        Ok(Self {
            master: pair.master,
            child,
            input,
            output,
            child_exited: false,
            shell_pid,
        })
    }
}
