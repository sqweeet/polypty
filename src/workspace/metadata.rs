use crate::{agent, info::TabInfo, session::TerminalSession};

use super::Workspace;

impl Workspace {
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub(super) fn active_session(&self) -> &dyn TerminalSession {
        self.panes
            .get(self.focus.active())
            .expect("active pane exists")
            .session
            .as_ref()
    }

    pub(super) fn active_session_mut(&mut self) -> &mut dyn TerminalSession {
        self.panes
            .get_mut(self.focus.active())
            .expect("active pane exists")
            .session
            .as_mut()
    }

    pub fn info(&self) -> TabInfo {
        let mut info = self.active_session().info().clone();
        info.agent = self.agent_status();
        info
    }

    pub fn agent_status(&self) -> Option<agent::AgentStatus> {
        let active = self.focus.active();
        agent::rollup(
            self.panes
                .iter()
                .filter(|pane| pane.id() == active)
                .chain(self.panes.iter().filter(|pane| pane.id() != active))
                .filter_map(|pane| pane.session.info().agent),
        )
    }

    pub fn active_screen(&self) -> &vt100::Screen {
        self.active_session().screen()
    }
}
