use super::*;

#[test]
fn glint_timeline_collapses_the_two_second_rest() {
    assert_eq!(
        GlintFrame::for_elapsed(Duration::from_millis(79)),
        GlintFrame::REST
    );
    assert_eq!(
        GlintFrame::for_elapsed(Duration::from_millis(80)),
        GlintFrame(1)
    );
    assert_eq!(
        GlintFrame::for_elapsed(Duration::from_millis(3_920)),
        GlintFrame::REST
    );
    assert_eq!(
        GlintFrame::for_elapsed(Duration::from_millis(4_000)),
        GlintFrame::REST
    );
    assert_eq!(
        GlintFrame::for_elapsed(Duration::from_millis(5_920)),
        GlintFrame::REST
    );
    assert_eq!(
        GlintFrame::for_elapsed(Duration::from_millis(6_000)),
        GlintFrame::REST
    );
    assert_eq!(
        GlintFrame::for_elapsed(Duration::from_millis(6_080)),
        GlintFrame(1)
    );
}
