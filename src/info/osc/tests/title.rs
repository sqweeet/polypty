use super::super::OscTracker;

#[test]
fn titles_are_stateful_sanitized_and_clearable() {
    let mut tracker = OscTracker::default();
    for chunk in "\x1b]0;агент\x1b\\".as_bytes().chunks(2) {
        tracker.feed(chunk);
    }
    assert_eq!(tracker.title.as_deref(), Some("агент"));

    tracker.feed(b"\x1b");
    tracker.feed(b"]2;safe\x01 title\t\x7f\x1b");
    tracker.feed(b"\\");
    assert_eq!(tracker.title.as_deref(), Some("safe title"));

    tracker.feed(b"\x1b]7;file://host/work/tree\x07");
    assert_eq!(tracker.cwd.as_deref(), Some("/work/tree"));
    assert_eq!(tracker.title.as_deref(), Some("safe title"));

    tracker.feed(b"\x1b]2;\x07");
    assert_eq!(tracker.title.as_deref(), Some(""));
    assert_eq!(tracker.cwd.as_deref(), Some("/work/tree"));
}
