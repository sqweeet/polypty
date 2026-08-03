use super::*;

#[test]
fn glint_spans_preserve_exact_unicode_row_width() {
    for width in [1, 10, 18] {
        let spans = sidebar_paint_spans(
            "claude 👩‍💻",
            width,
            Color::Rgb {
                r: 48,
                g: 48,
                b: 48,
            },
            Color::White,
            Some((true, GlintFrame(7))),
        );
        let text: String = spans.iter().map(|span| span.text.as_str()).collect();
        assert_eq!(UnicodeWidthStr::width(text.as_str()), width);
    }
}
