mod animation;
mod badge;
mod cache;
mod card;
mod footer;
mod frame;
mod glint;
mod hit_map;
mod model;
mod painter;
mod palette;
mod presentation;
mod tab_motion;
mod text;
mod viewport;

pub use cache::SidebarCache;
pub(crate) use footer::SidebarShortcuts;
pub use glint::GlintFrame;
pub use hit_map::SidebarMap;
pub use model::SidebarTab;
#[cfg(test)]
pub use painter::draw_sidebar;
pub(in crate::render) use presentation::SidebarPresentation;

#[cfg(test)]
mod tests;
