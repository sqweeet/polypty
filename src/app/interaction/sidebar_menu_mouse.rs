use anyhow::Result;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::{app::App, render, render::SidebarMenuAction};

pub(super) enum MenuMouseRoute {
    Handled(bool),
    Continue(bool),
}

impl App {
    pub(super) fn route_sidebar_menu_mouse(
        &mut self,
        event: MouseEvent,
        layout: render::Layout,
    ) -> Result<MenuMouseRoute> {
        let in_sidebar = layout.sidebar_visible && event.column < layout.sidebar_width;
        if event.kind == MouseEventKind::Down(MouseButton::Right) && in_sidebar {
            self.open_sidebar_menu(event, layout);
            return Ok(MenuMouseRoute::Handled(true));
        }
        if !self.sidebar_menu.visible() {
            return Ok(MenuMouseRoute::Continue(false));
        }
        let hit = self.menu_hit(event, layout);
        if self.sidebar_menu.is_pressed() {
            return self.route_pressed_menu(event, hit);
        }
        match event.kind {
            MouseEventKind::Moved | MouseEventKind::Drag(_) if in_sidebar => {
                Ok(MenuMouseRoute::Handled(self.select_menu_hit(hit)))
            }
            MouseEventKind::Down(MouseButton::Left) if hit.is_some() => {
                let action = hit.expect("checked menu action");
                let changed = self.sidebar_menu.select(action) | self.sidebar_menu.press(action);
                self.frame.invalidate();
                Ok(MenuMouseRoute::Handled(changed))
            }
            MouseEventKind::Down(_) if hit.is_some() => Ok(MenuMouseRoute::Handled(false)),
            MouseEventKind::Down(_) => {
                let changed = self.close_sidebar_menu();
                Ok(MenuMouseRoute::Continue(changed))
            }
            _ if in_sidebar => Ok(MenuMouseRoute::Handled(false)),
            _ => Ok(MenuMouseRoute::Continue(false)),
        }
    }

    fn route_pressed_menu(
        &mut self,
        event: MouseEvent,
        hit: Option<SidebarMenuAction>,
    ) -> Result<MenuMouseRoute> {
        match event.kind {
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved => {
                let changed = self.sidebar_menu.update_press(hit) | self.select_menu_hit(hit);
                if changed {
                    self.frame.invalidate();
                }
                Ok(MenuMouseRoute::Handled(changed))
            }
            MouseEventKind::Up(MouseButton::Left) => {
                let action = self.sidebar_menu.release(hit);
                self.frame.invalidate();
                if let Some(action) = action {
                    self.run_sidebar_menu_action(action)?;
                }
                Ok(MenuMouseRoute::Handled(true))
            }
            _ => Ok(MenuMouseRoute::Handled(false)),
        }
    }

    fn menu_hit(&self, event: MouseEvent, layout: render::Layout) -> Option<SidebarMenuAction> {
        self.sidebar_menu
            .view(self.shortcuts_visible)
            .and_then(|menu| render::sidebar_menu_hit(layout, menu, event.column, event.row))
    }

    fn select_menu_hit(&mut self, hit: Option<SidebarMenuAction>) -> bool {
        hit.is_some_and(|action| self.sidebar_menu.select(action))
    }

    fn open_sidebar_menu(&mut self, event: MouseEvent, layout: render::Layout) {
        self.clear_sidebar_pointer();
        let target_tab = self
            .presenter
            .sidebar_tab_at(event.column, event.row)
            .and_then(|index| self.book.get(index))
            .map(|workspace| workspace.id());
        self.sidebar_menu.open(event.column, event.row, target_tab);
        let hit = self.menu_hit(event, layout);
        self.select_menu_hit(hit);
        self.presenter.invalidate_sidebar();
        self.frame.invalidate();
    }
}
