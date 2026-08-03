use super::*;

#[test]
fn activity_falls_back_to_ready_after_a_quiet_window() {
    let blank = parser_with("");
    assert_eq!(
        detect(
            AgentKind::Gemini,
            &blank,
            "",
            AgentState::Ready,
            Duration::from_millis(200)
        ),
        AgentState::Working
    );
    assert_eq!(
        detect(
            AgentKind::Gemini,
            &blank,
            "",
            AgentState::Working,
            Duration::from_secs(2)
        ),
        AgentState::Ready
    );
}

#[test]
fn generic_activity_cannot_promote_explicit_state_agents() {
    let blank = parser_with("");
    for kind in [AgentKind::Codex, AgentKind::Claude, AgentKind::OpenCode] {
        assert_eq!(
            detect(kind, &blank, "", AgentState::Ready, Duration::ZERO),
            AgentState::Ready
        );
        assert_eq!(
            detect(kind, &blank, "", AgentState::Working, Duration::ZERO),
            AgentState::Working
        );
        assert_eq!(
            detect(
                kind,
                &blank,
                "",
                AgentState::Working,
                Duration::from_secs(2)
            ),
            AgentState::Ready
        );
    }
}
