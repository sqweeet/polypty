use crate::agent::{AgentState, AgentStatus};

use super::glint::{GlintFrame, GlintRow};
use super::tab_motion::TabVisual;

/// Sidebar tab row model — cmux-style primary + secondary.
#[derive(Debug, Clone)]
pub struct SidebarTab {
    pub key: u64,
    pub primary: String,
    pub secondary: String,
    pub agent: Option<AgentStatus>,
    pub glint_frame: Option<GlintFrame>,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub(super) struct TabCard {
    pub(super) tab_idx: usize,
    pub(super) key: u64,
    pub(super) active: bool,
    pub(super) agent_state: Option<AgentState>,
    pub(super) glint_frame: Option<GlintFrame>,
    pub(super) visual: TabVisual,
    pub(super) lines: Vec<(u8, String)>,
}

#[derive(Debug, Clone)]
pub(super) struct SidebarContentRow {
    pub(super) tab_idx: Option<usize>,
    pub(super) key: Option<u64>,
    pub(super) active: bool,
    pub(super) agent_state: Option<AgentState>,
    pub(super) glint_frame: Option<GlintFrame>,
    pub(super) visual: TabVisual,
    pub(super) glint_row: GlintRow,
    pub(super) kind: u8,
    pub(super) text: String,
}

impl SidebarContentRow {
    pub(super) fn card(card: &TabCard, kind: u8, text: &str, glint_row: GlintRow) -> Self {
        Self {
            tab_idx: Some(card.tab_idx),
            key: Some(card.key),
            active: card.active,
            agent_state: card.agent_state,
            glint_frame: card.glint_frame,
            visual: card.visual,
            glint_row,
            kind,
            text: text.to_string(),
        }
    }

    pub(super) fn chrome(kind: u8, text: &str) -> Self {
        Self {
            tab_idx: None,
            key: None,
            active: false,
            agent_state: None,
            glint_frame: None,
            visual: TabVisual::default(),
            glint_row: GlintRow::Flat,
            kind,
            text: text.to_string(),
        }
    }

    pub(super) fn empty() -> Self {
        Self::chrome(0, "")
    }
}
