use std::time::{Duration, Instant};

use crate::agent::{AgentInteraction, AgentObservation, AgentState};

/// A TUI redraw caused by local input is not evidence that the child started
/// doing work. Keep the grace long enough to cover an echoed paste/redraw
/// burst; a fresh strong OSC title can still report Working immediately.
const INPUT_REACTION_GRACE: Duration = Duration::from_millis(700);
/// SIGWINCH commonly makes a full-screen TUI repaint every cell. Those bytes
/// describe the same state at a new geometry, so they must not start a glint.
const RESIZE_REACTION_GRACE: Duration = Duration::from_millis(700);
/// A submission is an edge, not a permanent working state. If the child does
/// not acknowledge it with output within this window, preserve the old state.
const SUBMIT_REACTION_GRACE: Duration = Duration::from_millis(1_200);

/// Separates unattributed child output from output causally following a local
/// interaction. Ignored output never becomes activity later when the grace
/// expires; only a subsequent independent output burst can move the clock.
#[derive(Debug, Default)]
pub(super) struct AgentActivity {
    last_unattributed_output: Option<Instant>,
    edit_until: Option<Instant>,
    resize_until: Option<Instant>,
    submit_until: Option<Instant>,
    submit_observed_output: bool,
    draft_chars: usize,
}

impl AgentActivity {
    pub(super) fn note_input(&mut self, data: &[u8], now: Instant) {
        // Key encoding sends unmodified Enter as exactly CR (LF is accepted
        // for portability). Newlines inside a paste are editing, not a submit
        // edge, because the whole paste arrives in one larger message.
        if matches!(data, b"\r" | b"\n") {
            self.last_unattributed_output = None;
            if self.draft_chars > 0 {
                self.edit_until = None;
                self.submit_until = Some(now + SUBMIT_REACTION_GRACE);
            } else {
                // Empty Enter is just another local redraw. It cannot start a
                // best-effort Working transition by itself.
                self.edit_until = Some(now + INPUT_REACTION_GRACE);
                self.submit_until = None;
            }
            self.submit_observed_output = false;
            self.draft_chars = 0;
        } else {
            self.last_unattributed_output = None;
            self.edit_until = Some(now + INPUT_REACTION_GRACE);
            self.submit_until = None;
            self.submit_observed_output = false;
            match data {
                // Ctrl+C cancels, Ctrl+U clears the line, Ctrl+W removes at
                // least the current word. Conservatively disarm submit rather
                // than animating an input that may now be empty.
                b"\x03" | b"\x15" | b"\x17" => self.draft_chars = 0,
                b"\x08" | b"\x7f" | b"\x1b[3~" => {
                    self.draft_chars = self.draft_chars.saturating_sub(1);
                }
                _ => {
                    self.draft_chars = self.draft_chars.saturating_add(draft_text_len(data));
                }
            }
        }
    }

    pub(super) fn note_resize(&mut self, now: Instant) {
        self.last_unattributed_output = None;
        self.resize_until = Some(now + RESIZE_REACTION_GRACE);
    }

    pub(super) fn note_output(&mut self, now: Instant, screen_changed: bool) {
        // OSC, terminal queries, cursor visibility and a byte-for-byte redraw
        // are transport traffic, not evidence that an agent is doing work.
        if !screen_changed {
            return;
        }
        if self.deadline_active(self.resize_until, now)
            || self.deadline_active(self.edit_until, now)
        {
            return;
        }
        if self.deadline_active(self.submit_until, now) {
            self.submit_observed_output = true;
        }
        self.last_unattributed_output = Some(now);
    }

    pub(super) fn observation(&self, previous: AgentState, now: Instant) -> AgentObservation {
        let interaction = if self.deadline_active(self.resize_until, now) {
            AgentInteraction::Resizing
        } else if self.deadline_active(self.edit_until, now) {
            AgentInteraction::Editing
        } else if self.deadline_active(self.submit_until, now) {
            if self.submit_observed_output {
                AgentInteraction::Submitted
            } else {
                AgentInteraction::SubmitPending
            }
        } else {
            AgentInteraction::None
        };
        let quiet_for = self
            .last_unattributed_output
            .map(|last| now.saturating_duration_since(last))
            .unwrap_or(Duration::MAX);
        AgentObservation {
            previous,
            quiet_for,
            interaction,
        }
    }

    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    fn deadline_active(&self, deadline: Option<Instant>, now: Instant) -> bool {
        deadline.is_some_and(|deadline| now <= deadline)
    }
}

fn draft_text_len(data: &[u8]) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn exact_enter_requires_child_output_before_becoming_submitted() {
        let start = Instant::now();
        let mut activity = AgentActivity::default();

        activity.note_input(b"x", start);
        activity.note_input(b"\r", start + Duration::from_millis(10));
        assert_eq!(
            activity
                .observation(AgentState::Ready, start + Duration::from_millis(20))
                .interaction,
            AgentInteraction::SubmitPending
        );

        activity.note_output(start + Duration::from_millis(40), true);
        assert_eq!(
            activity
                .observation(AgentState::Ready, start + Duration::from_millis(50))
                .interaction,
            AgentInteraction::Submitted
        );
    }

    #[test]
    fn newline_inside_a_paste_is_only_editing() {
        let start = Instant::now();
        let mut activity = AgentActivity::default();
        activity.note_input(b"first\nsecond", start);

        assert_eq!(
            activity
                .observation(AgentState::Ready, start + Duration::from_millis(20))
                .interaction,
            AgentInteraction::Editing
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

        let observation =
            activity.observation(AgentState::Ready, start + Duration::from_millis(30));
        assert_eq!(observation.interaction, AgentInteraction::SubmitPending);
        assert_eq!(observation.quiet_for, Duration::MAX);
    }

    #[test]
    fn empty_enter_is_an_edit_redraw_not_a_submission() {
        let start = Instant::now();
        let mut activity = AgentActivity::default();
        activity.note_input(b"\r", start);
        activity.note_output(start + Duration::from_millis(10), true);

        let observation =
            activity.observation(AgentState::Ready, start + Duration::from_millis(20));
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

        assert_eq!(
            activity
                .observation(AgentState::Ready, start + Duration::from_millis(40))
                .interaction,
            AgentInteraction::Editing
        );
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
                activity
                    .observation(AgentState::Ready, start + Duration::from_millis(40))
                    .interaction,
                AgentInteraction::Editing,
                "erase sequence {erase:?} left submit armed",
            );
        }
    }
}
