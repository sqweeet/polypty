use std::time::Instant;

use crate::{
    core::geometry::TerminalRect,
    render::{ExitDialogButton, Layout, ShortcutDialogView, SidebarMenuView, SidebarTab},
};

use super::fingerprint::sidebar_fingerprint;
use crate::app::App;

pub(super) struct FramePlan {
    pub(super) layout: Layout,
    pub(super) area: TerminalRect,
    pub(super) active: usize,
    pub(super) hard_clear: bool,
    pub(super) cursor_settled: bool,
    pub(super) need_cursor_restore: bool,
    pub(super) resize_in_progress: bool,
    pub(super) sidebar_tabs: Vec<SidebarTab>,
    pub(super) fingerprint: String,
    pub(super) need_sidebar: bool,
    pub(super) need_workspace: bool,
    pub(super) dialog_visible: bool,
    pub(super) dialog_exit_selected: bool,
    pub(super) dialog_opacity: u8,
    pub(super) dialog_pressed: Option<ExitDialogButton>,
    pub(super) dialog_press_opacity: u8,
    pub(super) dialog_selection_opacity: (u8, u8),
    pub(super) sidebar_menu: Option<SidebarMenuView>,
    pub(super) shortcut_dialog: Option<ShortcutDialogView>,
}

impl FramePlan {
    pub(super) fn new(app: &App) -> Self {
        let now = Instant::now();
        let layout = app.layout();
        let area = layout.terminal_rect();
        let active = app.book.active_index();
        let cursor_settled = app.frame.cursor_settled(now);
        let sidebar_tabs = app
            .book
            .iter()
            .enumerate()
            .map(|(index, workspace)| {
                let info = workspace.info();
                let key = workspace.id();
                SidebarTab {
                    key,
                    primary: info.primary,
                    secondary: info.secondary,
                    agent: info.agent,
                    glint_frame: app.presenter.sidebar_frame(key, now),
                    active: index == active,
                }
            })
            .collect::<Vec<_>>();
        let fingerprint = sidebar_fingerprint(
            &sidebar_tabs,
            layout.sidebar_visible,
            layout.sidebar_width,
            layout.rows,
        );
        let glint_due = app.presenter.sidebar_frame_due(now);
        let need_sidebar = layout.sidebar_visible
            && (app.frame.force_draw()
                || app.frame.hard_clear()
                || fingerprint != app.presenter.sidebar_fingerprint()
                || glint_due);
        let need_workspace = app.frame.force_draw()
            || app.frame.hard_clear()
            || app.frame.viewport_dirty()
            || app.frame.cursor_restore_due(now)
            || app.book.active().is_some_and(|workspace| {
                app.presenter
                    .workspace_needs_draw(&workspace.snapshot(area))
            });
        let (dialog_pressed, dialog_press_opacity) = app.exit_dialog.press_visual(now);
        let dialog_selection_opacity = app.exit_dialog.selection_visual(now);
        Self {
            layout,
            area,
            active,
            hard_clear: app.frame.hard_clear(),
            cursor_settled,
            need_cursor_restore: app.frame.cursor_restore_due(now),
            resize_in_progress: app.frame.resize_in_progress(),
            sidebar_tabs,
            fingerprint,
            need_sidebar,
            need_workspace,
            dialog_visible: app.exit_dialog.visible(),
            dialog_exit_selected: app.exit_dialog.exit_selected(),
            dialog_opacity: app.exit_dialog.opacity(now),
            dialog_pressed,
            dialog_press_opacity,
            dialog_selection_opacity,
            sidebar_menu: app.sidebar_menu.view_at(app.shortcuts_visible, now),
            shortcut_dialog: app.shortcut_dialog.view_at(now),
        }
    }
}
