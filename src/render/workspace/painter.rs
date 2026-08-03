use std::io::Write;

use anyhow::Result;

use crate::{render, workspace::snapshot::WorkspaceSnapshot};

use super::WorkspaceRenderer;

pub(in crate::render) fn paint(
    renderer: &mut WorkspaceRenderer,
    output: &mut impl Write,
    snapshot: &WorkspaceSnapshot<'_>,
    suppress_active_cursor: bool,
    force: bool,
) -> Result<Vec<u64>> {
    if force {
        renderer.invalidate();
    }
    let redraw_all = renderer.redraw_all(force);
    let chrome_changed = renderer.chrome_changed(snapshot);
    let geometry_changed = renderer.sync_geometry(snapshot);
    if redraw_all || geometry_changed || chrome_changed {
        render::draw_dividers(output, &snapshot.dividers)?;
    }

    let mut painted = Vec::new();
    for pane in snapshot
        .panes
        .iter()
        .filter(|pane| pane.id != snapshot.active)
    {
        if pane.dirty || geometry_changed || redraw_all {
            let state = renderer.pane_mut(pane.id);
            render::draw_terminal_rect(
                output,
                pane.rect,
                pane.screen,
                &mut state.cache,
                false,
                true,
            )?;
            painted.push(pane.id);
        }
    }

    if let Some(pane) = snapshot
        .panes
        .iter()
        .find(|pane| pane.id == snapshot.active)
    {
        let state = renderer.pane_mut(pane.id);
        render::draw_terminal_rect(
            output,
            pane.rect,
            pane.screen,
            &mut state.cache,
            false,
            suppress_active_cursor,
        )?;
        painted.push(pane.id);
    }
    renderer.finish(snapshot);
    Ok(painted)
}
