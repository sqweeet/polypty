use std::time::Instant;

use crate::render::Layout;

use super::cache::SidebarPaintRow;
use super::card::build_animated_cards;
use super::footer::{configured_footer, SidebarShortcuts};
use super::hit_map::SidebarMap;
use super::model::{SidebarContentRow, SidebarTab};
use super::palette::SidebarPalette;
use super::tab_motion::TabMotion;
use super::viewport::sidebar_content_rows;

pub(super) struct SidebarFrame {
    pub(super) map: SidebarMap,
    pub(super) rows: Vec<SidebarPaintRow>,
}

pub(super) fn build_sidebar_frame(
    layout: &Layout,
    tabs: &[SidebarTab],
    shortcuts: &SidebarShortcuts,
    motion: &TabMotion,
    now: Instant,
) -> SidebarFrame {
    let width = usize::from(layout.sidebar_width);
    let height = usize::from(layout.rows);
    let card_width = width.saturating_sub(usize::from(width > 1)).max(1);
    let cards = build_animated_cards(tabs, card_width, motion, now);
    let footer = configured_footer(width, height, shortcuts);
    let footer_start = height.saturating_sub(footer.len());
    let content = sidebar_content_rows(&cards, footer_start);
    let palette = SidebarPalette::new();
    let mut map = SidebarMap::empty(layout.rows, layout.sidebar_width);
    let mut rows = Vec::with_capacity(height);

    for row_index in 0..height {
        let row = if row_index >= footer_start {
            let (kind, text) = &footer[row_index - footer_start];
            SidebarContentRow::chrome(*kind, text)
        } else {
            content
                .get(row_index)
                .cloned()
                .unwrap_or_else(SidebarContentRow::empty)
        };
        map.record(row_index, &row);
        rows.push(palette.paint(&row, width));
    }
    SidebarFrame { map, rows }
}
