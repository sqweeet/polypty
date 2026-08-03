use crate::render::SidebarTab;

pub(super) fn sidebar_fingerprint(
    tabs: &[SidebarTab],
    visible: bool,
    width: u16,
    rows: u16,
) -> String {
    let mut fingerprint = format!("v{visible}|w{width}|r{rows}|");
    for (index, tab) in tabs.iter().enumerate() {
        let agent = tab
            .agent
            .map(|status| {
                format!(
                    "{}:{}:{}:{}",
                    status.kind.label(),
                    status.state.label(),
                    status.panes,
                    status.mixed_kinds as u8
                )
            })
            .unwrap_or_default();
        fingerprint.push_str(&format!(
            "{index}:{}:{}:{agent}:{}|",
            tab.primary, tab.secondary, tab.active as u8
        ));
    }
    fingerprint
}

#[cfg(test)]
mod tests;
