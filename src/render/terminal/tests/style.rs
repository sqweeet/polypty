use super::super::cell::ATTR_DIM;
use super::*;

#[test]
fn renderer_preserves_dim_cells() {
    let mut child = vt100::Parser::new(1, 8, 0);
    child.process(b"\x1b[2mdim\x1b[22mN");

    assert!(child.screen().cell(0, 0).unwrap().dim());
    assert!(!child.screen().cell(0, 3).unwrap().dim());
    assert_ne!(cell_to_paint(child.screen().cell(0, 0)).attrs & ATTR_DIM, 0);

    let layout = Layout::new(8, 1, false, 0);
    let mut cache = TermCache::default();
    let frame = render_frame(&layout, child.screen(), &mut cache, true);
    assert!(contains(&frame, b"\x1b[2m"));

    let mut host = vt100::Parser::new(1, 8, 0);
    host.process(&frame);
    assert_same_cells(child.screen(), host.screen(), 1, 8);
    assert!(host.screen().cell(0, 0).unwrap().dim());
    assert!(!host.screen().cell(0, 3).unwrap().dim());
}

#[test]
fn renderer_preserves_inverse_with_default_and_explicit_colors() {
    let mut child = vt100::Parser::new(1, 8, 0);
    child.process(b"\x1b[7mD\x1b[27m \x1b[31;44;7mC\x1b[0m");

    assert!(child.screen().cell(0, 0).unwrap().inverse());
    assert!(child.screen().cell(0, 2).unwrap().inverse());

    let layout = Layout::new(8, 1, false, 0);
    let mut cache = TermCache::default();
    let frame = render_frame(&layout, child.screen(), &mut cache, true);
    assert!(contains(&frame, b"\x1b[7m"));

    let mut host = vt100::Parser::new(1, 8, 0);
    host.process(&frame);
    assert_same_cells(child.screen(), host.screen(), 1, 8);
}
