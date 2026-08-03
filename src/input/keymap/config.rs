use crate::input::Action;

pub(super) fn config_action(name: &str) -> Option<Action> {
    let action = match name {
        "quit" => Action::Quit,
        "new-tab" => Action::NewTab,
        "close-tab" => Action::CloseTab,
        "next-tab" => Action::NextTab,
        "prev-tab" => Action::PrevTab,
        "split-vertical" => Action::SplitVertical,
        "split-horizontal" => Action::SplitHorizontal,
        "close-pane" => Action::ClosePane,
        "next-pane" => Action::NextPane,
        "pane-left" => Action::PaneLeft,
        "pane-right" => Action::PaneRight,
        "pane-up" => Action::PaneUp,
        "pane-down" => Action::PaneDown,
        "toggle-sidebar" => Action::ToggleSidebar,
        "sidebar-wider" => Action::SidebarWider,
        "sidebar-narrower" => Action::SidebarNarrower,
        "paste-clipboard" => Action::PasteClipboard,
        _ => return config_tab(name),
    };
    Some(action)
}

fn config_tab(name: &str) -> Option<Action> {
    let number = name.strip_prefix("tab-")?.parse::<u8>().ok()?;
    (1..=9).contains(&number).then_some(Action::Tab(number))
}

pub(super) fn action_name(action: Action) -> &'static str {
    match action {
        Action::NewTab => "new-tab",
        Action::CloseTab => "close-tab",
        Action::NextTab => "next-tab",
        Action::PrevTab => "prev-tab",
        Action::SplitVertical => "split-vertical",
        Action::SplitHorizontal => "split-horizontal",
        Action::ClosePane => "close-pane",
        Action::NextPane => "next-pane",
        Action::PaneLeft => "pane-left",
        Action::PaneRight => "pane-right",
        Action::PaneUp => "pane-up",
        Action::PaneDown => "pane-down",
        Action::ToggleSidebar => "toggle-sidebar",
        Action::SidebarWider => "sidebar-wider",
        Action::SidebarNarrower => "sidebar-narrower",
        Action::PasteClipboard => "paste-clipboard",
        Action::Quit => "quit",
        Action::Tab(_) => "tab-number",
        Action::Forward => "forward",
    }
}
