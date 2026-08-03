use super::{model::ProcEntry, selection::select_group_process};

#[test]
fn background_agent_is_excluded_from_foreground_group() {
    let processes = [
        ProcEntry {
            pid: 100,
            pgrp: 100,
            comm: "codex".into(),
            argv: vec!["codex".into()],
        },
        ProcEntry {
            pid: 999,
            pgrp: 999,
            comm: "node".into(),
            argv: vec![
                "node".into(),
                "/opt/node_modules/@anthropic-ai/claude-code/cli.js".into(),
            ],
        },
    ];
    assert_eq!(
        select_group_process(&processes, 100).as_deref(),
        Some("codex")
    );
    assert_eq!(
        select_group_process(&processes, 999).as_deref(),
        Some("claude")
    );
}

#[test]
fn foreground_tmux_is_not_an_agent() {
    let processes = [ProcEntry {
        pid: 42,
        pgrp: 42,
        comm: "tmux".into(),
        argv: vec!["tmux".into(), "codex".into()],
    }];
    assert_eq!(
        select_group_process(&processes, 42).as_deref(),
        Some("tmux")
    );
}
