use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(super) fn wrap_text(s: &str, width: usize, max_lines: usize) -> Vec<String> {
    if width == 0 || max_lines == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;
    let mut truncated = false;

    for raw in UnicodeSegmentation::graphemes(s, true) {
        let grapheme: String = raw
            .chars()
            .filter(|character| !character.is_control())
            .collect();
        if grapheme.is_empty() {
            continue;
        }
        let grapheme_width = UnicodeWidthStr::width(grapheme.as_str());
        if grapheme_width == 0 {
            if !current.is_empty() {
                current.push_str(&grapheme);
            }
            continue;
        }
        if grapheme_width > width {
            truncated = true;
            continue;
        }
        if current_width + grapheme_width > width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            current_width = 0;
            if lines.len() >= max_lines {
                truncated = true;
                break;
            }
        }
        current.push_str(&grapheme);
        current_width += grapheme_width;
    }
    if lines.len() < max_lines && !current.is_empty() {
        lines.push(current);
    }

    if truncated && !lines.is_empty() {
        let last = lines.last_mut().expect("non-empty lines");
        while UnicodeWidthStr::width(last.as_str()) + 1 > width && !last.is_empty() {
            let start = UnicodeSegmentation::grapheme_indices(last.as_str(), true)
                .next_back()
                .map(|(index, _)| index)
                .unwrap_or(0);
            last.truncate(start);
        }
        if UnicodeWidthStr::width(last.as_str()) < width {
            last.push('…');
        }
    }
    lines
}

pub(super) fn pad_fit(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut output = String::new();
    let mut output_width = 0;
    for raw in UnicodeSegmentation::graphemes(s, true) {
        let grapheme: String = raw
            .chars()
            .filter(|character| !character.is_control())
            .collect();
        if grapheme.is_empty() {
            continue;
        }
        let grapheme_width = UnicodeWidthStr::width(grapheme.as_str());
        if grapheme_width == 0 && output.is_empty() {
            continue;
        }
        if output_width + grapheme_width > width {
            break;
        }
        output.push_str(&grapheme);
        output_width += grapheme_width;
    }
    while output_width < width {
        output.push(' ');
        output_width += 1;
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_text_preserves_combining_graphemes_without_false_ellipsis() {
        let title = "e\u{301}";
        assert_eq!(wrap_text(title, 4, 1), vec![title.to_string()]);
        assert_eq!(pad_fit(title, 4), format!("{title}   "));
        assert_eq!(UnicodeWidthStr::width(pad_fit(title, 4).as_str()), 4);
    }

    #[test]
    fn sidebar_text_keeps_zwj_emoji_atomic() {
        let emoji = "👩‍💻";
        let title = format!("{emoji}x");
        assert_eq!(UnicodeWidthStr::width(emoji), 2);
        assert_eq!(wrap_text(&title, 3, 1), vec![title]);
        assert_eq!(pad_fit(emoji, 4), format!("{emoji}  "));
        assert_eq!(UnicodeWidthStr::width(pad_fit(emoji, 4).as_str()), 4);
    }
}
