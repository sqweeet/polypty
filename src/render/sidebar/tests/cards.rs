use super::*;

#[test]
fn tab_cards_have_no_blank_rows_between_them() {
    let tabs = [
        SidebarTab {
            key: 1,
            primary: "one".into(),
            secondary: String::new(),
            agent: None,
            glint_frame: None,
            active: true,
        },
        SidebarTab {
            key: 2,
            primary: "two".into(),
            secondary: String::new(),
            agent: None,
            glint_frame: None,
            active: false,
        },
    ];

    let cards = build_cards(&tabs, 18);
    assert!(cards.iter().all(|card| card.lines.len() == 1));
    assert_eq!(cards[0].lines[0].1, "one");
    assert!(cards
        .iter()
        .flat_map(|card| &card.lines)
        .all(|(kind, _)| *kind != 0));
    assert_eq!(sidebar_footer(18, 12)[0].1, "Alt+t new tab");

    let long = [SidebarTab {
        key: 1,
        primary: "a very long process title".into(),
        secondary: "~/projects/mux".into(),
        agent: None,
        glint_frame: None,
        active: true,
    }];
    let cards = build_cards(&long, 8);
    assert_eq!(cards[0].lines.len(), 2);
    assert_eq!(cards[0].lines[0].0, 2);
    assert_eq!(cards[0].lines[1].0, 3);
}
