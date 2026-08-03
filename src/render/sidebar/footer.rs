pub(super) fn sidebar_footer(width: usize, height: usize) -> Vec<(u8, &'static str)> {
    let full = [
        (4, "shortcuts"),
        (5, "Alt+t new tab"),
        (5, "Alt+w close tab"),
        (5, "Alt+[/] tabs"),
        (5, "Alt+v/s split"),
        (5, "Alt+hjkl pane"),
        (5, "Alt+x close pane"),
        (5, "Alt+b sidebar"),
        (5, "Alt+q quit"),
    ];
    let compact = [
        (4, "keys"),
        (5, "Alt+t new"),
        (5, "Alt+w tab"),
        (5, "Alt+[] tab"),
        (5, "Alt+v/s"),
        (5, "Alt+hjkl"),
        (5, "Alt+x pane"),
        (5, "Alt+b bar"),
        (5, "Alt+q out"),
    ];

    let max_rows = height.saturating_sub(3).min(full.len());
    if max_rows < 2 {
        return Vec::new();
    }
    let rows = if width >= 14 { &full } else { &compact };
    rows.iter().copied().take(max_rows).collect()
}
