use super::*;

#[test]
fn action_title_requires_an_exact_signal() {
    let blank = parser_with("");
    assert_eq!(
        detect(
            AgentKind::Codex,
            &blank,
            "fix action required detector",
            AgentState::Ready,
            Duration::MAX,
        ),
        AgentState::Ready
    );
    assert_eq!(
        detect(
            AgentKind::Codex,
            &blank,
            "[!] Action Required",
            AgentState::Ready,
            Duration::MAX,
        ),
        AgentState::Blocked
    );
}

#[test]
fn opencode_interrupt_footer_is_working() {
    let parser = parser_with("BUILD  ■⬝⬝  esc interrupt");
    assert_eq!(
        detect(
            AgentKind::OpenCode,
            &parser,
            "",
            AgentState::Ready,
            Duration::MAX,
        ),
        AgentState::Working
    );
}
