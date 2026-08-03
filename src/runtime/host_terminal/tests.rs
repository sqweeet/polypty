use super::write_restore;

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[test]
fn restore_resets_every_output_mode() {
    let mut output = Vec::new();
    write_restore(&mut output).unwrap();

    for sequence in [
        b"\x1b[?7h".as_slice(),
        b"\x1b[?2026l",
        b"\x1b[?1000l",
        b"\x1b[?1002l",
        b"\x1b[?1003l",
        b"\x1b[?1006l",
        b"\x1b[?2004l",
        b"\x1b[?25h",
        b"\x1b[?1049l",
    ] {
        assert!(contains(&output, sequence), "missing {sequence:?}");
    }
}
