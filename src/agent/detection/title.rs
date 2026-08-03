use crate::agent::{AgentKind, AgentState};

pub(super) fn state_from_title(kind: AgentKind, title: &str) -> Option<AgentState> {
    let lower = title.to_ascii_lowercase();
    if lower.trim_matches(|ch: char| !ch.is_alphanumeric()) == "action required" {
        return Some(AgentState::Blocked);
    }
    if title.trim_start().chars().next().is_some_and(is_spinner) {
        return Some(AgentState::Working);
    }
    (kind == AgentKind::Claude && title.starts_with('✳')).then_some(AgentState::Ready)
}

fn is_spinner(ch: char) -> bool {
    ('\u{2801}'..='\u{28ff}').contains(&ch)
}
