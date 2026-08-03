use crate::agent::{
    catalog::AgentCatalog, AgentInteraction, AgentKind, AgentObservation, AgentState,
    ACTIVITY_WINDOW,
};

use super::{screen::ScreenEvidence, title::state_from_title};

struct AgentDetector<'a> {
    kind: AgentKind,
    screen: &'a vt100::Screen,
    title: &'a str,
    observation: AgentObservation,
}

impl<'a> AgentDetector<'a> {
    fn new(
        kind: AgentKind,
        screen: &'a vt100::Screen,
        title: Option<&'a str>,
        observation: AgentObservation,
    ) -> Self {
        Self {
            kind,
            screen,
            title: title.unwrap_or_default().trim(),
            observation,
        }
    }

    fn detect(self) -> AgentState {
        let title_state = state_from_title(self.kind, self.title);
        if title_state == Some(AgentState::Blocked) {
            return AgentState::Blocked;
        }
        if matches!(
            self.observation.interaction,
            AgentInteraction::Resizing | AgentInteraction::Editing
        ) {
            return title_state.unwrap_or(self.observation.previous);
        }

        let activity_is_recent = self.observation.quiet_for <= ACTIVITY_WINDOW;
        let evidence = ScreenEvidence::capture(self.kind, self.screen);
        let screen_state = self.reduce_screen(&evidence, activity_is_recent);
        // A permission menu can retain Claude's last ready/working title. The
        // visible request is newer and must still surface as action required.
        if screen_state == Some(AgentState::Blocked) {
            return AgentState::Blocked;
        }
        if let Some(state) = title_state {
            return state;
        }

        match self.observation.interaction {
            AgentInteraction::SubmitPending => screen_state.unwrap_or(self.observation.previous),
            AgentInteraction::Submitted => screen_state.unwrap_or(AgentState::Working),
            AgentInteraction::None => {
                screen_state.unwrap_or_else(|| self.activity_fallback(activity_is_recent))
            }
            AgentInteraction::Editing | AgentInteraction::Resizing => unreachable!(),
        }
    }

    fn reduce_screen(
        &self,
        evidence: &ScreenEvidence,
        activity_is_recent: bool,
    ) -> Option<AgentState> {
        if evidence.blocked {
            Some(AgentState::Blocked)
        } else if evidence.working
            && (!evidence.ready
                || self.observation.interaction == AgentInteraction::Submitted
                || (self.observation.previous == AgentState::Working && activity_is_recent))
        {
            Some(AgentState::Working)
        } else if evidence.ready {
            Some(AgentState::Ready)
        } else {
            None
        }
    }

    fn activity_fallback(&self, activity_is_recent: bool) -> AgentState {
        let explicit = AgentCatalog::profile(self.kind).explicit_screen_state;
        if activity_is_recent && (!explicit || self.observation.previous == AgentState::Working) {
            AgentState::Working
        } else {
            AgentState::Ready
        }
    }
}

pub(crate) fn detect_state(
    kind: AgentKind,
    screen: &vt100::Screen,
    fresh_osc_title: Option<&str>,
    observation: AgentObservation,
) -> AgentState {
    AgentDetector::new(kind, screen, fresh_osc_title, observation).detect()
}
