use super::*;

fn rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb { r, g, b } => (r, g, b),
        _ => panic!("expected RGB color"),
    }
}

fn working_glint_bg(active: bool, frame: GlintFrame, column: usize, width: usize) -> Color {
    super::working_glint_bg(active, frame, column, width, GlintRow::Flat)
}

#[test]
fn glint_is_neutral_smooth_and_has_a_true_rest() {
    assert_eq!(
        rgb(working_glint_bg(true, GlintFrame::REST, 5, 18)),
        (48, 48, 48)
    );
    assert_eq!(
        rgb(working_glint_bg(false, GlintFrame::REST, 5, 18)),
        (36, 36, 36)
    );
    assert_eq!(
        rgb(working_glint_bg(true, GlintFrame(25), 9, 18)),
        (68, 68, 68)
    );

    for active in [false, true] {
        let max = if active { 68 } else { 50 };
        for frame in 0..50 {
            for column in 0..18 {
                let (r, g, b) = rgb(working_glint_bg(active, GlintFrame(frame), column, 18));
                assert_eq!(r, g);
                assert_eq!(g, b);
                assert!(r <= max);
            }
        }
    }
    for active in [false, true] {
        for column in 0..18 {
            let before = rgb(working_glint_bg(active, GlintFrame(10), column, 18));
            let after = rgb(working_glint_bg(active, GlintFrame(11), column, 18));
            assert!(before.0.abs_diff(after.0) <= 7);
            assert!(before.1.abs_diff(after.1) <= 7);
            assert!(before.2.abs_diff(after.2) <= 7);
        }
    }

    assert_eq!(
        rgb(working_glint_bg(true, GlintFrame(20), 0, 5)),
        (56, 56, 56)
    );
    assert_eq!(
        rgb(working_glint_bg(false, GlintFrame(20), 0, 5)),
        (42, 42, 42)
    );
}
