use super::catalog::AgentCatalog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Codex,
    Claude,
    OpenCode,
    Gemini,
    Cursor,
    Copilot,
    Kimi,
    Amp,
    Pi,
    Devin,
    Droid,
    Kiro,
    Grok,
}

impl AgentKind {
    pub fn label(self) -> &'static str {
        AgentCatalog::profile(self).label
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Ready,
    Working,
    Blocked,
}

impl AgentState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Working => "working",
            Self::Blocked => "blocked",
        }
    }

    pub fn priority(self) -> u8 {
        match self {
            Self::Ready => 1,
            Self::Working => 2,
            Self::Blocked => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentStatus {
    pub kind: AgentKind,
    pub state: AgentState,
    /// Number of agent panes represented by this workspace status.
    pub panes: usize,
    /// At least one represented pane is running a different agent kind.
    pub mixed_kinds: bool,
}

impl AgentStatus {
    pub const fn single(kind: AgentKind, state: AgentState) -> Self {
        Self {
            kind,
            state,
            panes: 1,
            mixed_kinds: false,
        }
    }
}
