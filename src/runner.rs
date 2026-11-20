use macroquad::prelude::*;

use crate::defense::{FIRST_BASE, SECOND_BASE, THIRD_BASE};

const BASES: [Vec2; 4] = [Vec2::ZERO, FIRST_BASE, SECOND_BASE, THIRD_BASE];
const ARRIVALS: [f64; 4] = [0.0, 3.95, 6.70, 9.45];

#[derive(Clone, Copy, Default)]
pub struct Runner {
    progress: f64,
}

impl Runner {
    pub fn advance(&mut self, dt: f64) {
        self.progress = (self.progress + dt).min(ARRIVALS[3]);
    }

    pub fn reached(&self) -> usize {
        ARRIVALS.partition_point(|&time| time <= self.progress + 1e-9) - 1
    }

    pub fn moving(&self) -> bool {
        self.reached() < 3
    }

    pub fn position(&self) -> Vec2 {
        let base = self.reached();
        if base == 3 {
            return THIRD_BASE;
        }
        let leg_time = ARRIVALS[base + 1] - ARRIVALS[base];
        let fraction = ((self.progress - ARRIVALS[base]) / leg_time).clamp(0.0, 1.0);
        BASES[base].lerp(BASES[base + 1], fraction as f32)
    }

    pub fn direction(&self) -> Vec2 {
        let base = self.reached().min(2);
        (BASES[base + 1] - BASES[base]).normalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runner_touches_all_three_bases_and_stops_at_third() {
        let mut runner = Runner::default();
        assert_eq!(runner.position(), Vec2::ZERO);
        runner.advance(3.94);
        assert_eq!(runner.reached(), 0);
        runner.advance(0.01);
        assert_eq!(runner.reached(), 1);
        assert_eq!(runner.position(), FIRST_BASE);
        runner.advance(2.75);
        assert_eq!(runner.reached(), 2);
        assert_eq!(runner.position(), SECOND_BASE);
        runner.advance(9.0);
        assert_eq!(runner.reached(), 3);
        assert_eq!(runner.position(), THIRD_BASE);
        assert!(!runner.moving());
    }
}
