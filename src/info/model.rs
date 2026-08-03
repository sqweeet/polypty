use crate::agent::AgentStatus;

/// Snapshot of live tab information presented by the sidebar.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TabInfo {
    pub primary: String,
    pub secondary: String,
    pub agent: Option<AgentStatus>,
}
