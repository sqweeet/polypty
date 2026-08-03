use std::io::Write;

use anyhow::Result;

use crate::{render, workspace::snapshot::WorkspaceSnapshot};

pub(in crate::render) fn restore_cursor(
    output: &mut impl Write,
    snapshot: &WorkspaceSnapshot<'_>,
    suppress_cursor: bool,
) -> Result<()> {
    let Some(pane) = snapshot
        .panes
        .iter()
        .find(|pane| pane.id == snapshot.active)
    else {
        return Ok(());
    };
    render::restore_terminal_cursor(output, pane.rect, pane.screen, suppress_cursor)
}
