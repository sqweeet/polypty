pub(super) fn draft_text_len(data: &[u8]) -> usize {
    if let Some(paste) = data
        .strip_prefix(b"\x1b[200~")
        .and_then(|data| data.strip_suffix(b"\x1b[201~"))
    {
        return printable_chars(paste);
    }
    if data.starts_with(b"\x1b") {
        return 0;
    }
    printable_chars(data)
}

fn printable_chars(data: &[u8]) -> usize {
    std::str::from_utf8(data).map_or_else(
        |_| {
            data.iter()
                .filter(|byte| **byte >= b' ' && **byte != 0x7f)
                .count()
        },
        |text| text.chars().filter(|ch| !ch.is_control()).count(),
    )
}
