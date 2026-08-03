//! Stateful OSC metadata tracking for PTY byte streams.

mod decode;
mod parser;
mod tracker;

pub use tracker::OscTracker;

#[cfg(test)]
mod tests;
