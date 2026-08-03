mod selection;

use std::time::{Duration, Instant};

pub(super) use selection::UiSelection;

pub(super) const FADE_DURATION: Duration = Duration::from_millis(120);
const PRESS_DURATION: Duration = Duration::from_millis(70);
const PRESS_START_ALPHA: u16 = 96;

pub(super) fn fade_opacity(started: Instant, now: Instant) -> u8 {
    eased_opacity(started, now, FADE_DURATION)
}

fn press_opacity(started: Instant, now: Instant) -> u8 {
    let eased = u16::from(eased_opacity(started, now, PRESS_DURATION));
    (PRESS_START_ALPHA + eased * (255 - PRESS_START_ALPHA) / 255) as u8
}

fn eased_opacity(started: Instant, now: Instant, duration: Duration) -> u8 {
    let elapsed = now.checked_duration_since(started).unwrap_or_default();
    if elapsed >= duration {
        return u8::MAX;
    }
    let linear = elapsed.as_micros().saturating_mul(255) / duration.as_micros();
    let remaining = 255_u128.saturating_sub(linear);
    (255_u128.saturating_sub(remaining.saturating_mul(remaining) / 255)) as u8
}

pub(super) struct UiPress<T> {
    target: Option<T>,
    inside: bool,
    started_at: Option<Instant>,
    complete: bool,
}

impl<T> Default for UiPress<T> {
    fn default() -> Self {
        Self {
            target: None,
            inside: false,
            started_at: None,
            complete: false,
        }
    }
}

impl<T: Copy + Eq> UiPress<T> {
    pub(super) fn begin(&mut self, target: T) -> bool {
        let changed = self.target != Some(target) || !self.inside;
        self.target = Some(target);
        self.inside = true;
        self.started_at = Some(Instant::now());
        self.complete = false;
        changed
    }

    pub(super) fn update(&mut self, hit: Option<T>) -> bool {
        let inside = self.target.is_some() && self.target == hit;
        let changed = inside != self.inside;
        if inside && !self.inside {
            self.started_at = Some(Instant::now());
            self.complete = false;
        }
        self.inside = inside;
        changed
    }

    pub(super) fn release(&mut self, hit: Option<T>) -> Option<T> {
        let target = self.target.filter(|target| Some(*target) == hit);
        *self = Self::default();
        target
    }

    pub(super) fn visual(&self, now: Instant) -> Option<(T, u8)> {
        if !self.inside {
            return None;
        }
        Some((self.target?, press_opacity(self.started_at?, now)))
    }

    pub(super) fn active(&self) -> bool {
        self.target.is_some()
    }

    pub(super) fn animation_due(&self) -> bool {
        self.target.is_some() && self.inside && !self.complete
    }

    pub(super) fn mark_frame(&mut self, opacity: Option<u8>) {
        self.complete |= opacity == Some(255);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_is_monotonic_and_finishes_opaque() {
        let start = Instant::now();
        let middle = fade_opacity(start, start + FADE_DURATION / 2);
        assert!(middle > 127);
        assert_eq!(fade_opacity(start, start), 0);
        assert_eq!(fade_opacity(start, start + FADE_DURATION), 255);
    }

    #[test]
    fn press_feedback_starts_visible_and_eases_to_full_strength() {
        let start = Instant::now();
        assert_eq!(press_opacity(start, start), PRESS_START_ALPHA as u8);
        assert_eq!(press_opacity(start, start + PRESS_DURATION), 255);
    }
}
