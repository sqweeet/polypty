use super::*;

#[test]
fn sidebar_footer_is_anchored_and_not_clickable() {
    let layout = Layout::new(40, 12, true, 18);
    let tabs = [SidebarTab {
        key: 1,
        primary: "shell".into(),
        secondary: "~/projects/mux".into(),
        agent: None,
        glint_frame: None,
        active: true,
    }];
    let mut out = Vec::new();
    let mut cache = SidebarCache::default();

    let map = draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
    let footer_rows = sidebar_footer(18, 12).len();
    let footer_start = 12 - footer_rows;
    let rendered = String::from_utf8_lossy(&out);

    assert!(map.row_tab[0].is_some());
    assert!(map.row_tab[footer_start..].iter().all(Option::is_none));
    assert!(rendered.contains(" shell"));
    assert!(rendered.contains(" ~/projects/mux"));
    assert!(!rendered.contains("Tabs"));
    assert!(!rendered.contains("Shortcuts"));
    assert!(rendered.contains("Alt+t new tab"));
    assert!(rendered.contains("Alt+[/] tabs"));
    assert!(rendered.contains("Alt+v/s split"));
    assert!(rendered.contains("Alt+hjkl pane"));
    assert!(rendered.contains("Alt+q quit"));

    out.clear();
    draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
    assert!(out.is_empty(), "unchanged sidebar should emit no bytes");
}

#[test]
fn sidebar_viewport_keeps_active_tab_visible_when_shrinking() {
    let tabs: Vec<SidebarTab> = (0..8)
        .map(|index| SidebarTab {
            key: index as u64,
            primary: format!("agent-{index}"),
            secondary: if index == 6 {
                "~/projects/mux".to_string()
            } else {
                String::new()
            },
            agent: None,
            glint_frame: None,
            active: index == 6,
        })
        .collect();
    let mut cache = SidebarCache::default();

    for rows in [12, 7, 5, 2, 1] {
        let layout = Layout::new(40, rows, true, 18);
        let mut out = Vec::new();
        let map = draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
        assert!(
            map.row_tab.contains(&Some(6)),
            "active tab disappeared at {rows} rows: {:?}",
            map.row_tab
        );
        assert!(map
            .row_tab
            .iter()
            .flatten()
            .all(|tab_index| *tab_index < tabs.len()));
    }

    let one_row = Layout::new(40, 1, true, 18);
    let mut out = Vec::new();
    let map = draw_sidebar(&mut out, &one_row, &tabs, &mut cache, true).unwrap();
    assert_eq!(map.row_tab, vec![Some(6)]);
    assert!(String::from_utf8_lossy(&out).contains("agent-6"));
}

#[test]
fn sidebar_footer_reflects_configured_shortcuts() {
    let shortcuts = SidebarShortcuts {
        new_tab: Some("Ctrl+n".into()),
        quit: None,
        ..SidebarShortcuts::default()
    };
    let text = configured_footer(18, 12, &shortcuts)
        .into_iter()
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(text.contains("Ctrl+n new tab"));
    assert!(text.contains("— quit"));
}

#[test]
fn sidebar_footer_can_be_hidden() {
    let shortcuts = SidebarShortcuts {
        visible: false,
        ..SidebarShortcuts::default()
    };
    assert!(configured_footer(18, 12, &shortcuts).is_empty());
}
