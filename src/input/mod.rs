//! Host input routing and terminal protocol encoding.

mod action;
mod keyboard;
mod keymap;
mod mouse;

pub use action::Action;
pub use keyboard::encode_key;
pub use keymap::map_key;
pub use mouse::encode_mouse;

#[cfg(test)]
mod tests;
