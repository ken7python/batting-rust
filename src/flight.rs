use macroquad::prelude::*;

use crate::batting::{BALL_RADIUS, Contact, Grade, MEET_RADIUS};
use crate::defense::{Defense, FIRST_BASE, Fielder};
use crate::park::Park;

pub const STEP: f64 = 1.0 / 120.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Homer,
    HardFair,
    Fair,
    FlyOut(Fielder),
    GroundOut(Fielder),
    Foul,
    Whiff,
    Strike,
    Ball,
}

impl Outcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Homer => "ホームラン",
            Self::HardFair => "強いヒット",
            Self::Fair => "ヒット",
            Self::FlyOut(_) => "フライアウト",
            Self::GroundOut(_) => "内野ゴロアウト",
            Self::Foul => "ファウル",
            Self::Whiff => "空振り",
            Self::Strike => "見送りストライク",
            Self::Ball => "見送りボール",
        }
    }

    pub fn fielder(self) -> Option<Fielder> {
        match self {
            Self::FlyOut(fielder) | Self::GroundOut(fielder) => Some(fielder),
            _ => None,
        }
    }
}

pub struct Flight {
    pub position: Vec3,
    pub velocity: Vec3,
    pub elapsed: f32,
    pub strong: bool,
    pub cleared_fence: bool,
    pub defense: Defense,
    park: Park,
    rolling_for: Option<f32>,
    accumulator: f64,
}

