mod painter;

pub(crate) use painter::draw_sidebar_menu;

use super::Layout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarMenuAction {
    NewTab,
    CloseTab,
    ToggleShortcuts,
}

impl SidebarMenuAction {
    pub(crate) fn items(can_close: bool) -> &'static [Self] {
        const DEFAULT: &[SidebarMenuAction] = &[
            SidebarMenuAction::NewTab,
            SidebarMenuAction::ToggleShortcuts,
        ];
        const WITH_CLOSE: &[SidebarMenuAction] = &[
            SidebarMenuAction::NewTab,
            SidebarMenuAction::CloseTab,
            SidebarMenuAction::ToggleShortcuts,
        ];
        if can_close {
            WITH_CLOSE
        } else {
            DEFAULT
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarMenuView {
    pub anchor_column: u16,
    pub anchor_row: u16,
    pub selected: SidebarMenuAction,
    pub shortcuts_visible: bool,
    pub can_close: bool,
    pub opacity: u8,
    pub pressed: Option<SidebarMenuAction>,
    pub press_opacity: u8,
}

pub(crate) fn sidebar_menu_hit(
    layout: Layout,
    menu: SidebarMenuView,
    column: u16,
    row: u16,
) -> Option<SidebarMenuAction> {
    let geometry = geometry(layout, menu)?;
    if !(geometry.x..geometry.x + geometry.width).contains(&column)
        || !(geometry.y..geometry.y + geometry.height).contains(&row)
    {
        return None;
    }
    SidebarMenuAction::items(menu.can_close)
        .get(usize::from(row - geometry.y))
        .copied()
}

#[derive(Clone, Copy)]
pub(super) struct MenuGeometry {
    pub(super) x: u16,
    pub(super) y: u16,
    pub(super) width: u16,
    pub(super) height: u16,
}

pub(super) fn geometry(layout: Layout, menu: SidebarMenuView) -> Option<MenuGeometry> {
    if !layout.sidebar_visible || layout.sidebar_width == 0 || layout.rows == 0 {
        return None;
    }
    let width = SidebarMenuAction::items(menu.can_close)
        .iter()
        .map(|action| label(*action, menu.shortcuts_visible).len() as u16 + 2)
        .max()
        .unwrap_or(1)
        .min(layout.sidebar_width);
    let height = (SidebarMenuAction::items(menu.can_close).len() as u16).min(layout.rows);
    Some(MenuGeometry {
        x: menu.anchor_column.min(layout.sidebar_width - width),
        y: menu.anchor_row.min(layout.rows - height),
        width,
        height,
    })
}

pub(super) fn label(action: SidebarMenuAction, shortcuts_visible: bool) -> &'static str {
    match action {
        SidebarMenuAction::NewTab => "New tab",
        SidebarMenuAction::CloseTab => "Close tab",
        SidebarMenuAction::ToggleShortcuts => {
            if shortcuts_visible {
                "Hide shortcuts"
            } else {
                "Show shortcuts"
            }
        }
    }
}

#[cfg(test)]
mod tests;
