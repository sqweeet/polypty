use std::time::Duration;

use crossterm::style::Color;
use unicode_width::UnicodeWidthStr;

use crate::agent::{AgentKind, AgentState, AgentStatus};
use crate::render::Layout;

use super::animation::SidebarAnimation;
use super::badge::ready_badge_spans;
use super::card::build_cards;
use super::footer::{configured_footer, sidebar_footer, SidebarShortcuts};
use super::glint::{sidebar_paint_spans, working_glint_bg};
use super::text::pad_fit;
use super::{draw_sidebar, GlintFrame, SidebarCache, SidebarTab};

mod animation;
mod badge;
mod cards;
mod glint_math;
mod glint_render;
mod glint_timeline;
mod labels;
mod layout;
mod spans;
