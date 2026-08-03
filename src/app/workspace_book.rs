use crate::workspace::Workspace;

#[derive(Default)]
pub(super) struct WorkspaceBook {
    items: Vec<Workspace>,
    active: usize,
    next_pane_id: u64,
}

impl WorkspaceBook {
    pub(super) fn allocate_pane_id(&mut self) -> u64 {
        if self.next_pane_id == 0 {
            self.next_pane_id = 1;
        }
        let id = self.next_pane_id;
        self.next_pane_id += 1;
        id
    }

    pub(super) fn active(&self) -> Option<&Workspace> {
        self.items.get(self.active)
    }

    pub(super) fn active_index(&self) -> usize {
        self.active
    }
    pub(super) fn len(&self) -> usize {
        self.items.len()
    }
    pub(super) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub(super) fn get(&self, index: usize) -> Option<&Workspace> {
        self.items.get(index)
    }
    pub(super) fn get_mut(&mut self, index: usize) -> Option<&mut Workspace> {
        self.items.get_mut(index)
    }
    pub(super) fn iter(&self) -> std::slice::Iter<'_, Workspace> {
        self.items.iter()
    }
    pub(super) fn iter_mut(&mut self) -> std::slice::IterMut<'_, Workspace> {
        self.items.iter_mut()
    }

    pub(super) fn push_and_select(&mut self, workspace: Workspace) {
        self.items.push(workspace);
        self.active = self.items.len() - 1;
    }

    pub(super) fn remove(&mut self, index: usize) -> Option<Workspace> {
        if index >= self.items.len() {
            return None;
        }
        let removed = self.items.remove(index);
        if index < self.active {
            self.active -= 1;
        } else if !self.items.is_empty() {
            self.active = self.active.min(self.items.len() - 1);
        }
        Some(removed)
    }

    pub(super) fn active_mut(&mut self) -> Option<&mut Workspace> {
        self.items.get_mut(self.active)
    }

    pub(super) fn select(&mut self, index: usize) -> bool {
        if index >= self.items.len() || index == self.active {
            return false;
        }
        self.active = index;
        true
    }

    pub(super) fn next_index(&self) -> Option<usize> {
        (!self.items.is_empty()).then(|| (self.active + 1) % self.items.len())
    }

    pub(super) fn previous_index(&self) -> Option<usize> {
        (!self.items.is_empty()).then(|| {
            if self.active == 0 {
                self.items.len() - 1
            } else {
                self.active - 1
            }
        })
    }
}
