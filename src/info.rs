//! Tab metadata — process + cwd, cmux-style.
//!
//! Primary line: foreground process or OSC title  
//! Secondary line: shortened working directory

use std::path::{Path, PathBuf};

use crate::agent::AgentStatus;

mod osc;
mod process;

pub use osc::OscTracker;

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
    process::probe_session(pid, foreground_pgrp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_prefers_process() {
        let i = compose_info("", Some("/home/u/code"), Some("nvim"), false);
        assert_eq!(i.primary, "nvim");
        assert!(i.secondary.contains('~') || i.secondary.contains("code"));
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
