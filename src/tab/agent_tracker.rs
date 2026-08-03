use std::time::{Duration, Instant};

use crate::agent::{self, AgentKind, AgentState, AgentStatus};

use super::{activity::AgentActivity, signature::screen_signature, Tab};

pub(super) struct AgentTracker {
    kind: Option<AgentKind>,
    state: Option<AgentState>,
    title_updated_at: Option<Instant>,
    activity: AgentActivity,
    screen_signature: Option<u64>,
    last_scan: Instant,
}

impl AgentTracker {
    pub fn new(now: Instant) -> Self {
        Self {
            kind: None,
            state: None,
            title_updated_at: None,
            activity: AgentActivity::default(),
            screen_signature: None,
            last_scan: now - Duration::from_secs(10),
        }
    }

    pub fn note_input(&mut self, data: &[u8], now: Instant) {
        self.activity.note_input(data, now);
    }

    pub fn note_resize(&mut self, screen: &vt100::Screen, now: Instant) {
        self.activity.note_resize(now);
        self.screen_signature = self.kind.map(|_| screen_signature(screen));
    }

    pub fn note_title_update(&mut self, now: Instant) {
        self.title_updated_at = Some(now);
    }

    pub fn note_output(&mut self, screen: &vt100::Screen, now: Instant) {
        let current = self.kind.map(|_| screen_signature(screen));
        let changed = self
            .screen_signature
            .zip(current)
            .is_some_and(|(before, after)| before != after);
        self.screen_signature = current;
        self.activity.note_output(now, changed);
    }

    pub fn refresh(
        &mut self,
        process: Option<&str>,
        screen: &vt100::Screen,
        osc_title: Option<&str>,
        now: Instant,
        force: bool,
    ) -> bool {
        if !force && now.duration_since(self.last_scan) < agent::SCAN_INTERVAL {
            return false;
        }
        self.last_scan = now;
        let next_kind = process.and_then(agent::identify_name);
        if next_kind != self.kind {
            self.kind = next_kind;
            self.state = next_kind.map(|_| AgentState::Ready);
            self.reset_evidence(next_kind.map(|_| screen_signature(screen)));
            return true;
        }

        let next_state = next_kind.map(|kind| {
            let title = recent_osc_title(osc_title, self.title_updated_at, now);
            let previous = self.state.unwrap_or(AgentState::Ready);
            agent::detect_state(
                kind,
                screen,
                title,
                self.activity.observation(previous, now),
            )
        });
        if next_state == self.state {
            return false;
        }
        self.state = next_state;
        true
    }

    pub fn status(&self) -> Option<AgentStatus> {
        self.kind
            .zip(self.state)
            .map(|(kind, state)| AgentStatus::single(kind, state))
    }

    #[cfg(test)]
    pub(super) fn evidence_snapshot(
        &self,
        now: Instant,
    ) -> (Option<Instant>, Option<u64>, Duration) {
        (
            self.title_updated_at,
            self.screen_signature,
            self.activity.observation(AgentState::Ready, now).quiet_for,
        )
    }

    fn reset_evidence(&mut self, next_signature: Option<u64>) {
        self.activity.reset();
        self.title_updated_at = None;
        self.screen_signature = next_signature;
    }
}

impl Tab {
    pub(super) fn refresh_agent_status(&mut self, now: Instant, force: bool) -> bool {
        let changed = self.agent.refresh(
            self.metadata.process(),
            self.terminal.screen(),
            self.terminal.title(),
            now,
            force,
        );
        if changed {
            self.recompose();
        }
        changed
    }
}

pub(super) fn recent_osc_title(
    title: Option<&str>,
    updated_at: Option<Instant>,
    now: Instant,
) -> Option<&str> {
    updated_at
        .filter(|updated| now.saturating_duration_since(*updated) <= agent::ACTIVITY_WINDOW)
        .and(title)
}
