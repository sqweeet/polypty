use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::{app::App, render::SidebarMenuAction};

impl App {
    pub(super) fn route_sidebar_menu_key(&mut self, key: KeyEvent) -> Result<Option<bool>> {
        if !self.sidebar_menu.visible() {
            return Ok(None);
        }
        match key.code {
            KeyCode::Esc => {
                self.close_sidebar_menu();
                Ok(Some(false))
            }
            KeyCode::Up | KeyCode::BackTab => {
                self.select_sidebar_menu_previous();
                Ok(Some(false))
            }
            KeyCode::Down | KeyCode::Tab => {
                self.select_sidebar_menu_next();
                Ok(Some(false))
            }
            KeyCode::Enter => {
                let action = self.sidebar_menu.selected_action();
                self.run_sidebar_menu_action(action)?;
                Ok(Some(false))
            }
            _ => {
                self.close_sidebar_menu();
                Ok(None)
            }
        }
    }

    pub(in crate::app) fn close_sidebar_menu(&mut self) -> bool {
        if !self.sidebar_menu.close() {
            return false;
        }
        self.presenter.invalidate_sidebar();
        self.frame.request_full_draw();
        true
    }

    pub(in crate::app) fn run_sidebar_menu_action(
        &mut self,
        action: SidebarMenuAction,
    ) -> Result<()> {
        let target_tab = self.sidebar_menu.target_tab();
        self.close_sidebar_menu();
        match action {
            SidebarMenuAction::NewTab => self.spawn_workspace()?,
            SidebarMenuAction::CloseTab => {
                if let Some(index) = target_tab.and_then(|target| {
                    self.book
                        .iter()
                        .position(|workspace| workspace.id() == target)
                }) {
                    self.close_workspace(index)?;
                }
            }
            SidebarMenuAction::ToggleShortcuts => {
                self.open_shortcut_dialog(!self.shortcuts_visible);
            }
        }
        Ok(())
    }

    fn select_sidebar_menu_next(&mut self) {
        if self.sidebar_menu.select_next() {
            self.frame.invalidate();
        }
    }

    fn select_sidebar_menu_previous(&mut self) {
        if self.sidebar_menu.select_previous() {
            self.frame.invalidate();
        }
    }
}
