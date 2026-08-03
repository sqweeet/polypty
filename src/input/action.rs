/// Action handled by polypty itself instead of being forwarded to the child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    /// Jump to a one-based tab number in `1..=9`.
    Tab(u8),
    SplitVertical,
    SplitHorizontal,
    ClosePane,
    NextPane,
    PaneLeft,
    PaneRight,
    PaneUp,
    PaneDown,
    ToggleSidebar,
    SidebarWider,
    SidebarNarrower,
    /// Paste from CLIPBOARD (Ctrl+Shift+V).
    PasteClipboard,
    /// Literal byte sequence to send to the active PTY.
    Forward,
}