impl Flight {
    pub fn new(crossing: Vec2, grade: Grade, park: Park) -> Self {
        let strong = grade.contact == Contact::Barrel;
        let quality = (1.0
            - 0.65 * (grade.offset / MEET_RADIUS).length()
            - 0.35 * (grade.timing.abs() as f32 / 0.065))
            .clamp(0.0, 1.0);
        let squared_quality = quality * quality;
        let speed = if strong {
            35.0 + 23.0 * squared_quality
        } else {
            24.0 + 11.0 * squared_quality
        };
        let horizontal =
            (grade.timing as f32 / 0.065 * 1.05 + grade.offset.x * 0.48).clamp(-1.18, 1.18);
        let vertical = (0.43 - grade.offset.y * 3.1).clamp(-0.16, 0.83);
        Self {
            position: crossing.extend(0.0),
            velocity: vec3(
                horizontal.sin() * vertical.cos(),
                vertical.sin(),
                -horizontal.cos() * vertical.cos(),
            ) * speed,
            elapsed: 0.0,
            strong,
            cleared_fence: false,
            defense: Defense::new(park),
            park,
            rolling_for: None,
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
        self.elapsed += dt;
        if let Some(rolling_for) = self.rolling_for {
            return self.roll(dt, rolling_for);
        }
        let old = self.position;
        self.velocity +=
            (vec3(0.0, -9.81, 0.0) - self.velocity * self.velocity.length() * 0.0032) * dt;
        let next = old + self.velocity * dt;

        let (fence_radius, fence_height) = self.park.fence();
        let radius_sq = |p: Vec3| p.x * p.x + p.z * p.z;
        let ground = if next.y <= BALL_RADIUS {
            Some(((old.y - BALL_RADIUS) / (old.y - next.y)).clamp(0.0, 1.0))
        } else {
            None
        };
        let wall = if !self.cleared_fence && radius_sq(next) >= fence_radius * fence_radius {
            let (mut low, mut high) = (0.0, 1.0);
            for _ in 0..16 {
                let middle = (low + high) * 0.5;
                if radius_sq(old.lerp(next, middle)) >= fence_radius * fence_radius {
                    high = middle;
                } else {
                    low = middle;
                }
            }
            Some(high)
        } else {
            None
        };
        if let Some(fraction) = wall.filter(|w| *w <= ground.unwrap_or(1.0)) {
            let crossing = old.lerp(next, fraction);
            if fair(crossing) && crossing.y >= fence_height {
                self.cleared_fence = true;
            } else {
                self.position = crossing;
                return Some(if fair(crossing) {
                    self.hit()
                } else {
                    Outcome::Foul
                });
            }
        }
        if !self.cleared_fence {
            let reachable = old.lerp(next, ground.unwrap_or(1.0));
            if fair(reachable) {
                self.defense
                    .chase(reachable, self.velocity, self.elapsed, dt, false);
                if self.velocity.y < 0.0
                    && (0.45..=2.35).contains(&reachable.y)
                    && let Some(fielder) = self.defense.within_reach(reachable.xz(), 1.05)
                {
                    self.position = reachable;
                    return Some(Outcome::FlyOut(fielder));
                }
            }
        }
        if let Some(fraction) = ground {
            let point = old.lerp(next, fraction);
            self.position = point;
            if self.cleared_fence {
                return Some(Outcome::Homer);
            }
            if !fair(point) {
                return Some(Outcome::Foul);
            }
            self.velocity = vec3(self.velocity.x * 0.73, 0.0, self.velocity.z * 0.73);
            self.rolling_for = Some(0.0);
            return None;
        }
        self.position = next;
        None
    }

    fn roll(&mut self, dt: f32, rolling_for: f32) -> Option<Outcome> {
        self.position += self.velocity * dt;
        self.velocity *= 1.0 - 1.25 * dt;
        self.rolling_for = Some(rolling_for + dt);
        let fence_radius = self.park.fence().0;
        if self.position.xz().length_squared() >= fence_radius * fence_radius {
            let stop = self.position.xz().normalize() * fence_radius;
            self.position = vec3(stop.x, BALL_RADIUS, stop.y);
            return Some(self.hit());
        }
        self.defense
            .chase(self.position, self.velocity, self.elapsed, dt, true);
        if let Some(fielder) = self.defense.within_reach(self.position.xz(), 1.05) {
            let time_at_first =
                self.elapsed + 0.42 + self.position.xz().distance(FIRST_BASE) / 26.0;
            return Some(if fielder.infield() && time_at_first < 4.25 {
                Outcome::GroundOut(fielder)
            } else {
                self.hit()
            });
        }
        (rolling_for + dt >= 2.2 || self.velocity.length_squared() < 1.5).then(|| self.hit())
    }

    fn hit(&self) -> Outcome {
        if self.strong {
            Outcome::HardFair
        } else {
            Outcome::Fair
        }
    }
}

fn fair(point: Vec3) -> bool {
    point.z <= 0.0 && point.x.abs() <= -point.z + 1e-4
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batting::CENTER;

    fn simulate(frame_time: f64, grade: Grade, park: Park) -> (Outcome, Vec3) {
        let mut flight = Flight::new(CENTER, grade, park);
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
        let (a, p) = simulate(1.0 / 30.0, ideal, Park::Central);
        let (b, q) = simulate(1.0 / 144.0, ideal, Park::Central);
        assert_eq!(a, Outcome::Homer);
        assert_eq!(a, b);
        assert_eq!(p, q);
        assert!((p.y - BALL_RADIUS).abs() < 1e-5);
        assert!(p.xz().length() > Park::Central.fence().0 + 35.0);
    }

    #[test]
    fn clearing_the_fence_keeps_the_ball_in_flight_until_landing() {
        let ideal = Grade {
            contact: Contact::Barrel,
            timing: 0.0,
            offset: Vec2::ZERO,
        };
        let mut flight = Flight::new(CENTER, ideal, Park::Central);
        for _ in 0..240 {
            assert!(flight.advance(STEP).is_none());
            if flight.cleared_fence {
                assert!(flight.position.y > Park::Central.fence().1);
                assert!(flight.position.xz().length() >= Park::Central.fence().0);
                return;
            }
        }
        panic!("barreled hit did not clear the fence");
    }

    #[test]
    fn the_same_fly_ball_can_clear_sunset_but_hit_the_central_fence() {
        let grade = Grade {
            contact: Contact::Touch,
            timing: 0.0,
            offset: Vec2::ZERO,
        };
        for (park, expected) in [
            (Park::Sunset, Outcome::Homer),
            (Park::Central, Outcome::Fair),
        ] {
            let mut flight = Flight::new(CENTER, grade, park);
            flight.position = vec3(0.0, 2.8, -76.5);
            flight.velocity = vec3(0.0, 0.0, -31.0);
            let result = (0..120).find_map(|_| flight.advance(STEP));
            assert_eq!(result, Some(expected));
        }
    }

    #[test]
    fn glancing_and_late_hits_stay_in_both_parks_but_centered_contact_clears_them() {
        use crate::batting::{SWING_LEAD, Swing, grade};
        let arrival = 0.48;
        for park in [Park::Central, Park::Sunset] {
            for (timing, offset) in [
                (0.0, vec2(0.03, -0.10)),
                (0.04, Vec2::ZERO),
                (0.0, Vec2::ZERO),
            ] {
                let hit = grade(
                    Swing {
                        time: arrival - SWING_LEAD + timing,
                        aim: CENTER + offset,
                    },
                    arrival,
                    CENTER,
                );
                let (outcome, end) = simulate(1.0 / 60.0, hit, park);
                if hit.contact == Contact::Barrel {
                    assert_eq!(outcome, Outcome::Homer);
                } else {
                    assert_eq!(hit.contact, Contact::Touch);
                    assert!(matches!(
                        outcome,
                        Outcome::Fair | Outcome::FlyOut(_) | Outcome::GroundOut(_)
                    ));
                    assert!(end.xz().length() < park.fence().0 - 5.0);
                }
            }
        }
    }

    #[test]
    fn a_fly_to_the_center_fielder_is_caught_but_a_gap_shot_is_a_hit() {
        for (position, velocity, expected) in [
            (
                vec3(3.8, 5.5, -53.0),
                vec3(0.0, -2.0, -7.0),
                Outcome::FlyOut(Fielder::Center),
            ),
            (
                vec3(16.0, 2.8, -38.0),
                vec3(0.0, -5.0, -20.0),
                Outcome::Fair,
            ),
        ] {
            let mut flight = Flight::new(
                CENTER,
                Grade {
                    contact: Contact::Touch,
                    timing: 0.0,
                    offset: Vec2::ZERO,
                },
                Park::Central,
            );
            flight.position = position;
            flight.velocity = velocity;
            flight.elapsed = 1.0;
            let outcome = (0..480).find_map(|_| flight.advance(STEP));
            assert_eq!(outcome, Some(expected));
        }
    }

    #[test]
    fn a_grounder_is_an_out_only_when_the_throw_would_beat_the_batter() {
        for (elapsed, expected) in [
            (1.1, Outcome::GroundOut(Fielder::Second)),
            (4.0, Outcome::Fair),
        ] {
            let mut flight = Flight::new(
                CENTER,
                Grade {
                    contact: Contact::Touch,
                    timing: 0.0,
                    offset: Vec2::ZERO,
                },
                Park::Central,
            );
            flight.position = vec3(10.4, BALL_RADIUS, -24.0);
            flight.velocity = vec3(0.0, 0.0, -13.0);
            flight.rolling_for = Some(0.0);
            flight.elapsed = elapsed;
            let outcome = (0..150).find_map(|_| flight.advance(STEP));
            assert_eq!(outcome, Some(expected));
        }
    }

    #[test]
    fn an_early_glancing_swing_can_go_foul() {
        let early = Grade {
            contact: Contact::Touch,
            timing: -0.064,
            offset: Vec2::ZERO,
        };
        let (result, point) = simulate(1.0 / 60.0, early, Park::Central);
        assert_eq!(result, Outcome::Foul);
        assert!(point.x < point.z);
    }
}
