use macroquad::prelude::*;

use crate::defense::{FIRST_BASE, PLAYER_SPEED, SECOND_BASE, THIRD_BASE};

const BASES: [Vec2; 4] = [Vec2::ZERO, FIRST_BASE, SECOND_BASE, THIRD_BASE];
const LEG_TIME: f64 = FIRST_BASE.x as f64 * std::f64::consts::SQRT_2 / PLAYER_SPEED as f64;
const ARRIVALS: [f64; 4] = [0.0, LEG_TIME, 2.0 * LEG_TIME, 3.0 * LEG_TIME];
pub const SLIDE_SECONDS: f64 = 0.34;
const BOOST_DURATION: f64 = 0.30;
const BOOST_EXTRA_SPEED: f64 = 0.3;

#[derive(Clone, Copy)]
pub struct Runner {
    progress: f64,
    boost_remaining: f64,
    target: usize,
    pub sliding: bool,
}

impl Default for Runner {
    fn default() -> Self {
        Self {
            progress: 0.0,
            boost_remaining: 0.0,
            target: 1,
            sliding: false,
        }
    }
}

impl Runner {
    pub fn advance(&mut self, dt: f64) {
        let boosted = dt.min(self.boost_remaining);
        self.boost_remaining -= boosted;
        self.progress =
            (self.progress + dt + boosted * BOOST_EXTRA_SPEED).min(ARRIVALS[self.target]);
    }

    pub fn target(&self) -> usize {
        self.target
    }

    pub fn remaining_time(&self) -> f64 {
        let remaining = (ARRIVALS[self.target] - self.progress).max(0.0);
        let boosted = (remaining / (1.0 + BOOST_EXTRA_SPEED)).min(self.boost_remaining);
        remaining - boosted * BOOST_EXTRA_SPEED
    }

    pub fn start_next_leg(&mut self) {
        debug_assert!(self.reached() == self.target && self.target < 3);
        self.target += 1;
        self.sliding = false;
    }

    pub fn tap(&mut self) {
        self.boost_remaining = BOOST_DURATION;
    }

    pub fn boosted(&self) -> bool {
        self.moving() && self.boost_remaining > 1e-9
    }

    pub fn reached(&self) -> usize {
        ARRIVALS.partition_point(|&time| time <= self.progress + 1e-9) - 1
    }

    pub fn moving(&self) -> bool {
        self.reached() < self.target
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
        let base = self.target - 1;
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
        runner.advance(LEG_TIME - 0.01);
        assert_eq!(runner.reached(), 0);
        runner.advance(0.01);
        assert_eq!(runner.reached(), 1);
        assert_eq!(runner.position(), FIRST_BASE);
        runner.advance(5.0);
        assert_eq!(runner.position(), FIRST_BASE);
        runner.start_next_leg();
        runner.advance(LEG_TIME - 0.01);
        assert_eq!(runner.reached(), 1);
        runner.advance(0.01);
        assert_eq!(runner.reached(), 2);
        assert_eq!(runner.position(), SECOND_BASE);
        runner.start_next_leg();
        runner.advance(9.0);
        assert_eq!(runner.reached(), 3);
        assert_eq!(runner.position(), THIRD_BASE);
        assert!(!runner.moving());
    }

    #[test]
    fn covering_fielders_match_the_unboosted_runner_while_the_ball_is_airborne() {
        use crate::defense::{Defense, Fielder};
        use crate::park::Park;

        let mut defense = Defense::new(Park::Central);
        let before = defense;
        let mut runner = Runner::default();
        let dt = 0.1;
        runner.advance(dt);
        defense.chase(vec3(0.0, 3.0, -60.0), Vec3::ZERO, 1.0, dt as f32, false);
        let run = runner.position().length();
        for (fielder, base) in [
            (Fielder::First, FIRST_BASE),
            (Fielder::Shortstop, SECOND_BASE),
            (Fielder::Third, THIRD_BASE),
        ] {
            let closed =
                before.position(fielder).distance(base) - defense.position(fielder).distance(base);
            assert!(
                (closed - run).abs() < 1e-5,
                "{fielder:?} covered {closed}, runner covered {run}"
            );
        }
    }

    #[test]
    fn taps_refresh_the_boost_and_cannot_stack_speed_or_survive_a_hold() {
        let mut runner = Runner::default();
        runner.tap();
        runner.tap();
        runner.advance(0.20);
        assert!((runner.progress - 0.224).abs() < 1e-9);
        runner.tap();
        runner.advance(0.50);
        assert!((runner.progress - 0.760).abs() < 1e-9);
        assert!(!runner.boosted());
    }
}
