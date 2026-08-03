use super::*;

#[test]
fn agent_status_keeps_working_label_minimal() {
    let mut tabs = [SidebarTab {
        key: 1,
        primary: "node".into(),
        secondary: "~/projects/mux".into(),
        agent: Some(AgentStatus::single(AgentKind::Codex, AgentState::Working)),
        glint_frame: Some(GlintFrame(10)),
        active: true,
    }];

    let cards = build_cards(&tabs, 18);
    assert_eq!(cards[0].lines.len(), 2);
    assert_eq!(cards[0].lines[0], (7, "codex".into()));
    assert_eq!(cards[0].lines[1], (3, "~/projects/mux".into()));

    tabs[0].agent.as_mut().unwrap().state = AgentState::Ready;
    assert_eq!(build_cards(&tabs, 18)[0].lines[0], (6, "codex".into()));
    tabs[0].agent.as_mut().unwrap().state = AgentState::Blocked;
    assert_eq!(
        build_cards(&tabs, 18)[0].lines[0],
        (8, "codex · blocked".into())
    );
}

#[test]
fn agent_status_labels_split_panes_compactly() {
    let mut tabs = [SidebarTab {
        key: 1,
        primary: "node".into(),
        secondary: "~/projects/mux".into(),
        agent: Some(AgentStatus {
            kind: AgentKind::Codex,
            state: AgentState::Working,
            panes: 2,
            mixed_kinds: false,
        }),
        glint_frame: Some(GlintFrame(10)),
        active: true,
    }];

    assert_eq!(build_cards(&tabs, 18)[0].lines[0], (7, "codex ×2".into()));
    tabs[0].agent.as_mut().unwrap().state = AgentState::Ready;
    assert_eq!(build_cards(&tabs, 18)[0].lines[0], (6, "codex ×2".into()));
    tabs[0].agent.as_mut().unwrap().state = AgentState::Blocked;
    assert_eq!(
        build_cards(&tabs, 18)[0].lines[0],
        (8, "codex ×2 · blocked".into())
    );

    let status = tabs[0].agent.as_mut().unwrap();
    status.kind = AgentKind::Claude;
    status.state = AgentState::Working;
    status.mixed_kinds = true;
    assert_eq!(build_cards(&tabs, 18)[0].lines[0], (7, "claude+1".into()));
    tabs[0].agent.as_mut().unwrap().state = AgentState::Blocked;
    assert_eq!(
        build_cards(&tabs, 18)[0].lines[0],
        (8, "claude+1 · blocked".into())
    );
    assert_eq!(
        build_cards(&tabs, 12)[0].lines[0],
        (8, "c… · blocked".into())
    );
    assert_eq!(build_cards(&tabs, 8)[0].lines[0], (8, "blocked".into()));
    assert_eq!(build_cards(&tabs, 5)[0].lines[0], (8, "!".into()));
}
