//! Tab metadata — process + cwd, cmux-style.
//!
//! Primary line: foreground process or OSC title  
//! Secondary line: shortened working directory

use std::path::{Path, PathBuf};

use crate::agent::{self, AgentStatus};

/// Parsed OSC 7 cwd from a stream of PTY bytes (stateful).
#[derive(Debug, Default)]
pub struct OscTracker {
    buf: Vec<u8>,
    in_osc: bool,
    title_revision: u64,
    pub cwd: Option<String>,
    /// Last OSC 0/2 window title. `Some("")` represents an explicit clear,
    /// while `None` means the child has not emitted a title yet.
    pub title: Option<String>,
}

impl OscTracker {
    pub fn title_revision(&self) -> u64 {
        self.title_revision
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if !self.in_osc {
                // ESC ]
                if b == 0x1b {
                    self.buf.clear();
                    self.buf.push(b);
                } else if self.buf == [0x1b] && b == b']' {
                    self.buf.push(b);
                    self.in_osc = true;
                } else {
                    self.buf.clear();
                }
                continue;
            }

            self.buf.push(b);
            // BEL or ST (ESC \) terminate OSC
            let done = b == 0x07
                || (b == b'\\' && self.buf.len() >= 2 && self.buf[self.buf.len() - 2] == 0x1b);
            if !done {
                // Cap runaway sequences
                if self.buf.len() > 4096 {
                    self.buf.clear();
                    self.in_osc = false;
                }
                continue;
            }

            self.in_osc = false;
            let seq = std::mem::take(&mut self.buf);
            self.handle_osc(&seq);
        }
    }

    fn handle_osc(&mut self, seq: &[u8]) {
        // ESC ] <body> BEL/ST
        if seq.len() < 4 || seq[0] != 0x1b || seq[1] != b']' {
            return;
        }
        let end = if seq.ends_with(&[0x1b, b'\\']) {
            seq.len() - 2
        } else if seq.ends_with(&[0x07]) {
            seq.len() - 1
        } else {
            return;
        };
        let body = &seq[2..end];
        let Some(separator) = body.iter().position(|byte| *byte == b';') else {
            return;
        };
        let (command, value) = (&body[..separator], &body[separator + 1..]);

        match command {
            // OSC 0 sets icon + window title; OSC 2 sets window title.
            b"0" | b"2" => {
                self.title = Some(sanitize_osc_title(value));
                self.title_revision = self.title_revision.wrapping_add(1);
            }
            // 7;file://host/path
            b"7" => {
                if let Ok(s) = std::str::from_utf8(value) {
                    if let Some(path) = parse_osc7(s) {
                        self.cwd = Some(path);
                    }
                }
            }
            _ => {}
        }
    }
}

fn sanitize_osc_title(bytes: &[u8]) -> String {
    // Never let a child smuggle terminal controls into mux's own sidebar.
    // Lossy UTF-8 keeps metadata displayable without affecting PTY parsing.
    String::from_utf8_lossy(bytes)
        .chars()
        .filter(|ch| !ch.is_control())
        .collect()
}

fn parse_osc7(s: &str) -> Option<String> {
    // file://hostname/path or file:///path
    let s = s.trim();
    let rest = s.strip_prefix("file://")?;
    // skip host
    let path = &rest[rest.find('/')?..];
    // percent-decode lightly
    let decoded = percent_decode(path)?;
    if decoded.is_empty() {
        None
    } else {
        Some(decoded)
    }
}

