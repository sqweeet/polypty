use super::*;
use crate::workspace::layout::compact_horizontal_extent;

#[test]
fn horizontal_split_hugs_sparse_content() {
    let mut parser = vt100::Parser::new(38, 49, 0);
    parser.process(b"one\r\ntwo\x1b[11;1H");

    assert_eq!(compact_horizontal_extent(parser.screen(), 38), Some(11));
}

#[test]
fn horizontal_split_keeps_shells_and_busy_screens_half() {
    let sparse = vt100::Parser::new(38, 49, 0);
    assert_eq!(compact_horizontal_extent(sparse.screen(), 38), None);

    let mut busy = vt100::Parser::new(38, 49, 0);
    busy.process(b"\x1b[30;1Hbusy");
    assert_eq!(compact_horizontal_extent(busy.screen(), 38), None);
}

#[test]
fn compact_extent_survives_growth_and_clamps_without_zero_panes() {
    let mut tree = SplitTree::new(1);
    split(&mut tree, 1, 2, SplitAxis::Horizontal, Some(11));
    let area = TerminalRect {
        x: 0,
        y: 0,
        cols: 20,
        rows: 60,
    };
    let grown = layout(&tree, area, 2);
    assert_eq!(grown.panes[0].1.rows, 11);
    assert_eq!(grown.panes[1].1.rows, 48);

    let tiny = layout(&tree, TerminalRect { rows: 8, ..area }, 2);
    assert_eq!(tiny.panes[0].1.rows, 6);
    assert_eq!(tiny.panes[1].1.rows, 1);
}
