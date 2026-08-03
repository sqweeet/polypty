use anyhow::{bail, Context, Result};

use super::App;

impl App {
    pub(super) fn resolve_tab(&self, target: &str) -> Result<usize> {
        if matches!(target, "active" | ".") {
            return Ok(self.book.active_index());
        }
        if let Some(id) = target.strip_prefix('@') {
            let id = id
                .parse::<u64>()
                .with_context(|| format!("invalid tab target `{target}`"))?;
            return self
                .book
                .iter()
                .position(|workspace| workspace.id() == id)
                .with_context(|| format!("tab @{id} does not exist"));
        }
        let number = target
            .parse::<usize>()
            .with_context(|| format!("invalid tab target `{target}`"))?;
        if number == 0 || number > self.book.len() {
            bail!("tab index {number} does not exist");
        }
        Ok(number - 1)
    }

    pub(super) fn resolve_pane(
        &self,
        tab: Option<&str>,
        pane: Option<u64>,
    ) -> Result<(usize, u64)> {
        let index = match tab {
            Some(target) => self.resolve_tab(target)?,
            None => self.book.active_index(),
        };
        let workspace = self.book.get(index).context("mux has no active tab")?;
        let pane = pane.unwrap_or_else(|| workspace.active_pane_id());
        if !workspace.pane_ids().contains(&pane) {
            bail!("pane %{pane} does not belong to tab {}", index + 1);
        }
        Ok((index, pane))
    }
}
