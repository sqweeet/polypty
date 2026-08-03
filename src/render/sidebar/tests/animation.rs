use std::time::{Duration, Instant};

use crate::{
    agent::AgentState,
    render::{sidebar::SidebarMap, GlintFrame},
};

use super::SidebarAnimation;

#[test]
fn later_workspaces_keep_an_independent_phase() {
    let start = Instant::now();
    let mut animation = SidebarAnimation::default();
    assert!(animation.reconcile([(10, Some(AgentState::Working))], start));
    let later = start + Duration::from_millis(800);
    assert!(animation.reconcile(
        [
            (10, Some(AgentState::Working)),
            (20, Some(AgentState::Working))
        ],
        later
    ));
    assert_ne!(animation.frame(10, later), animation.frame(20, later));
}

#[test]
fn metadata_keeps_phase_but_a_new_transition_restarts_it() {
    let start = Instant::now();
    let later = start + Duration::from_millis(800);
    let mut animation = SidebarAnimation::default();
    animation.reconcile([(7, Some(AgentState::Working))], start);
    assert!(!animation.reconcile([(7, Some(AgentState::Working))], later));
    assert_eq!(
        animation.frame(7, later),
        Some(GlintFrame::for_elapsed(Duration::from_millis(800)))
    );
    assert!(animation.reconcile([(7, Some(AgentState::Ready))], later));
    assert_eq!(animation.frame(7, later), None);
    assert!(animation.reconcile([(7, Some(AgentState::Working))], later));
    assert_eq!(
        animation.frame(7, later),
        Some(GlintFrame::for_elapsed(Duration::ZERO))
    );
}

#[test]
fn removal_prunes_lifecycle() {
    let start = Instant::now();
    let mut animation = SidebarAnimation::default();
    animation.reconcile(
        [
            (1, Some(AgentState::Working)),
            (2, Some(AgentState::Working)),
        ],
        start,
    );
    assert!(animation.reconcile([(2, Some(AgentState::Working))], start));
    assert_eq!(animation.frame(1, start), None);
    assert!(animation.frame(2, start).is_some());
}

#[test]
fn rest_frames_do_not_schedule_identical_repaints() {
    let start = Instant::now();
    let mut animation = SidebarAnimation::default();
    animation.reconcile([(2, Some(AgentState::Working))], start);
    let rest = GlintFrame::for_elapsed(Duration::from_millis(4_000));
    let map = SidebarMap::with_visible_glints(vec![(2, rest)]);
    assert!(!animation.frame_due(&map, start + Duration::from_millis(5_920)));
    assert!(!animation.frame_due(&map, start + Duration::from_millis(6_000)));
    assert!(animation.frame_due(&map, start + Duration::from_millis(6_080)));
}
