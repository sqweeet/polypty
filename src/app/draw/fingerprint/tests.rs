use std::time::Duration;

use crate::{
    agent::{AgentKind, AgentState, AgentStatus},
    render::{GlintFrame, SidebarTab},
};

use super::sidebar_fingerprint;

#[test]
fn semantic_agent_state_is_structural_but_glint_phase_is_not() {
    let mut tabs = vec![SidebarTab {
        key: 1,
        primary: "codex".into(),
        secondary: "~/projects/polypty".into(),
        agent: Some(AgentStatus::single(AgentKind::Codex, AgentState::Working)),
        glint_frame: Some(GlintFrame::for_elapsed(Duration::ZERO)),
        active: true,
    }];
    let working = sidebar_fingerprint(&tabs, true, 18, 24);
    tabs[0].agent.as_mut().unwrap().state = AgentState::Blocked;
    let blocked = sidebar_fingerprint(&tabs, true, 18, 24);
    tabs[0].agent.as_mut().unwrap().panes = 2;
    let split = sidebar_fingerprint(&tabs, true, 18, 24);
    assert_ne!(working, blocked);
    assert_ne!(blocked, split);

    let structural = sidebar_fingerprint(&tabs, true, 18, 24);
    tabs[0].glint_frame = Some(GlintFrame::for_elapsed(Duration::from_millis(800)));
    assert_eq!(structural, sidebar_fingerprint(&tabs, true, 18, 24));
}
