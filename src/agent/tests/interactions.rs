use super::*;

#[test]
fn local_interactions_reduce_without_reading_partial_redraws() {
    let stale = parser_with("• Working (esc to interrupt)");
    for interaction in [AgentInteraction::Editing, AgentInteraction::Resizing] {
        assert_eq!(
            detect_with_interaction(
                AgentKind::Codex,
                &stale,
                "",
                AgentState::Ready,
                Duration::ZERO,
                interaction,
            ),
            AgentState::Ready
        );
    }
    assert_eq!(
        detect_with_interaction(
            AgentKind::Codex,
            &stale,
            "",
            AgentState::Working,
            Duration::MAX,
            AgentInteraction::Resizing,
        ),
        AgentState::Working
    );
}

#[test]
fn acknowledged_submit_promotes_only_after_the_composer_disappears() {
    let blank = parser_with("");
    assert_eq!(
        detect_with_interaction(
            AgentKind::OpenCode,
            &blank,
            "",
            AgentState::Ready,
            Duration::MAX,
            AgentInteraction::SubmitPending,
        ),
        AgentState::Ready
    );
    assert_eq!(
        detect_with_interaction(
            AgentKind::OpenCode,
            &blank,
            "",
            AgentState::Ready,
            Duration::ZERO,
            AgentInteraction::Submitted,
        ),
        AgentState::Working
    );
    let prompt = parser_with("› explain esc to interrupt and permission required");
    assert_eq!(
        detect_with_interaction(
            AgentKind::Codex,
            &prompt,
            "",
            AgentState::Ready,
            Duration::ZERO,
            AgentInteraction::Submitted,
        ),
        AgentState::Ready
    );
}
