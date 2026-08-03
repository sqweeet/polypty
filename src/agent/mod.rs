//! Coding-agent identity and live-state detection.
//!
//! The facade stays intentionally small. Profiles own agent-specific identity,
//! while `AgentDetector` reduces fresh terminal evidence into a display state.

mod catalog;
mod detection;
mod identity;
mod model;
mod observation;
mod rollup;

pub(crate) use detection::detect_state;
pub use identity::{identify_name, identify_process};
pub use model::{AgentKind, AgentState, AgentStatus};
pub use rollup::rollup;

pub(crate) use observation::{AgentInteraction, AgentObservation};

use std::time::Duration;

pub const SCAN_INTERVAL: Duration = Duration::from_millis(120);
pub const ACTIVITY_WINDOW: Duration = Duration::from_millis(1_200);

#[cfg(test)]
mod tests;
