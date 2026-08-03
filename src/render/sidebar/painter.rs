use std::io::Write;

use anyhow::Result;
use crossterm::cursor::{Hide, MoveTo};
use crossterm::queue;
use crossterm::style::{
    Attribute, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};

use crate::render::Layout;

use super::cache::{SidebarCache, SidebarPaintRow};
use super::footer::SidebarShortcuts;
use super::frame::build_sidebar_frame;
use super::hit_map::SidebarMap;
use super::model::SidebarTab;

#[cfg(test)]
pub fn draw_sidebar(
    out: &mut impl Write,
    layout: &Layout,
    tabs: &[SidebarTab],
    cache: &mut SidebarCache,
    force: bool,
) -> Result<SidebarMap> {
    draw_sidebar_with_shortcuts(
        out,
        layout,
        tabs,
        cache,
        force,
        &SidebarShortcuts::default(),
    )
}

pub(super) fn draw_sidebar_with_shortcuts(
    out: &mut impl Write,
    layout: &Layout,
    tabs: &[SidebarTab],
    cache: &mut SidebarCache,
    force: bool,
    shortcuts: &SidebarShortcuts,
) -> Result<SidebarMap> {
    if !layout.sidebar_visible || layout.sidebar_width == 0 {
        return Ok(SidebarMap::empty(layout.rows, layout.sidebar_width));
    }

    let frame = build_sidebar_frame(layout, tabs, shortcuts);
    paint_changed_rows(out, layout.sidebar_width, &frame.rows, cache, force)?;
    Ok(frame.map)
}

fn paint_changed_rows(
    out: &mut impl Write,
    width: u16,
    rows: &[SidebarPaintRow],
    cache: &mut SidebarCache,
    force: bool,
) -> Result<()> {
    let width_changed = cache.width != width;
    let mut cursor_hidden = false;
    for (row_index, row) in rows.iter().enumerate() {
        if !force && !width_changed && cache.rows.get(row_index) == Some(row) {
            continue;
        }
        if !cursor_hidden {
            queue!(out, Hide)?;
            cursor_hidden = true;
        }
        queue!(out, MoveTo(0, row_index as u16))?;
        for span in &row.spans {
            queue!(
                out,
                SetForegroundColor(span.fg),
                SetBackgroundColor(span.bg),
                Print(&span.text)
            )?;
        }
    }

    cache.width = width;
    cache.rows = rows.to_vec();
    if cursor_hidden {
        queue!(out, ResetColor, SetAttribute(Attribute::Reset))?;
    }
    Ok(())
}
