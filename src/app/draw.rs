mod fingerprint;
mod plan;
mod schedule;

use std::io::Write;

use anyhow::{Context, Result};

use crate::render;

use self::plan::FramePlan;
use super::App;

impl App {
    pub fn draw(&mut self, output: &mut impl Write) -> Result<()> {
        if !self.needs_draw() {
            return Ok(());
        }
        let plan = FramePlan::new(self);
        let frame_cols = if plan.need_workspace {
            plan.layout.cols
        } else {
            plan.layout.sidebar_width
        };
        let cells = (frame_cols as usize).saturating_mul(plan.layout.rows as usize);
        let mut frame = Vec::with_capacity(cells.saturating_mul(4));
        render::begin_sync(&mut frame)?;

        if plan.hard_clear {
            render::clear(&mut frame)?;
            if let Some(workspace) = self.book.get(plan.active) {
                self.presenter
                    .reset_workspace_blank(&workspace.snapshot(plan.area));
            }
            self.presenter.invalidate_sidebar();
        }
        self.paint_sidebar(&mut frame, &plan)?;
        self.paint_workspace(&mut frame, &plan)?;

        render::end_sync(&mut frame)?;
        output.write_all(&frame).context("write frame")?;
        output.flush().context("flush stdout")?;
        self.frame.finish_frame(plan.need_cursor_restore);
        Ok(())
    }

    fn paint_sidebar(&mut self, frame: &mut Vec<u8>, plan: &FramePlan) -> Result<()> {
        if plan.need_sidebar {
            self.presenter.draw_sidebar(
                frame,
                &plan.layout,
                &plan.sidebar_tabs,
                &plan.fingerprint,
                plan.hard_clear,
            )?;
        } else if !plan.layout.sidebar_visible {
            self.presenter.clear_sidebar();
        }
        Ok(())
    }

    fn paint_workspace(&mut self, frame: &mut Vec<u8>, plan: &FramePlan) -> Result<()> {
        let hide_cursor = !plan.cursor_settled || plan.resize_in_progress;
        if plan.need_workspace {
            if let Some(workspace) = self.book.get_mut(plan.active) {
                let painted = self.presenter.draw_workspace(
                    frame,
                    &workspace.snapshot(plan.area),
                    hide_cursor,
                    false,
                )?;
                workspace.mark_rendered(&painted);
            }
        } else if plan.need_sidebar {
            if let Some(workspace) = self.book.get(plan.active) {
                self.presenter.restore_workspace_cursor(
                    frame,
                    &workspace.snapshot(plan.area),
                    hide_cursor,
                )?;
            }
        }
        Ok(())
    }
}
