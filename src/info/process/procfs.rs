use super::model::ProcEntry;

pub(super) fn read_cwd(pid: u32) -> Option<String> {
    std::fs::read_link(format!("/proc/{pid}/cwd"))
        .ok()
        .and_then(|path| path.to_str().map(str::to_owned))
}

pub(super) fn read_comm(pid: u32) -> Option<String> {
    let name = std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()?
        .trim()
        .to_string();
    (!name.is_empty()).then_some(name)
}

pub(super) fn read_group(group: u32) -> Vec<ProcEntry> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.parse::<u32>().ok())
        .filter_map(|pid| read_entry(pid, group))
        .collect()
}

fn read_entry(pid: u32, group: u32) -> Option<ProcEntry> {
    let (pgrp, comm) = read_stat(pid)?;
    if pgrp != group {
        return None;
    }
    Some(ProcEntry {
        pid,
        pgrp,
        comm,
        argv: read_cmdline(pid),
    })
}

fn read_stat(pid: u32) -> Option<(u32, String)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let close = stat.rfind(')')?;
    let fields: Vec<_> = stat[close + 2..].split_whitespace().collect();
    let pgrp = fields.get(2)?.parse().ok()?;
    let open = stat.find('(')? + 1;
    Some((pgrp, stat[open..close].to_string()))
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
