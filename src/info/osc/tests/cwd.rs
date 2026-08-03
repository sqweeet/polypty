use super::super::OscTracker;

#[test]
fn osc7_parses_file_url() {
    let mut tracker = OscTracker::default();
    tracker.feed(b"\x1b]7;file://host/home/gotlib/proj\x07");
    assert_eq!(tracker.cwd.as_deref(), Some("/home/gotlib/proj"));
}

#[test]
fn osc7_decodes_percent_and_raw_utf8() {
    let mut tracker = OscTracker::default();
    tracker.feed(b"\x1b]7;file://host/home/%E7%95%8C%20project\x07");
    assert_eq!(tracker.cwd.as_deref(), Some("/home/界 project"));

    for chunk in "\x1b]7;file:///tmp/שלום\x1b\\".as_bytes().chunks(3) {
        tracker.feed(chunk);
    }
    assert_eq!(tracker.cwd.as_deref(), Some("/tmp/שלום"));
    tracker.feed(b"\x1b]7;file:///%FF\x07");
    assert_eq!(tracker.cwd.as_deref(), Some("/tmp/שלום"));
}
