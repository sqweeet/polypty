use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Level {
    from: u8,
    target: u8,
    started_at: Option<Instant>,
    duration: Duration,
    painted: u8,
}

impl Level {
    pub(super) fn jump(&mut self, value: u8) {
        self.from = value;
        self.target = value;
        self.started_at = None;
    }

    pub(super) fn transition(
        &mut self,
        target: u8,
        now: Instant,
        duration: Duration,
        floor: u8,
    ) -> bool {
        if self.target == target {
            return false;
        }
        self.from = self.value(now).max(floor.min(target));
        self.target = target;
        self.started_at = (self.from != target).then_some(now);
        self.duration = duration;
        true
    }

    pub(super) fn value(&self, now: Instant) -> u8 {
        let Some(started_at) = self.started_at else {
            return self.target;
        };
        let elapsed = now.checked_duration_since(started_at).unwrap_or_default();
        if elapsed >= self.duration || self.duration.is_zero() {
            return self.target;
        }
        let linear = elapsed.as_micros().saturating_mul(255) / self.duration.as_micros();
        let remaining = 255_u128.saturating_sub(linear);
        let eased = 255_u128.saturating_sub(remaining.saturating_mul(remaining) / 255) as u8;
        interpolate(self.from, self.target, eased)
    }

    pub(super) fn frame_due(&self, now: Instant) -> bool {
        let value = self.value(now);
        value != self.painted || value != self.target
    }

    pub(super) fn mark_frame(&mut self, now: Instant) {
        self.painted = self.value(now);
    }
}

fn interpolate(from: u8, to: u8, alpha: u8) -> u8 {
    let alpha = u16::from(alpha);
    ((u16::from(from) * (255 - alpha) + u16::from(to) * alpha) / 255) as u8
}
