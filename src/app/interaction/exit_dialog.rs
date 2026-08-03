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
        if matches!(event.kind, MouseEventKind::Moved | MouseEventKind::Drag(_)) {
            let changed = match button {
                Some(ExitDialogButton::Cancel) => self.exit_dialog.select_exit(false),
                Some(ExitDialogButton::Exit) => self.exit_dialog.select_exit(true),
                None => false,
            };
            if changed {
                self.frame.invalidate();
            }
            return Ok(changed);
        }
        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return Ok(false);
        }
        match button {
            Some(ExitDialogButton::Cancel) => self.cancel_exit(),
            Some(ExitDialogButton::Exit) => {
                self.confirm_exit();
                return Ok(false);
            }
            None => return Ok(false),
        }
        Ok(true)
    }
}
