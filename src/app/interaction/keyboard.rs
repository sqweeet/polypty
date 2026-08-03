use std::time::Instant;

use anyhow::Result;
use crossterm::event::KeyEvent;

use crate::{
    input::{self, Action},
    platform::clipboard::ClipboardKind,
    workspace::{PaneDirection, SplitAxis},
};

use crate::app::App;

impl App {
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        let quit = match input::map_key(key) {
            Action::Quit => {
                self.shutdown();
                true
            }
            Action::NewTab => self.spawn_workspace().map(|_| false)?,
            Action::CloseTab => self.close_workspace(self.book.active_index())?,
            Action::NextTab => self.select_next().map(|_| false)?,
            Action::PrevTab => self.select_prev().map(|_| false)?,
            Action::Tab(number) => self
                .select_workspace((number as usize).saturating_sub(1))
                .map(|_| false)?,
            Action::SplitVertical => self.split_active(SplitAxis::Vertical).map(|_| false)?,
            Action::SplitHorizontal => self.split_active(SplitAxis::Horizontal).map(|_| false)?,
            Action::ClosePane => self.close_active_pane()?,
            Action::NextPane => {
                if self
                    .book
                    .active_mut()
                    .is_some_and(|workspace| workspace.focus_next())
                {
                    self.on_pane_focus_changed()?;
                }
                false
            }
            Action::PaneLeft => self.focus_pane(PaneDirection::Left).map(|_| false)?,
            Action::PaneRight => self.focus_pane(PaneDirection::Right).map(|_| false)?,
            Action::PaneUp => self.focus_pane(PaneDirection::Up).map(|_| false)?,
            Action::PaneDown => self.focus_pane(PaneDirection::Down).map(|_| false)?,
            Action::ToggleSidebar => {
                self.viewport.toggle_sidebar();
                self.stage_workspace_resize();
                false
            }
            Action::SidebarWider => {
                if self.viewport.sidebar_visible() {
                    self.adjust_sidebar(2);
                } else {
                    self.viewport.show_wider();
                    self.stage_workspace_resize();
                }
                false
            }
            Action::SidebarNarrower => {
                if self.viewport.sidebar_visible() {
                    self.adjust_sidebar(-2);
                }
                false
            }
            Action::PasteClipboard => {
                if let Some(text) = self.clipboard.read(ClipboardKind::Clipboard) {
                    self.handle_paste(&text)?;
                }
                false
            }
            Action::Forward => {
                self.forward_key(key)?;
                false
            }
        };
        self.sync_sidebar_animation(Instant::now());
        Ok(quit)
    }

    fn close_active_pane(&mut self) -> Result<bool> {
        let close_workspace = self
            .book
            .active()
            .is_none_or(|workspace| workspace.pane_count() <= 1);
        if close_workspace {
            return self.close_workspace(self.book.active_index());
        }
        if self
            .book
            .active_mut()
            .is_some_and(|workspace| workspace.close_active_pane())
        {
            self.on_pane_focus_changed()?;
        }
        Ok(false)
    }

    fn forward_key(&mut self, key: KeyEvent) -> Result<()> {
        let (cursor, keypad) = self
            .book
            .active()
            .map(|workspace| {
                let screen = workspace.active_screen();
                (screen.application_cursor(), screen.application_keypad())
            })
            .unwrap_or((false, false));
        let bytes = input::encode_key(key, cursor, keypad);
        if !bytes.is_empty() {
            self.write_active(&bytes)?;
        }
        Ok(())
    }
}
