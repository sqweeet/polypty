#[derive(Debug, Clone)]
pub(crate) struct SidebarShortcuts {
    pub new_tab: Option<String>,
    pub close_tab: Option<String>,
    pub next_tab: Option<String>,
    pub split_vertical: Option<String>,
    pub split_horizontal: Option<String>,
    pub next_pane: Option<String>,
    pub close_pane: Option<String>,
    pub toggle_sidebar: Option<String>,
    pub quit: Option<String>,
}

impl Default for SidebarShortcuts {
    fn default() -> Self {
        Self {
            new_tab: Some("Alt+t".into()),
            close_tab: Some("Alt+w".into()),
            next_tab: Some("Alt+[/]".into()),
            split_vertical: Some("Alt+v".into()),
            split_horizontal: Some("Alt+s".into()),
            next_pane: Some("Alt+hjkl".into()),
            close_pane: Some("Alt+x".into()),
            toggle_sidebar: Some("Alt+b".into()),
            quit: Some("Alt+q".into()),
        }
    }
}

#[cfg(test)]
pub(super) fn sidebar_footer(width: usize, height: usize) -> Vec<(u8, String)> {
    configured_footer(width, height, &SidebarShortcuts::default())
}

pub(super) fn configured_footer(
    width: usize,
    height: usize,
    shortcuts: &SidebarShortcuts,
) -> Vec<(u8, String)> {
    let full = rows(shortcuts, false);
    let max_rows = height.saturating_sub(3).min(full.len());
    if max_rows < 2 {
        return Vec::new();
    }
    if width >= 14 {
        full.into_iter().take(max_rows).collect()
    } else {
        rows(shortcuts, true).into_iter().take(max_rows).collect()
    }
}

fn rows(shortcuts: &SidebarShortcuts, compact: bool) -> Vec<(u8, String)> {
    let labels = if compact {
        ["new", "tab", "tab", "", "", "pane", "bar", "out"]
    } else {
        [
            "new tab",
            "close tab",
            "tabs",
            "split",
            "pane",
            "close pane",
            "sidebar",
            "quit",
        ]
    };
    let keys = vec![
        key(&shortcuts.new_tab),
        key(&shortcuts.close_tab),
        key(&shortcuts.next_tab),
        combined_key(&shortcuts.split_vertical, &shortcuts.split_horizontal),
        key(&shortcuts.next_pane),
        key(&shortcuts.close_pane),
        key(&shortcuts.toggle_sidebar),
        key(&shortcuts.quit),
    ];
    keys.into_iter()
        .zip(labels)
        .map(|(key, label)| (5, format!("{key} {label}")))
        .collect()
}

fn key(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| "—".into())
}

fn combined_key(first: &Option<String>, second: &Option<String>) -> String {
    match (first, second) {
        (Some(first), Some(second)) => compact_pair(first, second),
        (Some(value), None) | (None, Some(value)) => value.clone(),
        (None, None) => "—".into(),
    }
}

fn compact_pair(first: &str, second: &str) -> String {
    match (first.rsplit_once('+'), second.rsplit_once('+')) {
        (Some((first_prefix, first_key)), Some((second_prefix, second_key)))
            if first_prefix == second_prefix =>
        {
            format!("{first_prefix}+{first_key}/{second_key}")
        }
        _ => format!("{first}/{second}"),
    }
}
