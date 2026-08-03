mod close_policy;
mod draw;
mod frame_scheduler;
mod interaction;
mod lifecycle;
mod polling;
mod resize;
mod session;
mod timing;
mod viewport;
mod workspace_book;

use anyhow::Result;

use crate::input::Keymap;
use crate::platform::clipboard::{Clipboard, SystemClipboard};
use crate::render::{Presenter, SidebarShortcuts};
use crate::session::{PtySessionFactory, SessionFactory};

use frame_scheduler::FrameScheduler;
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
}

impl App {
    pub(crate) fn configured(
        cols: u16,
        rows: u16,
        keymap: Keymap,
        sidebar_visible: bool,
        sidebar_width: u16,
        shell: Option<String>,
    ) -> Result<Self> {
        Self::with_components(
            cols,
            rows,
            Box::new(SystemClipboard),
            Box::new(PtySessionFactory::new(shell)),
            keymap,
            (sidebar_visible, sidebar_width),
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
            (true, 18),
        )
    }

    fn with_components(
        cols: u16,
        rows: u16,
        clipboard: Box<dyn Clipboard>,
        sessions: Box<dyn SessionFactory>,
        keymap: Keymap,
        sidebar: (bool, u16),
    ) -> Result<Self> {
        let shortcuts = sidebar_shortcuts(&keymap);
        let mut app = Self {
            book: WorkspaceBook::default(),
            viewport: Viewport::configured(cols, rows, sidebar.0, sidebar.1),
            frame: FrameScheduler::default(),
            presenter: Presenter::new(shortcuts),
            clipboard,
            sessions,
            keymap,
        };
        app.spawn_workspace()?;
        Ok(app)
    }
}

fn sidebar_shortcuts(keymap: &Keymap) -> SidebarShortcuts {
    use crate::input::Action;

    SidebarShortcuts {
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
