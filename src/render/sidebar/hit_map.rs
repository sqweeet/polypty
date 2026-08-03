use crate::agent::AgentState;

use super::glint::GlintFrame;
use super::model::SidebarContentRow;

/// Hit regions and visible animations for sidebar mouse control.
#[derive(Debug, Clone, Default)]
pub struct SidebarMap {
    pub row_tab: Vec<Option<usize>>,
    pub width: u16,
    visible_glints: Vec<(u64, GlintFrame)>,
}

impl SidebarMap {
    pub fn tab_at(&self, col: u16, row: u16) -> Option<usize> {
        if col >= self.width {
            return None;
        }
        self.row_tab.get(usize::from(row)).copied().flatten()
    }

    pub fn visible_glints(&self) -> &[(u64, GlintFrame)] {
        &self.visible_glints
    }

    pub(super) fn empty(rows: u16, width: u16) -> Self {
        Self {
            row_tab: vec![None; usize::from(rows)],
            width,
            visible_glints: Vec::new(),
        }
    }

    pub(super) fn record(&mut self, row_index: usize, row: &SidebarContentRow) {
        if let Some(tab_idx) = row.tab_idx {
            self.row_tab[row_index] = Some(tab_idx);
        }
        if row.agent_state != Some(AgentState::Working) || self.width < 6 {
            return;
        }
        let (Some(key), Some(frame)) = (row.key, row.glint_frame) else {
            return;
        };
        if !self
            .visible_glints
            .iter()
            .any(|(visible, _)| *visible == key)
        {
            self.visible_glints.push((key, frame));
        }
    }

    #[cfg(test)]
    pub(crate) fn with_visible_glints(visible_glints: Vec<(u64, GlintFrame)>) -> Self {
        Self {
            visible_glints,
            ..Self::default()
        }
    }
}
