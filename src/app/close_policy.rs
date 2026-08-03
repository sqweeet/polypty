#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CloseTransition {
    Ignore,
    Quit,
    Active(usize),
}

pub(super) fn transition(count: usize, active: usize, closing: usize) -> CloseTransition {
    if count == 0 || active >= count || closing >= count {
        return CloseTransition::Ignore;
    }
    if count == 1 {
        return CloseTransition::Quit;
    }
    let active = if closing < active {
        active - 1
    } else if closing == active {
        active.min(count - 2)
    } else {
        active
    };
    CloseTransition::Active(active)
}

#[cfg(test)]
mod tests;
