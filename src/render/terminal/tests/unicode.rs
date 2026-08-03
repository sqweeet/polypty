use super::*;

#[test]
fn renderer_uses_vt_cell_geometry_for_unicode() {
    let mut child = vt100::Parser::new(2, 8, 0);
    child.process("☰X\r\n界e\u{301}Z".as_bytes());

    let menu = child.screen().cell(0, 0).unwrap();
    let menu_width = cell_to_paint(Some(menu)).width;
    assert_eq!(menu_width, if menu.is_wide() { 2 } else { 1 });
    assert_eq!(
        child
            .screen()
            .cell(0, u16::from(menu_width))
            .unwrap()
            .contents(),
        "X"
    );
    assert!(child.screen().cell(1, 0).unwrap().is_wide());
    assert!(child.screen().cell(1, 1).unwrap().is_wide_continuation());
    assert_eq!(child.screen().cell(1, 2).unwrap().contents(), "e\u{301}");
    assert_eq!(cell_to_paint(child.screen().cell(1, 2)).width, 1);

    let layout = Layout::new(8, 2, false, 0);
    let mut cache = TermCache::default();
    let mut host = vt100::Parser::new(2, 8, 0);
    host.process(&render_frame(&layout, child.screen(), &mut cache, true));

    assert_same_cells(child.screen(), host.screen(), 2, 8);
    assert_eq!(
        host.screen()
            .cell(0, u16::from(menu_width))
            .unwrap()
            .contents(),
        "X",
        "the cell following ☰ must not be swallowed by a width mismatch"
    );
}
