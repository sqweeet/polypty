use std::time::{Duration, Instant};

use crate::agent::{AgentInteraction, AgentObservation, AgentState};

use super::activity_text::draft_text_len;

const INPUT_REACTION_GRACE: Duration = Duration::from_millis(700);
const RESIZE_REACTION_GRACE: Duration = Duration::from_millis(700);
const SUBMIT_REACTION_GRACE: Duration = Duration::from_millis(1_200);

/// Separates independent child output from local-input and resize reactions.
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
    pub fn note_input(&mut self, data: &[u8], now: Instant) {
        if matches!(data, b"\r" | b"\n") {
            self.note_enter(now);
            return;
        }
        self.last_unattributed_output = None;
        self.edit_until = Some(now + INPUT_REACTION_GRACE);
        self.submit_until = None;
        self.submit_observed_output = false;
        match data {
            b"\x03" | b"\x15" | b"\x17" => self.draft_chars = 0,
            b"\x08" | b"\x7f" | b"\x1b[3~" => {
                self.draft_chars = self.draft_chars.saturating_sub(1);
            }
            _ => self.draft_chars = self.draft_chars.saturating_add(draft_text_len(data)),
        }
    }

    fn note_enter(&mut self, now: Instant) {
        self.last_unattributed_output = None;
        if self.draft_chars > 0 {
            self.edit_until = None;
            self.submit_until = Some(now + SUBMIT_REACTION_GRACE);
        } else {
            self.edit_until = Some(now + INPUT_REACTION_GRACE);
            self.submit_until = None;
        }
        self.submit_observed_output = false;
        self.draft_chars = 0;
    }

    pub fn note_resize(&mut self, now: Instant) {
        self.last_unattributed_output = None;
        self.resize_until = Some(now + RESIZE_REACTION_GRACE);
    }

    pub fn note_output(&mut self, now: Instant, screen_changed: bool) {
        if !screen_changed
            || self.deadline_active(self.resize_until, now)
            || self.deadline_active(self.edit_until, now)
        {
            return;
        }
        if self.deadline_active(self.submit_until, now) {
            self.submit_observed_output = true;
        }
        self.last_unattributed_output = Some(now);
    }

    pub fn observation(&self, previous: AgentState, now: Instant) -> AgentObservation {
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

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn deadline_active(&self, deadline: Option<Instant>, now: Instant) -> bool {
        deadline.is_some_and(|deadline| now <= deadline)
    }
}
