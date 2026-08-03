use anyhow::{Context, Result};
use portable_pty::PtySize;

use super::PtyTransport;

impl PtyTransport {
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("pty resize")
    }

    pub fn shell_pid(&self) -> Option<u32> {
        self.shell_pid
    }

    #[cfg(unix)]
    pub fn foreground_process_group(&self) -> Option<u32> {
        self.master
            .process_group_leader()
            .and_then(|value| u32::try_from(value).ok())
    }

    #[cfg(not(unix))]
    pub fn foreground_process_group(&self) -> Option<u32> {
        None
    }

    pub fn try_reap(&mut self) -> bool {
        if !self.child_exited {
            match self.child.try_wait() {
                Ok(Some(_)) | Err(_) => self.child_exited = true,
                Ok(None) => {}
            }
        }
        self.child_exited && self.output.is_fully_disconnected()
    }

    pub fn child_exited(&self) -> bool {
        self.child_exited
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }
}
