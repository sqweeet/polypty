use crate::session::TerminalSession;

pub(super) struct Pane {
    pub(super) session: Box<dyn TerminalSession>,
}

impl Pane {
    fn new(session: Box<dyn TerminalSession>) -> Self {
        Self { session }
    }

    pub(super) fn id(&self) -> u64 {
        self.session.id()
    }
}

/// Owns pane order and tab lookup independently of split geometry.
pub(super) struct PaneStore {
    items: Vec<Pane>,
}

impl PaneStore {
    pub(super) fn new(session: Box<dyn TerminalSession>) -> Self {
        Self {
            items: vec![Pane::new(session)],
        }
    }

    pub(super) fn len(&self) -> usize {
        self.items.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(super) fn contains(&self, id: u64) -> bool {
        self.items.iter().any(|pane| pane.id() == id)
    }

    pub(super) fn get(&self, id: u64) -> Option<&Pane> {
        self.items.iter().find(|pane| pane.id() == id)
    }

    pub(super) fn get_mut(&mut self, id: u64) -> Option<&mut Pane> {
        self.items.iter_mut().find(|pane| pane.id() == id)
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &Pane> {
        self.items.iter()
    }

    pub(super) fn iter_mut(&mut self) -> impl Iterator<Item = &mut Pane> {
        self.items.iter_mut()
    }

    pub(super) fn push(&mut self, session: Box<dyn TerminalSession>) {
        self.items.push(Pane::new(session));
    }

    pub(super) fn remove(&mut self, id: u64) -> Option<Pane> {
        let index = self.items.iter().position(|pane| pane.id() == id)?;
        Some(self.items.remove(index))
    }

    pub(super) fn next_after(&self, id: u64) -> Option<u64> {
        let current = self.items.iter().position(|pane| pane.id() == id)?;
        self.items
            .get((current + 1) % self.items.len())
            .map(Pane::id)
    }
}
