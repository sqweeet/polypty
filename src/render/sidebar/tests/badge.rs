use super::*;

#[test]
fn ready_is_a_compact_right_aligned_symbol_badge() {
    let base_bg = Color::Rgb {
        r: 48,
        g: 48,
        b: 48,
    };
    let base_fg = Color::Rgb {
        r: 188,
        g: 188,
        b: 188,
    };
    let spans = ready_badge_spans("codex", 18, base_bg, base_fg);
    let text: String = spans.iter().map(|span| span.text.as_str()).collect();
    let badge = spans.last().unwrap();

    assert_eq!(UnicodeWidthStr::width(text.as_str()), 18);
    assert!(text.ends_with(" ✓ "));
    assert_eq!(
        badge.bg,
        Color::Rgb {
            r: 55,
            g: 82,
            b: 65
        }
    );
    assert_eq!(
        badge.fg,
        Color::Rgb {
            r: 184,
            g: 219,
            b: 196
        }
    );

    for (width, expected) in [(1, "✓"), (2, " ✓"), (3, " ✓ ")] {
        let spans = ready_badge_spans("codex", width, base_bg, base_fg);
        let text: String = spans.iter().map(|span| span.text.as_str()).collect();
        assert_eq!(text, expected);
    }
    for width in [1, 2, 3, 10, 18] {
        let spans = ready_badge_spans("codex", width, base_bg, base_fg);
        let text: String = spans.iter().map(|span| span.text.as_str()).collect();
        assert_eq!(UnicodeWidthStr::width(text.as_str()), width);
        assert_eq!(spans.last().unwrap().fg, badge.fg);
        assert_eq!(spans.last().unwrap().bg, badge.bg);
    }
}
