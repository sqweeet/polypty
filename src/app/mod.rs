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

use crate::platform::clipboard::{Clipboard, SystemClipboard};
use crate::render::Presenter;
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
}

impl App {
    pub fn new(cols: u16, rows: u16) -> Result<Self> {
        Self::with_clipboard(cols, rows, Box::new(SystemClipboard))
    }

    pub fn with_clipboard(cols: u16, rows: u16, clipboard: Box<dyn Clipboard>) -> Result<Self> {
        Self::with_services(cols, rows, clipboard, Box::new(PtySessionFactory))
    }

    pub(crate) fn with_services(
        cols: u16,
        rows: u16,
        clipboard: Box<dyn Clipboard>,
        sessions: Box<dyn SessionFactory>,
    ) -> Result<Self> {
        let mut app = Self {
            book: WorkspaceBook::default(),
            viewport: Viewport::new(cols, rows),
            frame: FrameScheduler::default(),
            presenter: Presenter::default(),
            clipboard,
            sessions,
        };
        app.spawn_workspace()?;
        Ok(app)
    }
}

#[cfg(test)]
mod tests;
