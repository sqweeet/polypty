//! Linux `/proc` probing and foreground process selection.

use crate::agent;

use super::is_shell;

pub(super) fn probe_session(
    pid: u32,
    foreground_pgrp: Option<u32>,
) -> (Option<String>, Option<String>) {
    let cwd = std::fs::read_link(format!("/proc/{pid}/cwd"))
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()));

    let process = foreground_pgrp
        .and_then(foreground_process)
        .or_else(|| read_comm(pid));
    (cwd, process)
}

fn read_comm(pid: u32) -> Option<String> {
    let s = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    let name = s.trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

#[derive(Debug, Clone)]
struct ProcEntry {
    pid: u32,
    pgrp: u32,
    comm: String,
    argv: Vec<String>,
}

fn read_proc_stat(pid: u32) -> Option<(u32, String)> {
    // /proc/pid/stat: pid (comm) state ppid pgrp ...
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let rparen = stat.rfind(')')?;
    let after = stat[rparen + 2..].split_whitespace().collect::<Vec<_>>();
    // after[0] = state, after[1] = ppid, after[2] = pgrp
    let pgrp: u32 = after.get(2)?.parse().ok()?;
    let comm_start = stat.find('(')? + 1;
    let comm = stat[comm_start..rparen].to_string();
    Some((pgrp, comm))
}

fn read_cmdline(pid: u32) -> Vec<String> {
    std::fs::read(format!("/proc/{pid}/cmdline"))
        .ok()
        .map(|bytes| {
            bytes
                .split(|byte| *byte == 0)
                .filter(|part| !part.is_empty())
                .map(|part| String::from_utf8_lossy(part).into_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn foreground_process(foreground_pgrp: u32) -> Option<String> {
    let mut processes = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return None;
    };
    for ent in entries.flatten() {
        let name = ent.file_name();
        let Some(pid_str) = name.to_str() else {
            continue;
        };
        let Ok(pid) = pid_str.parse::<u32>() else {
            continue;
        };
        if let Some((pgrp, comm)) = read_proc_stat(pid) {
            if pgrp != foreground_pgrp {
                continue;
            }
            processes.push(ProcEntry {
                pid,
                pgrp,
                comm,
                argv: read_cmdline(pid),
            });
        }
    }
    select_group_process(&processes, foreground_pgrp)
}

fn select_group_process(processes: &[ProcEntry], foreground_pgrp: u32) -> Option<String> {
    // Prefer an identified agent in the foreground group, with the process
    // group leader winning ties. Background jobs never enter this candidate
    // set, regardless of PID or spawn time.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreground_group_excludes_newer_background_agent() {
        let processes = [
            ProcEntry {
                pid: 100,
                pgrp: 100,
                comm: "codex".into(),
                argv: vec!["codex".into()],
            },
            ProcEntry {
                pid: 999,
                pgrp: 999,
                comm: "node".into(),
                argv: vec![
                    "node".into(),
                    "/opt/node_modules/@anthropic-ai/claude-code/cli.js".into(),
                ],
            },
        ];

        assert_eq!(
            select_group_process(&processes, 100).as_deref(),
            Some("codex")
        );
        assert_eq!(
            select_group_process(&processes, 999).as_deref(),
            Some("claude")
        );
    }

    #[test]
    fn foreground_tmux_is_not_misidentified_as_an_agent() {
        let processes = [ProcEntry {
            pid: 42,
            pgrp: 42,
            comm: "tmux".into(),
            argv: vec!["tmux".into(), "codex".into()],
        }];
        assert_eq!(
            select_group_process(&processes, 42).as_deref(),
            Some("tmux")
        );
    }
}
