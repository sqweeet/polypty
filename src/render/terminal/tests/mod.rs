use vt100::Screen;

use super::cell::cell_to_paint;
use super::{draw_terminal_rect, TermCache};
use crate::render::{begin_sync, enable_color_passthrough, end_sync, Layout};

mod agent_frame;
mod cache;
mod cursor;
mod resize;
mod style;
mod unicode;

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn draw_terminal(
    out: &mut impl std::io::Write,
    layout: &Layout,
    screen: &Screen,
    cache: &mut TermCache,
    force: bool,
    suppress_cursor: bool,
) -> anyhow::Result<()> {
    draw_terminal_rect(
        out,
        layout.terminal_rect(),
        screen,
        cache,
        force,
        suppress_cursor,
    )
}

fn render_frame(layout: &Layout, screen: &Screen, cache: &mut TermCache, force: bool) -> Vec<u8> {
    enable_color_passthrough();
    let mut out = Vec::new();
    begin_sync(&mut out).unwrap();
    draw_terminal(&mut out, layout, screen, cache, force, false).unwrap();
    end_sync(&mut out).unwrap();
    out
}

fn assert_same_cells(left: &Screen, right: &Screen, rows: u16, cols: u16) {
    for row in 0..rows {
        for col in 0..cols {
            assert_eq!(
                cell_to_paint(left.cell(row, col)),
                cell_to_paint(right.cell(row, col)),
                "cell mismatch at ({row}, {col})"
            );
        }
    }
}
