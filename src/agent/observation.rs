use std::time::Duration;

use super::AgentState;

/// What locally initiated the latest PTY reaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentInteraction {
    None,
    Editing,
    SubmitPending,
    Submitted,
    Resizing,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AgentObservation {
    pub previous: AgentState,
    pub quiet_for: Duration,
    pub interaction: AgentInteraction,
}
