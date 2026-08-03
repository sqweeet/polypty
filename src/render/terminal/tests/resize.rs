use super::*;

#[test]
fn preserved_resize_diff_matches_a_fresh_full_frame() {
    let mut child = vt100::Parser::new(3, 5, 0);
    child.process(b"abc\r\n\x1b[31mxy\x1b[0m");

    let initial = Layout::new(5, 3, false, 0);
    let mut cache = TermCache::default();
    let mut host = vt100::Parser::new(3, 5, 0);
    host.process(&render_frame(&initial, child.screen(), &mut cache, true));

    let grown = Layout::new(8, 5, false, 0);
    host.screen_mut().set_size(5, 8);
    host.process(&render_frame(&grown, child.screen(), &mut cache, false));

    let mut fresh_cache = TermCache::default();
    let mut fresh_host = vt100::Parser::new(5, 8, 0);
    fresh_host.process(&render_frame(
        &grown,
        child.screen(),
        &mut fresh_cache,
        true,
    ));
    assert_same_cells(host.screen(), fresh_host.screen(), 5, 8);

    let shrunk = Layout::new(4, 2, false, 0);
    host.screen_mut().set_size(2, 4);
    host.process(&render_frame(&shrunk, child.screen(), &mut cache, false));

    let mut fresh_cache = TermCache::default();
    let mut fresh_host = vt100::Parser::new(2, 4, 0);
    fresh_host.process(&render_frame(
        &shrunk,
        child.screen(),
        &mut fresh_cache,
        true,
    ));
    assert_same_cells(host.screen(), fresh_host.screen(), 2, 4);
}
