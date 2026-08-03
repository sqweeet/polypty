//! Live tab metadata assembled from process and OSC observations.

mod compose;
mod model;
mod osc;
mod path;
mod process;

pub use compose::compose_info;
pub use model::TabInfo;
pub use osc::OscTracker;
pub use path::shorten_path;

/// Read cwd and the terminal's foreground process group from Linux `/proc`.
pub fn probe_session(pid: u32, foreground_pgrp: Option<u32>) -> (Option<String>, Option<String>) {
    process::probe_session(pid, foreground_pgrp)
}

#[cfg(test)]
mod tests;
