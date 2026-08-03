use super::*;

#[test]
fn suppresses_and_restores_cursor_without_repainting_cells() {
    let parser = vt100::Parser::new(2, 3, 0);
    let layout = Layout::new(3, 2, false, 0);
    let mut cache = TermCache::default();
    let mut out = Vec::new();

    draw_terminal(&mut out, &layout, parser.screen(), &mut cache, true, true).unwrap();
    assert!(contains(&out, b"\x1b[?25l"));
    assert!(!contains(&out, b"\x1b[?25h"));

    out.clear();
    draw_terminal(&mut out, &layout, parser.screen(), &mut cache, false, false).unwrap();
    assert!(contains(&out, b"\x1b[?25h"));
}
