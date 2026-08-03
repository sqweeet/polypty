use crate::{
    core::geometry::{Divider, TerminalRect},
    workspace::snapshot::{PaneSnapshot, WorkspaceSnapshot},
};

use super::{paint, WorkspaceRenderer};

fn snapshot(screen: &vt100::Screen, dirty: bool) -> WorkspaceSnapshot<'_> {
    WorkspaceSnapshot {
        id: 7,
        active: 1,
        panes: vec![PaneSnapshot {
            id: 1,
            rect: TerminalRect {
                x: 1,
                y: 0,
                cols: 8,
                rows: 3,
            },
            screen,
            dirty,
        }],
        dividers: vec![Divider::Vertical { x: 0, y: 0, len: 3 }],
    }
}

#[test]
fn renderer_detects_structural_and_terminal_damage() {
    let parser = vt100::Parser::new(3, 8, 0);
    let mut renderer = WorkspaceRenderer::default();
    let clean = snapshot(parser.screen(), false);
    assert!(renderer.needs_draw(&clean));
    renderer.sync_geometry(&clean);
    renderer.finish(&clean);
    assert!(!renderer.needs_draw(&clean));

    let dirty = snapshot(parser.screen(), true);
    assert!(renderer.needs_draw(&dirty));
}

#[test]
fn dirty_terminal_frame_does_not_repaint_unchanged_dividers() {
    let parser = vt100::Parser::new(3, 8, 0);
    let mut renderer = WorkspaceRenderer::default();
    let mut initial = Vec::new();
    paint(
        &mut renderer,
        &mut initial,
        &snapshot(parser.screen(), true),
        true,
        false,
    )
    .unwrap();
    assert!(initial
        .windows("│".len())
        .any(|bytes| bytes == "│".as_bytes()));

    let mut next = Vec::new();
    paint(
        &mut renderer,
        &mut next,
        &snapshot(parser.screen(), true),
        true,
        false,
    )
    .unwrap();
    assert!(!next.windows("│".len()).any(|bytes| bytes == "│".as_bytes()));
}
