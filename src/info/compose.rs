use std::path::Path;

use super::{shorten_path, TabInfo};

/// Build cmux-style primary and secondary labels from known metadata.
pub fn compose_info(
    osc_title: &str,
    cwd: Option<&str>,
    process: Option<&str>,
    custom_title: bool,
) -> TabInfo {
    let process_name = process.map(clean_process).filter(|name| !name.is_empty());
    let title = clean_title(osc_title);
    let primary = if custom_title && !title.is_empty() {
        title.clone()
    } else if let Some(name) = process_name.as_ref().filter(|name| !is_shell(name)) {
        name.clone()
    } else if !title.is_empty() && !is_boring_title(&title) {
        pretty_name(&title)
    } else if let Some(name) = process_name {
        name
    } else {
        "shell".into()
    };
    let secondary = cwd
        .map(shorten_path)
        .filter(|path| !path.is_empty())
        .unwrap_or_default();

    TabInfo {
        primary,
        secondary,
        agent: None,
    }
}

fn clean_title(value: &str) -> String {
    let mut title = value.trim().to_string();
    for suffix in [" - fish", " - bash", " - zsh", " - sh", " - nu"] {
        if let Some(stripped) = title.strip_suffix(suffix) {
            title = stripped.trim().to_string();
        }
    }
    title.trim_start_matches("* ").trim().to_string()
}

fn clean_process(value: &str) -> String {
    let value = value.trim();
    Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value)
        .to_string()
}

pub(super) fn is_shell(name: &str) -> bool {
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

fn is_boring_title(title: &str) -> bool {
    is_shell(title) || title.starts_with("tab ") || title == "~" || title.is_empty()
}

fn pretty_name(title: &str) -> String {
    if title.contains('/') || title.contains('\\') {
        Path::new(title)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(title)
            .to_string()
    } else {
        title.to_string()
    }
}
