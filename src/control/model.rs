use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "kebab-case")]
pub(crate) enum ControlRequest {
    Ping,
    ListSessions,
    ListTabs,
    NewTab,
    SelectTab {
        target: String,
    },
    CloseTab {
        target: String,
    },
    CapturePane {
        tab: Option<String>,
        pane: Option<u64>,
    },
    SendKeys {
        tab: Option<String>,
        pane: Option<u64>,
        text: String,
        enter: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "response", rename_all = "kebab-case")]
pub(crate) enum ControlResponse {
    Pong { pid: u32 },
    Sessions { sessions: Vec<SessionSummary> },
    Tabs { tabs: Vec<TabSummary> },
    Tab { tab: TabSummary },
    Capture { capture: PaneCapture },
    Ack { message: String },
    Error { message: String },
}

impl ControlResponse {
    pub(crate) fn error(error: impl std::fmt::Display) -> Self {
        Self::Error {
            message: error.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SessionSummary {
    pub name: String,
    pub pid: u32,
    pub tabs: usize,
    pub active_tab: usize,
    pub attached: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TabSummary {
    pub index: usize,
    pub id: u64,
    pub active: bool,
    pub title: String,
    pub cwd: String,
    pub panes: Vec<u64>,
    pub agent: Option<AgentSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AgentSummary {
    pub name: String,
    pub state: String,
    pub panes: usize,
    pub mixed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PaneCapture {
    pub tab: usize,
    pub pane: u64,
    pub text: String,
}
