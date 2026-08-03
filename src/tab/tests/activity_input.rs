use std::time::{Duration, Instant};

use crate::agent::{AgentInteraction, AgentState};
use crate::tab::activity::AgentActivity;

fn interaction(activity: &AgentActivity, start: Instant, millis: u64) -> AgentInteraction {
    activity
        .observation(AgentState::Ready, start + Duration::from_millis(millis))
        .interaction
}

#[test]
fn exact_enter_requires_child_output_before_becoming_submitted() {
    let start = Instant::now();
    let mut activity = AgentActivity::default();
    activity.note_input(b"x", start);
    activity.note_input(b"\r", start + Duration::from_millis(10));
    assert_eq!(
        interaction(&activity, start, 20),
        AgentInteraction::SubmitPending
    );
    activity.note_output(start + Duration::from_millis(40), true);
    assert_eq!(
        interaction(&activity, start, 50),
        AgentInteraction::Submitted
    );
}

#[test]
fn newline_inside_a_paste_is_only_editing() {
    let start = Instant::now();
    let mut activity = AgentActivity::default();
    activity.note_input(b"first\nsecond", start);
    assert_eq!(interaction(&activity, start, 20), AgentInteraction::Editing);
}

#[test]
fn empty_enter_is_an_edit_redraw_not_a_submission() {
    let start = Instant::now();
    let mut activity = AgentActivity::default();
    activity.note_input(b"\r", start);
    activity.note_output(start + Duration::from_millis(10), true);
    let observation = activity.observation(AgentState::Ready, start + Duration::from_millis(20));
    assert_eq!(observation.interaction, AgentInteraction::Editing);
    assert_eq!(observation.quiet_for, Duration::MAX);
}

#[test]
fn clearing_a_draft_disarms_submit() {
    let start = Instant::now();
    let mut activity = AgentActivity::default();
    activity.note_input(b"draft", start);
    activity.note_input(b"\x15", start + Duration::from_millis(10));
    activity.note_input(b"\r", start + Duration::from_millis(20));
    activity.note_output(start + Duration::from_millis(30), true);
    assert_eq!(interaction(&activity, start, 40), AgentInteraction::Editing);
}

#[test]
fn erasing_the_last_draft_character_disarms_submit() {
    let start = Instant::now();
    for erase in [b"\x08".as_slice(), b"\x7f", b"\x1b[3~", b"\x17", b"\x03"] {
        let mut activity = AgentActivity::default();
        activity.note_input(b"x", start);
        activity.note_input(erase, start + Duration::from_millis(10));
        activity.note_input(b"\r", start + Duration::from_millis(20));
        activity.note_output(start + Duration::from_millis(30), true);
        assert_eq!(
            interaction(&activity, start, 40),
            AgentInteraction::Editing,
            "erase sequence {erase:?} left submit armed",
        );
    }
}
