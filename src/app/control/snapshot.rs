use crate::control::{AgentSummary, SessionSummary, TabSummary, SESSION_NAME};

use super::App;

impl App {
    pub(super) fn session_summary(&self) -> SessionSummary {
        SessionSummary {
            name: SESSION_NAME.into(),
            pid: std::process::id(),
            tabs: self.book.len(),
            active_tab: self.book.active_index() + 1,
            attached: true,
        }
    }

    pub(super) fn tab_summaries(&self) -> Vec<TabSummary> {
        (0..self.book.len())
            .map(|index| self.tab_summary(index))
            .collect()
    }

    pub(super) fn active_tab_summary(&self) -> TabSummary {
        self.tab_summary(self.book.active_index())
    }

    pub(super) fn tab_summary(&self, index: usize) -> TabSummary {
        let workspace = self.book.get(index).expect("tab index exists");
        let info = workspace.info();
        TabSummary {
            index: index + 1,
            id: workspace.id(),
            active: index == self.book.active_index(),
            title: info.primary,
            cwd: info.secondary,
            panes: workspace.pane_ids(),
            agent: info.agent.map(|status| AgentSummary {
                name: status.kind.label().into(),
                state: status.state.label().into(),
                panes: status.panes,
                mixed: status.mixed_kinds,
            }),
        }
    }
}
