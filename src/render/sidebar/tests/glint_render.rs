use super::*;

#[test]
fn working_glint_covers_both_rows_of_the_card() {
    let layout = Layout::new(40, 12, true, 18);
    let mut tabs = [SidebarTab {
        key: 1,
        primary: "node".into(),
        secondary: "~/projects/polypty".into(),
        agent: Some(AgentStatus::single(AgentKind::Codex, AgentState::Working)),
        glint_frame: Some(GlintFrame(10)),
        active: true,
    }];
    let mut out = Vec::new();
    let mut cache = SidebarCache::default();

    let map = draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
    assert_eq!(map.visible_glints(), &[(1, GlintFrame(10))]);
    let primary = cache.rows[0].clone();
    let secondary = cache.rows[1].clone();
    let primary_text: String = primary
        .spans
        .iter()
        .map(|span| span.text.as_str())
        .collect();
    assert_eq!(primary_text, pad_fit(" codex", 18));

    out.clear();
    draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
    assert!(out.is_empty());

    tabs[0].glint_frame = Some(GlintFrame(20));
    draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
    assert!(!out.is_empty());
    assert_ne!(cache.rows[0], primary);
    assert_ne!(cache.rows[1], secondary);
}

#[test]
fn offscreen_working_tab_does_not_keep_glint_running() {
    let tabs: Vec<SidebarTab> = (0..8)
        .map(|index| SidebarTab {
            key: index as u64,
            primary: format!("tab-{index}"),
            secondary: String::new(),
            agent: (index == 0)
                .then_some(AgentStatus::single(AgentKind::Codex, AgentState::Working)),
            glint_frame: (index == 0).then_some(GlintFrame(10)),
            active: index == 6,
        })
        .collect();
    let layout = Layout::new(40, 5, true, 18);
    let mut out = Vec::new();
    let mut cache = SidebarCache::default();

    let map = draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
    assert!(map.visible_glints().is_empty());
}

#[test]
fn narrow_working_card_is_static_and_does_not_schedule_animation() {
    let tabs = [SidebarTab {
        key: 1,
        primary: "codex".into(),
        secondary: "~/polypty".into(),
        agent: Some(AgentStatus::single(AgentKind::Codex, AgentState::Working)),
        glint_frame: Some(GlintFrame(20)),
        active: true,
    }];
    let layout = Layout::new(25, 12, true, 5);
    let mut out = Vec::new();
    let mut cache = SidebarCache::default();

    let map = draw_sidebar(&mut out, &layout, &tabs, &mut cache, false).unwrap();
    assert!(map.visible_glints().is_empty());
    assert!(cache.rows[0].spans.iter().all(|span| {
        span.bg
            == Color::Rgb {
                r: 56,
                g: 56,
                b: 56,
            }
    }));
}
