use std::io::Write;

use anyhow::Result;
use crossterm::{
    cursor::{Hide, MoveTo},
    queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
};

use crate::render::{fade::blend, Layout};

use super::{geometry, label, SidebarMenuAction, SidebarMenuView};

const MENU_BG: Color = rgb(52);
const SELECTED_BG: Color = rgb(70);
const PRESSED_BG: Color = rgb(88);
const BASE_BG: Color = rgb(36);
const DANGER: Color = Color::Rgb {
    r: 220,
    g: 105,
    b: 105,
};
const DANGER_SELECTED_BG: Color = Color::Rgb {
    r: 104,
    g: 48,
    b: 48,
};
const DANGER_PRESSED_BG: Color = Color::Rgb {
    r: 124,
    g: 54,
    b: 54,
};

pub(crate) fn draw_sidebar_menu(
    out: &mut impl Write,
    layout: Layout,
    menu: SidebarMenuView,
) -> Result<()> {
    let Some(geometry) = geometry(layout, menu) else {
        return Ok(());
    };
    queue!(out, Hide, SetAttribute(Attribute::Reset))?;
    for (index, action) in SidebarMenuAction::items(menu.can_close)
        .iter()
        .take(usize::from(geometry.height))
        .enumerate()
    {
        let selected = *action == menu.selected;
        let pressed = menu.pressed == Some(*action);
        let danger = *action == SidebarMenuAction::CloseTab;
        let foreground = if danger && !selected {
            DANGER
        } else {
            rgb(if selected { 230 } else { 185 })
        };
        let normal_background = if danger && selected {
            DANGER_SELECTED_BG
        } else if selected {
            SELECTED_BG
        } else {
            MENU_BG
        };
        let pressed_background = if danger {
            DANGER_PRESSED_BG
        } else {
            PRESSED_BG
        };
        let background = if pressed {
            blend(normal_background, pressed_background, menu.press_opacity)
        } else {
            normal_background
        };
        queue!(
            out,
            MoveTo(geometry.x, geometry.y + index as u16),
            SetForegroundColor(blend(BASE_BG, foreground, menu.opacity)),
            SetBackgroundColor(blend(BASE_BG, background, menu.opacity)),
            Print(padded_label(
                label(*action, menu.shortcuts_visible),
                geometry.width
            )),
        )?;
    }
    queue!(out, ResetColor, SetAttribute(Attribute::Reset), Hide)?;
    Ok(())
}

fn padded_label(label: &str, width: u16) -> String {
    let width = usize::from(width);
    let mut result = " ".repeat(width);
    let available = width.saturating_sub(2);
    let visible = &label[..label.len().min(available)];
    if !visible.is_empty() {
        result.replace_range(1..1 + visible.len(), visible);
    }
    result
}

const fn rgb(value: u8) -> Color {
    Color::Rgb {
        r: value,
        g: value,
        b: value,
    }
}
