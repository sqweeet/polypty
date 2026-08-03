use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};

use crate::{app::App, render::ExitDialogButton};

impl App {
    pub(super) fn handle_exit_dialog_key(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n' | 'N') => self.cancel_exit(),
            KeyCode::Left | KeyCode::Char('h') => {
                if self.exit_dialog.select_exit(false) {
                    self.frame.invalidate();
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.exit_dialog.select_exit(true) {
                    self.frame.invalidate();
                }
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.exit_dialog.toggle();
                self.frame.invalidate();
            }
            KeyCode::Char('y' | 'Y') => {
                self.confirm_exit();
                return Ok(true);
            }
            KeyCode::Enter if self.exit_dialog.exit_selected() => {
                self.confirm_exit();
                return Ok(true);
            }
            KeyCode::Enter => self.cancel_exit(),
            _ => {}
        }
        Ok(false)
    }

    pub(super) fn handle_exit_dialog_mouse(&mut self, event: MouseEvent) -> Result<bool> {
        let button = crate::render::exit_dialog_hit(self.layout(), event.column, event.row);
        if self.exit_dialog.is_pressed() {
            return match event.kind {
                MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved => {
                    let changed = self.exit_dialog.update_press(button)
                        | button.is_some_and(|button| self.select_exit_button(button));
                    if changed {
                        self.frame.invalidate();
                    }
                    Ok(changed)
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    let confirmed = self.exit_dialog.release(button);
                    self.frame.invalidate();
                    match confirmed {
                        Some(button) => self.activate_exit_button(button),
                        None => Ok(true),
                    }
                }
                _ => Ok(false),
            };
        }
        if event.kind == MouseEventKind::Moved {
            let changed = button.is_some_and(|button| self.select_exit_button(button));
            if changed {
                self.frame.invalidate();
            }
            return Ok(changed);
        }
        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return Ok(false);
        }
        let Some(button) = button else {
            return Ok(false);
        };
        self.select_exit_button(button);
        self.exit_dialog.press(button);
        self.frame.invalidate();
        Ok(true)
    }

    fn select_exit_button(&mut self, button: ExitDialogButton) -> bool {
        self.exit_dialog
            .select_exit(button == ExitDialogButton::Exit)
    }

    fn activate_exit_button(&mut self, button: ExitDialogButton) -> Result<bool> {
        match button {
            ExitDialogButton::Cancel => {
                self.cancel_exit();
                Ok(true)
            }
            ExitDialogButton::Exit => {
                self.confirm_exit();
                Ok(false)
            }
        }
    }
}
