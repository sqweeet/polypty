use std::time::{Duration, Instant};

use crate::agent::{AgentKind, AgentState};
use crate::tab::agent_tracker::{recent_osc_title, AgentTracker};
use crate::tab::signature::screen_signature;

#[test]
fn osc_agent_title_has_a_bounded_freshness_window() {
    let start = Instant::now();
    assert_eq!(
        recent_osc_title(
            Some("⠋ Codex"),
            Some(start),
            start + crate::agent::ACTIVITY_WINDOW
        ),
        Some("⠋ Codex")
    );
    assert_eq!(
        recent_osc_title(
            Some("⠋ Codex"),
            Some(start),
            start + crate::agent::ACTIVITY_WINDOW + Duration::from_millis(1),
        ),
        None
    );
}

#[test]
fn foreground_change_drops_previous_agent_evidence() {
    let start = Instant::now();
    let parser = vt100::Parser::new(4, 10, 0);
    let mut tracker = AgentTracker::new(start);
    tracker.refresh(Some("codex"), parser.screen(), None, start, true);
    tracker.note_title_update(start);
    tracker.refresh(
        Some("claude"),
        parser.screen(),
        Some("⠋ Codex"),
        start + Duration::from_millis(1),
        true,
    );
    assert_eq!(
        tracker.evidence_snapshot(start + Duration::from_millis(1)),
        (None, Some(screen_signature(parser.screen())), Duration::MAX)
    );
    tracker.refresh(
        Some("claude"),
        parser.screen(),
        Some("⠋ Codex"),
        start + Duration::from_millis(2),
        true,
    );
    assert_eq!(
        tracker.status(),
        Some(crate::agent::AgentStatus::single(
            AgentKind::Claude,
            AgentState::Ready,
        ))
    );
}

#[test]
fn agent_screen_signature_ignores_control_only_output() {
    let mut parser = vt100::Parser::new(4, 10, 0);
    let empty = screen_signature(parser.screen());
    parser.process(b"\x1b]0;Codex\x07\x1b[?25l\x1b[?25h");
    assert_eq!(screen_signature(parser.screen()), empty);
    parser.process(b"working");
    assert_ne!(screen_signature(parser.screen()), empty);
}
