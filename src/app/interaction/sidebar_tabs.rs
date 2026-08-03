use std::time::Instant;

use anyhow::Result;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::{app::App, render::Layout};

impl App {
    pub(super) fn begin_sidebar_tab_press(&mut self, event: MouseEvent, layout: Layout) -> bool {
        let Some(key) = self.sidebar_tab_key_at(event, layout) else {
            return false;
        };
        let now = Instant::now();
        self.presenter.set_hovered_sidebar_tab(Some(key), now);
        self.presenter.begin_sidebar_tab_press(key, now);
        self.frame.invalidate();
        true
    }

    pub(super) fn update_sidebar_tab_hover(&mut self, event: MouseEvent, layout: Layout) -> bool {
        if event.kind != MouseEventKind::Moved {
            return false;
        }
        let key = self.sidebar_tab_key_at(event, layout);
        let changed = self.presenter.set_hovered_sidebar_tab(key, Instant::now());
        if changed {
            self.frame.invalidate();
        }
        changed
    }

    pub(super) fn route_sidebar_tab_press(
        &mut self,
        event: MouseEvent,
        layout: Layout,
    ) -> Result<Option<bool>> {
        if !self.presenter.sidebar_tab_press_active() {
            return Ok(None);
        }
        let key = self.sidebar_tab_key_at(event, layout);
        match event.kind {
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved => {
                self.presenter.update_sidebar_tab_press(key, Instant::now());
                self.presenter.set_hovered_sidebar_tab(key, Instant::now());
                self.frame.invalidate();
                Ok(Some(true))
            }
            MouseEventKind::Up(MouseButton::Left) => {
                let selected = self
                    .presenter
                    .release_sidebar_tab_press(key, Instant::now());
                self.frame.invalidate();
                if let Some(index) = selected.and_then(|selected| {
                    self.book
                        .iter()
                        .position(|workspace| workspace.id() == selected)
                }) {
                    self.select_workspace(index)?;
                }
                Ok(Some(true))
            }
            _ => Ok(Some(false)),
        }
    }

    pub(in crate::app) fn clear_sidebar_pointer(&mut self) {
        if self.presenter.clear_sidebar_pointer(Instant::now()) {
            self.frame.invalidate();
        }
    }

    fn sidebar_tab_key_at(&self, event: MouseEvent, layout: Layout) -> Option<u64> {
        (layout.sidebar_visible && event.column < layout.sidebar_width)
            .then(|| self.presenter.sidebar_tab_at(event.column, event.row))
            .flatten()
            .and_then(|index| self.book.get(index))
            .map(|workspace| workspace.id())
    }
}
