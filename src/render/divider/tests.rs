use super::*;

#[test]
fn nested_dividers_render_connected_tees() {
    let mut out = Vec::new();
    draw_dividers(
        &mut out,
        &[
            Divider::Vertical { x: 4, y: 0, len: 4 },
            Divider::Horizontal { x: 5, y: 2, len: 6 },
        ],
    )
    .unwrap();

    let mut host = vt100::Parser::new(4, 11, 0);
    host.process(&out);
    assert_eq!(host.screen().cell(2, 4).unwrap().contents(), "├");
    assert_eq!(host.screen().cell(2, 5).unwrap().contents(), "─");
    assert_eq!(host.screen().cell(2, 10).unwrap().contents(), "─");
    assert_eq!(host.screen().cell(0, 4).unwrap().contents(), "│");
}
