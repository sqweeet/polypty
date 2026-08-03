use unicode_width::UnicodeWidthStr;

use crate::agent::{AgentState, AgentStatus};

use super::model::{SidebarTab, TabCard};
use super::text::{pad_fit, wrap_text};

fn agent_primary_label(status: AgentStatus, width: usize) -> String {
    let count = status.panes.max(1);
    let mut identity = status.kind.label().to_string();
    if count > 1 {
        let suffix = if status.mixed_kinds {
            format!("+{}", count - 1)
        } else {
            format!(" ×{count}")
        };
        identity.push_str(&suffix);
    }
    if status.state != AgentState::Blocked {
        return identity;
    }

    const BLOCKED: &str = "blocked";
    const SUFFIX: &str = " · blocked";
    if width < UnicodeWidthStr::width(BLOCKED) {
        return "!".to_string();
    }
    if width <= UnicodeWidthStr::width(SUFFIX) {
        return BLOCKED.to_string();
    }

    let identity_width = width - UnicodeWidthStr::width(SUFFIX);
    let identity = wrap_text(&identity, identity_width, 1)
        .into_iter()
        .next()
        .unwrap_or_default();
    format!("{identity}{SUFFIX}")
}

pub(super) fn build_cards(tabs: &[SidebarTab], inner_width: usize) -> Vec<TabCard> {
    let text_width = inner_width.max(1);
    tabs.iter()
        .enumerate()
        .map(|(tab_idx, tab)| {
            let (primary_kind, primary) = primary_line(tab, text_width);
            let mut lines: Vec<_> = wrap_text(&primary, text_width, 1)
                .into_iter()
                .map(|line| (primary_kind, line))
                .collect();
            if lines.is_empty() {
                lines.push((primary_kind, wrap_text("shell", text_width, 1)[0].clone()));
            }
            if !tab.secondary.is_empty() {
                let secondary = pad_fit(tab.secondary.trim(), text_width);
                let secondary = secondary.trim_end();
                if !secondary.is_empty() {
                    lines.push((3, secondary.to_string()));
                }
            }
            TabCard {
                tab_idx,
                key: tab.key,
                active: tab.active,
                agent_state: tab.agent.map(|status| status.state),
                glint_frame: tab.glint_frame,
                lines,
            }
        })
        .collect()
}

fn primary_line(tab: &SidebarTab, width: usize) -> (u8, String) {
    if let Some(status) = tab.agent {
        let kind = match status.state {
            AgentState::Ready => 6,
            AgentState::Working => 7,
            AgentState::Blocked => 8,
        };
        (kind, agent_primary_label(status, width))
    } else if tab.primary.is_empty() {
        (2, "shell".to_string())
    } else {
        (2, tab.primary.clone())
    }
}
