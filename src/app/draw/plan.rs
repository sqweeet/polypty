use std::time::Instant;

use crate::{
    core::geometry::TerminalRect,
    render::{Layout, SidebarTab},
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
        }
    }
}
