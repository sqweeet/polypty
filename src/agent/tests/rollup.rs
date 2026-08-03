use crate::agent::{rollup, AgentKind, AgentState, AgentStatus};

#[test]
fn counts_panes_and_preserves_priority_and_first_tie() {
    let codex_working = AgentStatus::single(AgentKind::Codex, AgentState::Working);
    let codex_ready = AgentStatus::single(AgentKind::Codex, AgentState::Ready);
    let claude_working = AgentStatus::single(AgentKind::Claude, AgentState::Working);
    let opencode_blocked = AgentStatus::single(AgentKind::OpenCode, AgentState::Blocked);

    assert_eq!(
        rollup([codex_working, codex_ready]),
        Some(AgentStatus {
            panes: 2,
            ..codex_working
        })
    );
    assert_eq!(
        rollup([codex_working, codex_ready, codex_working]),
        Some(AgentStatus {
            panes: 3,
            ..codex_working
        })
    );
    assert_eq!(
        rollup([codex_working, claude_working]),
        Some(AgentStatus {
            panes: 2,
            mixed_kinds: true,
            ..codex_working
        })
    );
    assert_eq!(
        rollup([codex_working, opencode_blocked]),
        Some(AgentStatus {
            panes: 2,
            mixed_kinds: true,
            ..opencode_blocked
        })
    );
}
