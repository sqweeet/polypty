use std::time::{Duration, Instant};

use crate::agent::{AgentInteraction, AgentState};
use crate::tab::activity::AgentActivity;

#[test]
fn echoed_input_never_turns_into_agent_activity_later() {
    let start = Instant::now();
    let mut activity = AgentActivity::default();
    activity.note_input(b"x", start);
    activity.note_output(start + Duration::from_millis(20), true);
    assert_eq!(
        activity
            .observation(AgentState::Ready, start + Duration::from_secs(2))
            .quiet_for,
        Duration::MAX
    );
}

#[test]
fn resize_repaint_is_ignored_but_later_output_is_counted() {
    let start = Instant::now();
    let mut activity = AgentActivity::default();
    activity.note_resize(start);
    activity.note_output(start + Duration::from_millis(200), true);
    assert_eq!(
        activity
            .observation(AgentState::Ready, start + Duration::from_secs(1))
            .quiet_for,
        Duration::MAX
    );
    activity.note_output(start + Duration::from_millis(800), true);
    assert_eq!(
        activity
            .observation(AgentState::Ready, start + Duration::from_millis(950))
            .quiet_for,
        Duration::from_millis(150)
    );
}

#[test]
fn local_redraw_invalidates_prior_activity() {
    let start = Instant::now();
    let mut activity = AgentActivity::default();
    activity.note_output(start, true);
    activity.note_input(b"x", start + Duration::from_millis(100));
    activity.note_output(start + Duration::from_millis(120), true);
    assert_eq!(
        activity
            .observation(AgentState::Ready, start + Duration::from_millis(500))
            .quiet_for,
        Duration::MAX
    );
}

#[test]
fn overlapping_local_interactions_extend_the_grace() {
    let start = Instant::now();
    let mut activity = AgentActivity::default();
    activity.note_input(b"x", start);
    activity.note_resize(start + Duration::from_millis(500));
    activity.note_output(start + Duration::from_millis(800), true);
    assert_eq!(
        activity
            .observation(AgentState::Ready, start + Duration::from_secs(2))
            .quiet_for,
        Duration::MAX
    );
}

#[test]
fn reset_drops_activity_from_the_previous_process() {
    let start = Instant::now();
    let mut activity = AgentActivity::default();
    activity.note_output(start, true);
    activity.reset();
    assert_eq!(
        activity
            .observation(AgentState::Ready, start + Duration::from_millis(10))
            .quiet_for,
        Duration::MAX
    );
}

#[test]
fn control_only_output_is_not_activity_or_submit_acknowledgement() {
    let start = Instant::now();
    let mut activity = AgentActivity::default();
    activity.note_input(b"x", start);
    activity.note_input(b"\r", start + Duration::from_millis(10));
    activity.note_output(start + Duration::from_millis(20), false);
    let observation = activity.observation(AgentState::Ready, start + Duration::from_millis(30));
    assert_eq!(observation.interaction, AgentInteraction::SubmitPending);
    assert_eq!(observation.quiet_for, Duration::MAX);
}
