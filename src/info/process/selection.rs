use crate::agent;

use super::model::ProcEntry;
use crate::info::compose::is_shell;

pub(super) fn select_group_process(
    processes: &[ProcEntry],
    foreground_pgrp: u32,
) -> Option<String> {
    if let Some((_, kind)) = processes
        .iter()
        .filter(|process| process.pgrp == foreground_pgrp)
        .filter_map(|process| {
            agent::identify_process(&process.comm, &process.argv).map(|kind| (process, kind))
        })
        .min_by_key(|(process, _)| (process.pid != foreground_pgrp, process.pid))
    {
        return Some(kind.label().to_string());
    }

    processes
        .iter()
        .filter(|process| process.pgrp == foreground_pgrp)
        .min_by_key(|process| {
            (
                process.pid != foreground_pgrp,
                is_shell(&process.comm) || process.comm == "mux",
                process.pid,
            )
        })
        .map(|process| process.comm.clone())
}
