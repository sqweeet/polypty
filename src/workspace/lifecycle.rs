use super::Workspace;

impl Workspace {
    /// Close only the active pane. Returns false for the final pane.
    pub fn close_active_pane(&mut self) -> bool {
        if self.panes.len() <= 1 {
            return false;
        }
        let id = self.focus.active();
        if let Some(pane) = self.panes.get_mut(id) {
            pane.session.kill();
        }
        self.remove_pane(id)
    }

    /// Reap exited panes. Returns true when split geometry changed.
    pub fn reap(&mut self) -> bool {
        let dead: Vec<u64> = self
            .panes
            .iter_mut()
            .filter_map(|pane| {
                (pane.session.try_reap() && !pane.session.is_alive()).then_some(pane.id())
            })
            .collect();
        let mut changed = false;
        for id in dead {
            if let Some(pane) = self.panes.get_mut(id) {
                let _ = pane.session.poll();
            }
            changed |= self.remove_pane(id);
        }
        changed
    }

    pub fn is_empty(&self) -> bool {
        self.panes.is_empty()
    }

    pub fn kill_all(&mut self) {
        for pane in self.panes.iter_mut() {
            pane.session.kill();
        }
    }

    fn remove_pane(&mut self, id: u64) -> bool {
        if self.panes.remove(id).is_none() {
            return false;
        }
        let removed = self.tree.remove(id);
        debug_assert!(removed);
        if !self.panes.contains(self.focus.active()) {
            if let Some(first) = self.tree.first_leaf() {
                self.focus.activate(first);
            }
        }
        true
    }
}
