use std::time::{Duration, Instant};

use super::eased_opacity;

const DURATION: Duration = Duration::from_millis(120);

struct Choice<T> {
    target: T,
    from: u8,
    to: u8,
    started_at: Instant,
    painted: u8,
}

impl<T> Choice<T> {
    fn fixed(target: T, opacity: u8, now: Instant) -> Self {
        Self {
            target,
            from: opacity,
            to: opacity,
            started_at: now,
            painted: 0,
        }
    }

    fn opacity(&self, now: Instant) -> u8 {
        interpolate(
            self.from,
            self.to,
            eased_opacity(self.started_at, now, DURATION),
        )
    }

    fn retarget(&mut self, selected: bool, now: Instant) {
        self.from = self.opacity(now);
        self.to = if selected { 255 } else { 0 };
        self.started_at = now;
    }

    fn animation_due(&self, now: Instant) -> bool {
        let opacity = self.opacity(now);
        opacity != self.painted || opacity != self.to
    }
}

pub(in crate::app) struct UiSelection<T> {
    selected: Option<T>,
    choices: Vec<Choice<T>>,
}

impl<T> Default for UiSelection<T> {
    fn default() -> Self {
        Self {
            selected: None,
            choices: Vec::new(),
        }
    }
}

impl<T: Copy + Eq> UiSelection<T> {
    pub(in crate::app) fn reset(&mut self, selected: T) {
        let now = Instant::now();
        self.selected = Some(selected);
        self.choices = vec![Choice::fixed(selected, 255, now)];
    }

    pub(in crate::app) fn select(&mut self, selected: T) -> bool {
        if self.selected == Some(selected) {
            return false;
        }
        let now = Instant::now();
        for choice in &mut self.choices {
            choice.retarget(choice.target == selected, now);
        }
        if !self.choices.iter().any(|choice| choice.target == selected) {
            let mut choice = Choice::fixed(selected, 0, now);
            choice.retarget(true, now);
            self.choices.push(choice);
        }
        self.selected = Some(selected);
        true
    }

    pub(in crate::app) fn opacity(&self, target: T, now: Instant) -> u8 {
        self.choices
            .iter()
            .find(|choice| choice.target == target)
            .map_or(0, |choice| choice.opacity(now))
    }

    pub(in crate::app) fn animation_due(&self) -> bool {
        let now = Instant::now();
        self.choices.iter().any(|choice| choice.animation_due(now))
    }

    pub(in crate::app) fn mark_frame(&mut self, target: T, opacity: u8) {
        if let Some(choice) = self
            .choices
            .iter_mut()
            .find(|choice| choice.target == target)
        {
            choice.painted = opacity;
        }
    }
}

fn interpolate(from: u8, to: u8, alpha: u8) -> u8 {
    let alpha = u16::from(alpha);
    ((u16::from(from) * (255 - alpha) + u16::from(to) * alpha) / 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_crossfades_both_choices() {
        let mut selection = UiSelection::default();
        selection.reset(1);
        assert!(selection.select(2));
        let now = Instant::now() + DURATION;
        assert_eq!(selection.opacity(1, now), 0);
        assert_eq!(selection.opacity(2, now), 255);
    }
}