fn percent_decode(s: &str) -> Option<String> {
    // Decode into bytes first. Percent escapes describe URL bytes, not
    // Unicode scalar values; converting every decoded byte directly to char
    // corrupts both percent-encoded UTF-8 and ordinary non-ASCII paths.
    let mut out = Vec::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (from_hex(b[i + 1]), from_hex(b[i + 2])) {
                out.push(h << 4 | l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).ok()
}

fn from_hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Snapshot of live tab info for the sidebar (cmux-like).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TabInfo {
    pub primary: String,
    pub secondary: String,
    pub agent: Option<AgentStatus>,
}

/// Build cmux-style primary/secondary from known metadata.
pub fn compose_info(
    osc_title: &str,
    cwd: Option<&str>,
    process: Option<&str>,
    custom_title: bool,
) -> TabInfo {
    let proc_name = process.map(clean_process).filter(|s| !s.is_empty());
    let title = clean_title(osc_title);

    // cmux: process title drives the row when the user hasn't renamed;
    // idle shell falls back to a short label, path goes on the second line.
    let primary = if custom_title && !title.is_empty() {
        title.clone()
    } else if let Some(p) = proc_name.as_ref().filter(|p| !is_shell(p)) {
        p.clone()
    } else if !title.is_empty() && !is_boring_title(&title) {
        // Prefer last path segment of title if it looks like a path
        pretty_name(&title)
    } else if let Some(p) = proc_name {
        p
    } else {
        "shell".into()
    };

    let secondary = cwd
        .map(shorten_path)
        .filter(|s| !s.is_empty())
        .unwrap_or_default();

    TabInfo {
        primary,
        secondary,
        agent: None,
    }
}

fn clean_title(s: &str) -> String {
    let mut t = s.trim().to_string();
    for suf in [" - fish", " - bash", " - zsh", " - sh", " - nu"] {
        if let Some(x) = t.strip_suffix(suf) {
            t = x.trim().to_string();
        }
    }
    t.trim_start_matches("* ").trim().to_string()
}

fn clean_process(s: &str) -> String {
    let s = s.trim();
    // Strip path from argv0
    Path::new(s)
        .file_name()
        .and_then(|x| x.to_str())
        .unwrap_or(s)
        .to_string()
}

fn is_shell(name: &str) -> bool {
    matches!(
        name,
        "bash"
            | "zsh"
            | "fish"
            | "sh"
            | "dash"
            | "nu"
            | "nushell"
            | "tmux"
            | "screen"
            | "login"
            | "mux"
    )
}

fn is_boring_title(t: &str) -> bool {
    is_shell(t) || t.starts_with("tab ") || t == "~" || t.is_empty()
}

fn pretty_name(t: &str) -> String {
    if t.contains('/') || t.contains('\\') {
        Path::new(t)
            .file_name()
            .and_then(|x| x.to_str())
            .unwrap_or(t)
            .to_string()
    } else {
        t.to_string()
    }
}

/// cmux-like path shortener: `$HOME` → `~`, keep it compact.
pub fn shorten_path(path: &str) -> String {
    let path = path.trim();
    if path.is_empty() {
        return String::new();
    }
    let home = dirs_home();
    let mut p = path.to_string();
    if let Some(ref h) = home {
        if p == *h {
            return "~".into();
        }
        let prefix = format!("{h}/");
        if let Some(rest) = p.strip_prefix(&prefix) {
            p = format!("~/{rest}");
        }
    }
    // Collapse long paths: ~/a/b/c/d/e → ~/a/…/d/e when too long
    const MAX: usize = 28;
    if unicode_width::UnicodeWidthStr::width(p.as_str()) <= MAX {
        return p;
    }
    let parts: Vec<&str> = p.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() <= 3 {
        return truncate_middle(&p, MAX);
    }
    // keep first (maybe ~) + last two
    let head = parts[0];
    let tail = &parts[parts.len() - 2..];
    let short = format!("{head}/…/{}", tail.join("/"));
    if unicode_width::UnicodeWidthStr::width(short.as_str()) <= MAX {
        short
    } else {
        truncate_middle(&short, MAX)
    }
}

fn truncate_middle(s: &str, max: usize) -> String {
    let w = unicode_width::UnicodeWidthStr::width(s);
    if w <= max {
        return s.to_string();
    }
    if max <= 3 {
        return "…".into();
    }
    let keep = max - 1;
    let left = keep / 2;
    let right = keep - left;
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= keep {
        return s.to_string();
    }
    let mut out: String = chars.iter().take(left).collect();
    out.push('…');
    out.extend(
        chars
            .iter()
            .rev()
            .take(right)
            .collect::<Vec<_>>()
            .into_iter()
            .rev(),
    );
    out
}

fn dirs_home() -> Option<String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .and_then(|p| p.to_str().map(|s| s.to_string()))
}

/// Read cwd + the terminal's actual foreground process group (Linux /proc).
pub fn probe_session(pid: u32, foreground_pgrp: Option<u32>) -> (Option<String>, Option<String>) {
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
    fn osc7_file_url() {
        let mut t = OscTracker::default();
        t.feed(b"\x1b]7;file://host/home/gotlib/proj\x07");
        assert_eq!(t.cwd.as_deref(), Some("/home/gotlib/proj"));
    }

    #[test]
    fn osc7_decodes_percent_encoded_and_raw_utf8() {
        let mut t = OscTracker::default();
        t.feed(b"\x1b]7;file://host/home/%E7%95%8C%20project\x07");
        assert_eq!(t.cwd.as_deref(), Some("/home/界 project"));

        for chunk in "\x1b]7;file:///tmp/שלום\x1b\\".as_bytes().chunks(3) {
            t.feed(chunk);
        }
        assert_eq!(t.cwd.as_deref(), Some("/tmp/שלום"));

        t.feed(b"\x1b]7;file:///%FF\x07");
        assert_eq!(
            t.cwd.as_deref(),
            Some("/tmp/שלום"),
            "invalid UTF-8 must not replace the last valid cwd"
        );
    }

    #[test]
    fn osc_titles_are_stateful_sanitized_and_can_be_cleared() {
        let mut t = OscTracker::default();
        for chunk in "\x1b]0;агент\x1b\\".as_bytes().chunks(2) {
            t.feed(chunk);
        }
        assert_eq!(t.title.as_deref(), Some("агент"));

        t.feed(b"\x1b");
        t.feed(b"]2;safe\x01 title\t\x7f\x1b");
        t.feed(b"\\");
        assert_eq!(t.title.as_deref(), Some("safe title"));

        t.feed(b"\x1b]7;file://host/work/tree\x07");
        assert_eq!(t.cwd.as_deref(), Some("/work/tree"));
        assert_eq!(t.title.as_deref(), Some("safe title"));

        t.feed(b"\x1b]2;\x07");
        assert_eq!(t.title.as_deref(), Some(""));
        assert_eq!(t.cwd.as_deref(), Some("/work/tree"));
    }

    #[test]
    fn compose_prefers_process() {
        let i = compose_info("", Some("/home/u/code"), Some("nvim"), false);
        assert_eq!(i.primary, "nvim");
        assert!(i.secondary.contains('~') || i.secondary.contains("code"));
    }

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

    #[test]
    fn shorten_home() {
        if let Some(h) = dirs_home() {
            let p = format!("{h}/projects/mux");
            let s = shorten_path(&p);
            assert!(s.starts_with("~/"), "{s}");
        }
    }
}
