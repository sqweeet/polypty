use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};

use crate::{app::App, config, render::ShortcutScope};

impl App {
    pub(super) fn handle_shortcut_dialog_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.cancel_shortcut_dialog();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.select_shortcut_scope(ShortcutScope::Session);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.select_shortcut_scope(ShortcutScope::Always);
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.shortcut_dialog.toggle();
                self.frame.invalidate();
            }
            KeyCode::Char('s' | 'S') => {
                self.confirm_shortcut_scope(ShortcutScope::Session);
            }
            KeyCode::Char('a' | 'A') => {
                self.confirm_shortcut_scope(ShortcutScope::Always);
            }
            KeyCode::Enter => {
                self.confirm_shortcut_scope(self.shortcut_dialog.selected());
            }
            _ => {}
        }
    }

    pub(super) fn handle_shortcut_dialog_mouse(&mut self, event: MouseEvent) -> bool {
        let scope = crate::render::shortcut_dialog_hit(self.layout(), event.column, event.row);
        if self.shortcut_dialog.is_pressed() {
            return match event.kind {
                MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved => {
                    let changed = self.shortcut_dialog.update_press(scope)
                        | scope.is_some_and(|scope| self.select_shortcut_scope(scope));
                    if changed {
                        self.frame.invalidate();
                    }
                    changed
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    let confirmed = self.shortcut_dialog.release(scope);
                    self.frame.invalidate();
                    if let Some(scope) = confirmed {
                        self.confirm_shortcut_scope(scope);
                    }
                    true
                }
                _ => false,
            };
        }
        if event.kind == MouseEventKind::Moved {
            return scope.is_some_and(|scope| self.select_shortcut_scope(scope));
        }
        if event.kind == MouseEventKind::Down(MouseButton::Left) {
            return scope.is_some_and(|scope| {
                self.select_shortcut_scope(scope);
                self.shortcut_dialog.press(scope);
                self.frame.invalidate();
                true
            });
        }
        false
    }

    pub(in crate::app) fn open_shortcut_dialog(&mut self, desired_visible: bool) {
        self.clear_sidebar_pointer();
        self.close_sidebar_menu();
        self.shortcut_dialog.open(desired_visible);
        self.frame.request_full_draw();
    }

    pub(in crate::app) fn cancel_shortcut_dialog(&mut self) -> bool {
        if !self.shortcut_dialog.close() {
            return false;
        }
        self.frame.request_hard_clear();
        true
    }

    fn confirm_shortcut_scope(&mut self, scope: ShortcutScope) {
        let Some(visible) = self.shortcut_dialog.desired_visible() else {
            return;
        };
        if scope == ShortcutScope::Always && !self.save_shortcuts_preference(visible) {
            self.shortcut_dialog.mark_save_failed();
            self.frame.invalidate();
            return;
        }
        self.shortcut_dialog.close();
        self.shortcuts_visible = visible;
        self.presenter.set_sidebar_shortcuts_visible(visible);
        self.frame.request_hard_clear();
    }

    fn save_shortcuts_preference(&self, visible: bool) -> bool {
        self.config_path
            .as_deref()
            .is_some_and(|path| config::save_sidebar_shortcuts(path, visible).is_ok())
    }

    fn select_shortcut_scope(&mut self, scope: ShortcutScope) -> bool {
        let changed = self.shortcut_dialog.select(scope);
        if changed {
            self.frame.invalidate();
        }
        changed
    }
}
