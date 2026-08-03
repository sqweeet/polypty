use super::super::color::PackedColor;
use super::*;

#[test]
fn agent_frame_survives_partial_redraw_and_resize() {
    let mut child = vt100::Parser::new(5, 20, 0);
    child.process(
        concat!(
            "\x1b[?1049h\x1b[2J\x1b[H",
            "\x1b[48;2;18;20;24m\x1b[38;2;120;200;255m agent ☰X ",
            "\x1b[2;1H\x1b[48;2;24;26;30m\x1b[38;2;130;220;160mstatus: ",
            "\x1b[1mRUN\x1b[22m",
            "\x1b[3;1H\x1b[0mwide 界 combine e\u{301}",
            "\x1b[4;1Hpartial: old",
            "\x1b[4;10H\x1b[?25l"
        )
        .as_bytes(),
    );
    assert!(child.screen().alternate_screen());

    let layout = Layout::new(20, 5, false, 0);
    let mut cache = TermCache::default();
    let mut host = vt100::Parser::new(5, 20, 0);
    let initial = render_frame(&layout, child.screen(), &mut cache, true);
    assert!(contains(&initial, b"\x1b[38;2;120;200;255m"));
    assert!(contains(&initial, b"\x1b[48;2;18;20;24m"));
    host.process(&initial);
    assert_same_cells(child.screen(), host.screen(), 5, 20);
    assert!(host.screen().hide_cursor());
    assert_eq!(
        cell_to_paint(child.screen().cell(0, 1)).fg,
        PackedColor::from_vt(vt100::Color::Rgb(120, 200, 255))
    );

    child.process(b"\x1b[4;10H\x1b[38;2;255;170;70mNEW\x1b[0m\x1b[5;7H\x1b[?25h");
    let delta = render_frame(&layout, child.screen(), &mut cache, false);
    assert!(contains(&delta, b"NEW"));
    assert!(!contains(&delta, "agent ☰X".as_bytes()));
    host.process(&delta);
    assert_same_cells(child.screen(), host.screen(), 5, 20);
    assert_eq!(
        host.screen().cursor_position(),
        child.screen().cursor_position()
    );
    assert!(!host.screen().hide_cursor());

    child.screen_mut().set_size(4, 14);
    host.screen_mut().set_size(4, 14);
    let shrunk = Layout::new(14, 4, false, 0);
    host.process(&render_frame(&shrunk, child.screen(), &mut cache, false));
    let mut fresh_cache = TermCache::default();
    let mut fresh_host = vt100::Parser::new(4, 14, 0);
    fresh_host.process(&render_frame(
        &shrunk,
        child.screen(),
        &mut fresh_cache,
        true,
    ));
    assert_same_cells(host.screen(), fresh_host.screen(), 4, 14);

    child.screen_mut().set_size(6, 22);
    child.process(b"\x1b[6;18Hedge");
    host.screen_mut().set_size(6, 22);
    let grown = Layout::new(22, 6, false, 0);
    host.process(&render_frame(&grown, child.screen(), &mut cache, false));
    let mut fresh_cache = TermCache::default();
    let mut fresh_host = vt100::Parser::new(6, 22, 0);
    fresh_host.process(&render_frame(
        &grown,
        child.screen(),
        &mut fresh_cache,
        true,
    ));
    assert_same_cells(host.screen(), fresh_host.screen(), 6, 22);
    assert_eq!(
        host.screen().cursor_position(),
        child.screen().cursor_position()
    );
}
