mod close_policy;
mod control;
mod draw;
mod exit_dialog;
mod frame_scheduler;
mod interaction;
mod lifecycle;
mod polling;
mod resize;
mod session;
mod shortcut_dialog;
mod sidebar_menu;
mod timing;
mod ui_animation;
mod viewport;
mod workspace_book;

use anyhow::Result;

use crate::input::Keymap;
use crate::platform::clipboard::{Clipboard, SystemClipboard};
use crate::render::{Presenter, SidebarShortcuts};
use crate::session::{PtySessionFactory, SessionFactory};

use exit_dialog::ExitDialog;
use frame_scheduler::FrameScheduler;
use shortcut_dialog::ShortcutDialog;
use sidebar_menu::SidebarMenu;
use viewport::Viewport;
use workspace_book::WorkspaceBook;

/// Application aggregate. Infrastructure details live in composed state
/// objects; capability modules implement the small public façade.
pub struct App {
    book: WorkspaceBook,
    viewport: Viewport,
    frame: FrameScheduler,
    presenter: Presenter,
    clipboard: Box<dyn Clipboard>,
    sessions: Box<dyn SessionFactory>,
    keymap: Keymap,
    exit_dialog: ExitDialog,
    sidebar_menu: SidebarMenu,
    shortcut_dialog: ShortcutDialog,
    shortcuts_visible: bool,
    config_path: Option<std::path::PathBuf>,
}

impl App {
    pub(crate) fn configured(
        cols: u16,
        rows: u16,
        keymap: Keymap,
        sidebar: (bool, u16, bool),
        shell: Option<String>,
        control_socket: Option<std::path::PathBuf>,
        config_path: Option<std::path::PathBuf>,
    ) -> Result<Self> {
        Self::with_components(
            cols,
            rows,
            Box::new(SystemClipboard),
            Box::new(PtySessionFactory::new(shell, control_socket)),
            keymap,
            sidebar,
            config_path,
        )
    }

    #[cfg(test)]
    pub(crate) fn with_services(
        cols: u16,
        rows: u16,
        clipboard: Box<dyn Clipboard>,
        sessions: Box<dyn SessionFactory>,
    ) -> Result<Self> {
        Self::with_components(
            cols,
            rows,
            clipboard,
            sessions,
            Keymap::default(),
            (true, 18, true),
            None,
        )
    }

    fn with_components(
        cols: u16,
        rows: u16,
        clipboard: Box<dyn Clipboard>,
        sessions: Box<dyn SessionFactory>,
        keymap: Keymap,
        sidebar: (bool, u16, bool),
        config_path: Option<std::path::PathBuf>,
    ) -> Result<Self> {
        let shortcuts = sidebar_shortcuts(&keymap, sidebar.2);
        let mut app = Self {
            book: WorkspaceBook::default(),
            viewport: Viewport::configured(cols, rows, sidebar.0, sidebar.1),
            frame: FrameScheduler::default(),
            presenter: Presenter::new(shortcuts),
            clipboard,
            sessions,
            keymap,
            exit_dialog: ExitDialog::default(),
            sidebar_menu: SidebarMenu::default(),
            shortcut_dialog: ShortcutDialog::default(),
            shortcuts_visible: sidebar.2,
            config_path,
        };
        app.spawn_workspace()?;
        Ok(app)
    }
}

fn sidebar_shortcuts(keymap: &Keymap, visible: bool) -> SidebarShortcuts {
    use crate::input::Action;

    SidebarShortcuts {
        visible,
        new_tab: keymap.binding_label(Action::NewTab),
        close_tab: keymap.binding_label(Action::CloseTab),
        next_tab: keymap.binding_label(Action::NextTab),
        split_vertical: keymap.binding_label(Action::SplitVertical),
        split_horizontal: keymap.binding_label(Action::SplitHorizontal),
        next_pane: keymap.binding_label(Action::NextPane),
        close_pane: keymap.binding_label(Action::ClosePane),
        toggle_sidebar: keymap.binding_label(Action::ToggleSidebar),
        quit: keymap.binding_label(Action::Quit),
    }
}

#[cfg(test)]
mod tests;
