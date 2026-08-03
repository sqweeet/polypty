use super::AgentStatus;

/// Roll pane statuses up without allowing an equal-priority background pane
/// to replace the first (normally active) pane.
pub fn rollup(statuses: impl IntoIterator<Item = AgentStatus>) -> Option<AgentStatus> {
    let mut selected: Option<AgentStatus> = None;
    let mut first_kind = None;
    let mut panes = 0usize;
    let mut mixed_kinds = false;

    for next in statuses {
        panes = panes.saturating_add(next.panes.max(1));
        mixed_kinds |= next.mixed_kinds;
        if let Some(first_kind) = first_kind {
            mixed_kinds |= first_kind != next.kind;
        } else {
            first_kind = Some(next.kind);
        }

        selected = match selected {
            Some(current) if current.state.priority() >= next.state.priority() => Some(current),
            _ => Some(next),
        };
    }

    selected.map(|mut status| {
        status.panes = panes;
        status.mixed_kinds = mixed_kinds;
        status
    })
}
