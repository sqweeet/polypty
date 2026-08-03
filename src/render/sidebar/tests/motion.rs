use std::time::{Duration, Instant};

use super::*;
use crate::render::sidebar::{
    model::SidebarContentRow,
    palette::SidebarPalette,
    tab_motion::{TabMotion, TabVisual},
};

#[test]
fn active_tab_crossfades_and_commits_the_final_frame() {
    let start = Instant::now();
    let mut tabs = tabs();
    let mut motion = TabMotion::default();
    motion.reconcile(&tabs, start);
    assert_eq!(motion.visual(1, true, start).active, 255);
    motion.mark_frame(&[1, 2], start);

    tabs[0].active = false;
    tabs[1].active = true;
    motion.reconcile(&tabs, start);
    assert_eq!(motion.visual(1, false, start).active, 255);
    assert_eq!(motion.visual(2, true, start).active, 0);
    assert!(motion.frame_due(&[1, 2], start));

    let settled = start + Duration::from_millis(200);
    assert_eq!(motion.visual(1, false, settled).active, 0);
    assert_eq!(motion.visual(2, true, settled).active, 255);
    motion.mark_frame(&[1, 2], settled);
    assert!(!motion.frame_due(&[1, 2], settled));
}

#[test]
fn pointer_feedback_starts_visible_and_dragging_out_cancels_release() {
    let start = Instant::now();
    let tabs = tabs();
    let mut motion = TabMotion::default();
    motion.reconcile(&tabs, start);

    assert!(motion.set_hover(Some(2), start));
    assert_eq!(motion.visual(2, false, start).hover, 32);
    assert!(motion.begin_press(2, start));
    assert_eq!(motion.visual(2, false, start).press, 96);

    motion.update_press(None, start + Duration::from_millis(10));
    assert_eq!(
        motion.release(None, start + Duration::from_millis(20)),
        None
    );
    assert!(!motion.press_active());
}

#[test]
fn tab_palette_blends_alpha_without_changing_card_geometry() {
    let card = build_cards(&tabs(), 18).remove(1);
    let mut row = SidebarContentRow::card(&card, 2, "two", GlintRow::Flat);
    let palette = SidebarPalette::new();

    assert_eq!(background(&palette, &row), rgb(36));
    row.visual = TabVisual {
        hover: 128,
        ..TabVisual::default()
    };
    assert_eq!(background(&palette, &row), rgb(46));
    row.visual = TabVisual {
        press: 255,
        ..TabVisual::default()
    };
    assert_eq!(background(&palette, &row), rgb(68));
}

fn background(palette: &SidebarPalette, row: &SidebarContentRow) -> Color {
    palette.paint(row, 18).spans[0].bg
}

fn rgb(value: u8) -> Color {
    Color::Rgb {
        r: value,
        g: value,
        b: value,
    }
}

fn tabs() -> [SidebarTab; 2] {
    [
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
    ]
}
