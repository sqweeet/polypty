use std::time::Instant;

use crate::agent::{AgentState, AgentStatus};

use super::model::{SidebarTab, TabCard};
use super::tab_motion::{TabMotion, TabVisual};
use super::text::{pad_fit, wrap_text};

fn agent_primary_label(status: AgentStatus) -> String {
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
    identity
}

#[cfg(test)]
pub(super) fn build_cards(tabs: &[SidebarTab], inner_width: usize) -> Vec<TabCard> {
    build_cards_with(tabs, inner_width, |tab| TabVisual::settled(tab.active))
}

pub(super) fn build_animated_cards(
    tabs: &[SidebarTab],
    inner_width: usize,
    motion: &TabMotion,
    now: Instant,
) -> Vec<TabCard> {
    build_cards_with(tabs, inner_width, |tab| {
        motion.visual(tab.key, tab.active, now)
    })
}

fn build_cards_with(
    tabs: &[SidebarTab],
    inner_width: usize,
    visual: impl Fn(&SidebarTab) -> TabVisual,
) -> Vec<TabCard> {
    let text_width = inner_width.max(1);
    tabs.iter()
        .enumerate()
        .map(|(tab_idx, tab)| {
            let (primary_kind, primary) = primary_line(tab);
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
                visual: visual(tab),
                lines,
            }
        })
        .collect()
}

fn primary_line(tab: &SidebarTab) -> (u8, String) {
    if let Some(status) = tab.agent {
        let kind = match status.state {
            AgentState::Ready => 6,
            AgentState::Working => 7,
            AgentState::Blocked => 8,
        };
        (kind, agent_primary_label(status))
    } else if tab.primary.is_empty() {
        (2, "shell".to_string())
    } else {
        (2, tab.primary.clone())
    }
}
