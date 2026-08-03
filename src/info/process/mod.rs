//! Linux `/proc` session probing.

mod model;
mod procfs;
mod selection;

pub(super) fn probe_session(
    pid: u32,
    foreground_pgrp: Option<u32>,
) -> (Option<String>, Option<String>) {
    let cwd = procfs::read_cwd(pid);
    let process = foreground_pgrp
        .and_then(|group| selection::select_group_process(&procfs::read_group(group), group))
        .or_else(|| procfs::read_comm(pid));
    (cwd, process)
}

#[cfg(test)]
mod tests;
