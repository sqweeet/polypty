use super::*;

#[test]
fn visible_agent_signals_override_recent_activity() {
    let blocked = parser_with("Allow command?\nPress Enter to confirm");
    assert_eq!(
        detect(
            AgentKind::Codex,
            &blocked,
            "Action Required",
            AgentState::Ready,
            Duration::ZERO
        ),
        AgentState::Blocked
    );
    let working = parser_with("• Working (esc to interrupt)");
    assert_eq!(
        detect(
            AgentKind::Codex,
            &working,
            "⠋ Codex",
            AgentState::Ready,
            Duration::from_secs(5)
        ),
        AgentState::Working
    );
    let ready = parser_with("────────────────\n❯ ");
    assert_eq!(
        detect(
            AgentKind::Claude,
            &ready,
            "✳ Claude",
            AgentState::Working,
            Duration::ZERO
        ),
        AgentState::Ready
    );
}

#[test]
fn claude_access_request_is_blocked() {
    let permission = parser_with(
        "Claude needs permission to use Bash\nDo you want to proceed?\n❯ 1. Yes\n  2. No",
    );
    assert_eq!(
        detect(
            AgentKind::Claude,
            &permission,
            "✳ Claude",
            AgentState::Ready,
            Duration::MAX,
        ),
        AgentState::Blocked
    );
}

#[test]
fn structured_working_row_outweighs_a_visible_composer() {
    let parser = parser_with("• Working (esc to interrupt)\n› editing a follow-up");
    assert_eq!(
        detect(
            AgentKind::Codex,
            &parser,
            "",
            AgentState::Working,
            Duration::ZERO
        ),
        AgentState::Working
    );
    assert_eq!(
        detect(
            AgentKind::Codex,
            &parser,
            "",
            AgentState::Ready,
            Duration::MAX
        ),
        AgentState::Ready
    );
    let keywords = parser_with("› explain esc to interrupt, working ( and permission required");
    assert_eq!(
        detect(
            AgentKind::Codex,
            &keywords,
            "",
            AgentState::Ready,
            Duration::ZERO
        ),
        AgentState::Ready
    );
    let mut rows = vec!["• Working (esc to interrupt)".to_string()];
    rows.extend((0..7).map(|index| format!("completed row {index}")));
    let stale = parser_with(&rows.join("\n"));
    assert_eq!(
        detect(
            AgentKind::Codex,
            &stale,
            "",
            AgentState::Working,
            Duration::MAX
        ),
        AgentState::Ready
    );
}

#[test]
fn live_spinner_remains_authoritative_while_composer_is_visible() {
    let parser = parser_with("› queued follow-up");
    assert_eq!(
        detect(
            AgentKind::Codex,
            &parser,
            "⠋ Codex",
            AgentState::Ready,
            Duration::MAX
        ),
        AgentState::Working
    );
}

#[test]
fn old_blocker_above_live_tail_does_not_win() {
    let mut rows = vec!["Allow command?".to_string()];
    rows.extend((0..13).map(|index| format!("row {index}")));
    rows.push("❯ ".to_string());
    let parser = parser_with(&rows.join("\n"));
    assert_eq!(
        detect(
            AgentKind::Claude,
            &parser,
            "✳ Claude",
            AgentState::Blocked,
            Duration::ZERO
        ),
        AgentState::Ready
    );
}
