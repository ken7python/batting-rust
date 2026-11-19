use macroquad::prelude::*;

use crate::batting::{BALL_RADIUS, Contact, Grade};
use crate::scene::{FENCE_HEIGHT, FENCE_RADIUS};

pub const STEP: f64 = 1.0 / 120.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Homer,
    HardFair,
    Fair,
    Foul,
    Whiff,
    Strike,
    Ball,
}

impl Outcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Homer => "ホームラン",
            Self::HardFair => "強いフェア打球",
            Self::Fair => "フェア打球",
            Self::Foul => "ファウル",
            Self::Whiff => "空振り",
            Self::Strike => "見送りストライク",
            Self::Ball => "見送りボール",
        }
    }
}

pub struct Flight {
    pub position: Vec3,
    pub velocity: Vec3,
    pub elapsed: f32,
    pub strong: bool,
    accumulator: f64,
}

impl Flight {
    pub fn new(crossing: Vec2, grade: Grade) -> Self {
        let strong = grade.contact == Contact::Barrel;
        let speed = if strong { 49.0 } else { 34.0 };
        let horizontal =
            (grade.timing as f32 / 0.065 * 1.05 + grade.offset.x * 0.48).clamp(-1.18, 1.18);
        let vertical = (0.38 - grade.offset.y * 3.1).clamp(-0.16, 0.83);
        Self {
            position: crossing.extend(0.0),
            velocity: vec3(
                horizontal.sin() * vertical.cos(),
                vertical.sin(),
                -horizontal.cos() * vertical.cos(),
            ) * speed,
            elapsed: 0.0,
            strong,
            accumulator: 0.0,
        }
    }

    pub fn advance(&mut self, dt: f64) -> Option<Outcome> {
        self.accumulator += dt;
        while self.accumulator + 1e-9 >= STEP {
            self.accumulator -= STEP;
            if let Some(result) = self.step() {
                return Some(result);
            }
        }
        None
    }

    fn step(&mut self) -> Option<Outcome> {
        let dt = STEP as f32;
        let old = self.position;
        self.velocity +=
            (vec3(0.0, -9.81, 0.0) - self.velocity * self.velocity.length() * 0.0032) * dt;
        let next = old + self.velocity * dt;
        self.elapsed += dt;

        let radius_sq = |p: Vec3| p.x * p.x + p.z * p.z;
        let ground = if next.y <= BALL_RADIUS {
            Some(((old.y - BALL_RADIUS) / (old.y - next.y)).clamp(0.0, 1.0))
        } else {
            None
        };
        let wall = if radius_sq(next) >= FENCE_RADIUS * FENCE_RADIUS {
            let (mut low, mut high) = (0.0, 1.0);
            for _ in 0..16 {
                let middle = (low + high) * 0.5;
                if radius_sq(old.lerp(next, middle)) >= FENCE_RADIUS * FENCE_RADIUS {
                    high = middle;
                } else {
                    low = middle;
                }
            }
            Some(high)
        } else {
            None
        };
        let over_wall = wall.is_some_and(|w| w <= ground.unwrap_or(1.0));
        if let Some(fraction) = if over_wall { wall } else { ground } {
            self.position = old.lerp(next, fraction);
            let fair = self.position.z <= 0.0 && self.position.x.abs() <= -self.position.z + 1e-4;
            return Some(if !fair {
                Outcome::Foul
            } else if over_wall && self.position.y >= FENCE_HEIGHT {
                Outcome::Homer
            } else if self.strong {
                Outcome::HardFair
            } else {
                Outcome::Fair
            });
        }
        self.position = next;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batting::CENTER;

    fn simulate(frame_time: f64, grade: Grade) -> (Outcome, Vec3) {
        let mut flight = Flight::new(CENTER, grade);
        for _ in 0..2000 {
            if let Some(outcome) = flight.advance(frame_time) {
                return (outcome, flight.position);
            }
        }
        panic!("flight did not finish");
    }

    #[test]
    fn ideal_contact_clears_the_fence_at_any_render_rate() {
        let ideal = Grade {
            contact: Contact::Barrel,
            timing: 0.0,
            offset: Vec2::ZERO,
        };
        let (a, p) = simulate(1.0 / 30.0, ideal);
        let (b, q) = simulate(1.0 / 144.0, ideal);
        assert_eq!(a, Outcome::Homer);
        assert_eq!(a, b);
        assert_eq!(p, q);
        assert!(p.y >= FENCE_HEIGHT);
    }

    #[test]
    fn an_early_glancing_swing_can_go_foul() {
        let early = Grade {
            contact: Contact::Touch,
            timing: -0.064,
            offset: Vec2::ZERO,
        };
        let (result, point) = simulate(1.0 / 60.0, early);
        assert_eq!(result, Outcome::Foul);
        assert!(point.x < point.z);
    }
}
