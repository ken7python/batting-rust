use macroquad::prelude::*;

use crate::batting::{BALL_RADIUS, Contact, Grade, HIT_TIMING, MEET_RADIUS};
use crate::defense::{Defense, FIELDING_REACH, FieldReturn, Fielder};
use crate::park::Park;
use crate::runner::Runner;

pub const STEP: f64 = 1.0 / 120.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Homer,
    HardFair,
    Fair,
    Double,
    Triple,
    FlyOut(Fielder),
    GroundOut(Fielder),
    BaseOut(Fielder),
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
            Self::Double => "二塁打",
            Self::Triple => "三塁打",
            Self::FlyOut(_) => "フライアウト",
            Self::GroundOut(_) => "内野ゴロアウト",
            Self::BaseOut(_) => "アウト",
            Self::Foul => "ファウル",
            Self::Whiff => "空振り",
            Self::Strike => "見送りストライク",
            Self::Ball => "見送りボール",
        }
    }

    pub fn fielder(self) -> Option<Fielder> {
        match self {
            Self::FlyOut(fielder) | Self::GroundOut(fielder) | Self::BaseOut(fielder) => {
                Some(fielder)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct GroundContact {
    pub position: Vec3,
    pub at: f32,
}

#[derive(Clone, Copy)]
pub struct Flight {
    pub position: Vec3,
    pub velocity: Vec3,
    pub elapsed: f32,
    pub strong: bool,
    pub cleared_fence: bool,
    pub defense: Defense,
    pub returning: Option<FieldReturn>,
    pub runner: Runner,
    pub ground_contact: Option<GroundContact>,
    first_pickup: Option<Vec3>,
    park: Park,
    rolling: bool,
    hit_wall: bool,
    accumulator: f64,
}

impl Flight {
    pub fn new(crossing: Vec2, grade: Grade, park: Park) -> Self {
        let strong = grade.contact == Contact::Barrel;
        let quality = (1.0
            - 0.65 * (grade.offset / MEET_RADIUS).length()
            - 0.35 * (grade.timing.abs() / HIT_TIMING) as f32)
            .clamp(0.0, 1.0);
        let squared_quality = quality * quality;
        let speed = if strong {
            35.0 + 23.0 * squared_quality
        } else {
            26.0 + 10.0 * squared_quality
        };
        let horizontal =
            ((grade.timing / HIT_TIMING) as f32 * 1.05 + grade.offset.x * 0.48).clamp(-1.18, 1.18);
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
            returning: None,
            runner: Runner::default(),
            ground_contact: None,
            first_pickup: None,
            park,
            rolling: false,
            hit_wall: false,
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

    pub fn can_boost_runner(&self) -> bool {
        !self.cleared_fence
            && self.runner.moving()
            && !self.returning.is_some_and(|returning| returning.arrived())
    }

    fn step(&mut self) -> Option<Outcome> {
        let dt = STEP as f32;
        self.elapsed += dt;
        let was_running = self.runner.moving();
        if !self.cleared_fence {
            self.runner.advance(STEP);
        }
        let result = if self.returning.is_some() {
            self.step_throw(dt)
        } else {
            self.step_batted_ball(dt)
        };
        if result.is_none()
            && was_running
            && !self.runner.moving()
            && self.runner.target() < 3
            && !self.cleared_fence
            && self.time_to_defend_next_base() >= self.runner.next_leg_time() + 0.05
        {
            self.runner.start_next_leg();
        }
        result
    }

    fn step_throw(&mut self, dt: f32) -> Option<Outcome> {
        if let Some(returning) = &mut self.returning {
            if !returning.arrived() {
                returning.advance(&mut self.defense, dt);
                self.position = returning.ball_position(&self.defense);
            }
            if !returning.arrived() {
                return None;
            }
            let base = self.runner.target();
            if returning.base < base {
                self.returning = Some(FieldReturn::new(
                    returning.receiver,
                    self.position,
                    &self.defense,
                    base,
                ));
                return None;
            }
            if !self.runner.moving() {
                return Some(self.hit_at_base());
            }
            if returning.base == base {
                return Some(if base == 1 {
                    Outcome::GroundOut(returning.fielder)
                } else {
                    Outcome::BaseOut(returning.receiver)
                });
            }
        }
        None
    }

    fn time_to_defend_next_base(&self) -> f64 {
        let next = self.runner.target() + 1;
        if let Some(mut returning) = self.returning {
            let remaining = returning.remaining();
            if returning.base == next {
                return f64::from(remaining);
            }
            let mut defense = self.defense;
            returning.advance(&mut defense, remaining);
            let relay = FieldReturn::new(
                returning.receiver,
                returning.ball_position(&defense),
                &defense,
                next,
            );
            return f64::from(remaining + relay.remaining());
        }

        // Forecast the pickup using the same physics as the live play.
        let mut forecast = *self;
        forecast.runner.start_next_leg();
        for step in 1..=2400 {
            forecast.elapsed += STEP as f32;
            if forecast.step_batted_ball(STEP as f32).is_some() || forecast.cleared_fence {
                break;
            }
            if let Some(returning) = forecast.returning {
                return f64::from(step) * STEP + f64::from(returning.remaining());
            }
        }
        0.0
    }

    fn step_batted_ball(&mut self, dt: f32) -> Option<Outcome> {
        if self.rolling {
            self.roll(dt);
            return None;
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
            if !fair(crossing) {
                self.position = crossing;
                return Some(Outcome::Foul);
            }
            if self.strong && crossing.y >= fence_height {
                self.cleared_fence = true;
            } else {
                let normal = vec3(crossing.x, 0.0, crossing.z).normalize();
                self.position = crossing - normal * 0.03;
                self.velocity = (self.velocity - 2.0 * self.velocity.dot(normal) * normal) * 0.22;
                self.velocity.y = self.velocity.y.min(0.0);
                self.hit_wall = true;
                return None;
            }
        }
        if !self.cleared_fence {
            let reachable = old.lerp(next, ground.unwrap_or(1.0));
            if fair(reachable) {
                self.defense
                    .chase(reachable, self.velocity, self.elapsed, dt, false);
                if !self.hit_wall
                    && self.velocity.y < 0.0
                    && (0.45..=2.35).contains(&reachable.y)
                    && let Some(fielder) = self.defense.within_reach(reachable.xz(), FIELDING_REACH)
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
            self.ground_contact = Some(GroundContact {
                position: point,
                at: self.elapsed,
            });
            if !fair(point) {
                return Some(Outcome::Foul);
            }
            self.velocity = vec3(self.velocity.x * 0.73, 0.0, self.velocity.z * 0.73);
            self.rolling = true;
            return None;
        }
        self.position = next;
        None
    }

    fn roll(&mut self, dt: f32) {
        self.position += self.velocity * dt;
        self.velocity *= 1.0 - 1.25 * dt;
        let fence_radius = self.park.fence().0;
        if self.position.xz().length_squared() >= fence_radius * fence_radius {
            let stop = self.position.xz().normalize() * fence_radius;
            self.position = vec3(stop.x, BALL_RADIUS, stop.y);
            self.velocity = Vec3::ZERO;
        }
        self.defense
            .chase(self.position, self.velocity, self.elapsed, dt, true);
        if let Some(fielder) = self
            .defense
            .within_reach(self.position.xz(), FIELDING_REACH)
        {
            self.defense.chasing = Some(fielder);
            let base = self
                .runner
                .target()
                .max(if fielder.infield() { 1 } else { 2 });
            self.returning = Some(FieldReturn::new(
                fielder,
                self.position,
                &self.defense,
                base,
            ));
            self.first_pickup = Some(self.position);
            self.velocity = Vec3::ZERO;
        }
    }

    pub fn batted_point(&self) -> Vec3 {
        self.first_pickup.unwrap_or(self.position)
    }

    fn hit_at_base(&self) -> Outcome {
        match self.runner.reached() {
            1 if self.strong => Outcome::HardFair,
            1 => Outcome::Fair,
            2 => Outcome::Double,
            3 => Outcome::Triple,
            _ => unreachable!("a hit requires a runner on base"),
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
                return (outcome, flight.batted_point());
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
            contact: Contact::Barrel,
            timing: 0.0,
            offset: Vec2::ZERO,
        };
        for (park, expected) in [
            (Park::Sunset, Outcome::Homer),
            (Park::Central, Outcome::HardFair),
        ] {
            let mut flight = Flight::new(CENTER, grade, park);
            flight.position = vec3(0.0, 2.8, -76.5);
            flight.velocity = vec3(0.0, 0.0, -31.0);
            let result = (0..1200).find_map(|_| flight.advance(STEP));
            assert_eq!(result, Some(expected));
        }
    }

    #[test]
    fn both_parks_require_precise_timing_and_aim_for_a_home_run() {
        use crate::batting::{SWING_LEAD, Swing, grade};
        let arrival = 0.48;
        for park in [Park::Central, Park::Sunset] {
            for (timing, offset, expected_contact) in [
                (0.016, Vec2::ZERO, Contact::Touch),
                (0.0, vec2(0.050, 0.0), Contact::Touch),
                (0.0, vec2(0.03, -0.10), Contact::Touch),
                (0.0, Vec2::ZERO, Contact::Barrel),
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
                assert_eq!(hit.contact, expected_contact);
                if expected_contact == Contact::Barrel {
                    assert_eq!(outcome, Outcome::Homer);
                } else {
                    assert!(matches!(
                        outcome,
                        Outcome::Fair
                            | Outcome::Double
                            | Outcome::Triple
                            | Outcome::FlyOut(_)
                            | Outcome::GroundOut(_)
                    ));
                    assert!(end.xz().length() <= park.fence().0 + 1e-4);
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
            let outcome = (0..1000).find_map(|_| flight.advance(STEP));
            assert_eq!(outcome, Some(expected));
            if expected == Outcome::Fair {
                let contact = flight
                    .ground_contact
                    .expect("a hit should record its first landing");
                assert!((contact.position.y - BALL_RADIUS).abs() < 1e-5);
                assert!(contact.position.xz().distance(flight.batted_point().xz()) > 1.0);
                let returning = flight
                    .returning
                    .expect("the outfielder should return the ball");
                assert!(!returning.fielder.infield());
                assert_eq!(returning.target, crate::defense::SECOND_BASE);
                assert!(flight.position.xz().abs_diff_eq(returning.target, 0.01));
                assert!(flight.batted_point().xz().distance(returning.target) > 5.0);
            } else {
                assert!(flight.ground_contact.is_none());
            }
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
            flight.rolling = true;
            flight.elapsed = elapsed;
            flight.runner.advance(f64::from(elapsed));
            let mut threw = false;
            let outcome = (0..480).find_map(|_| {
                let outcome = flight.advance(STEP);
                if flight
                    .returning
                    .is_some_and(|returning| returning.airborne())
                {
                    assert!(outcome.is_none());
                    threw = true;
                }
                outcome
            });
            assert_eq!(outcome, Some(expected));
            assert!(threw);
            let first = crate::defense::FIRST_BASE;
            assert!(flight.position.xz().abs_diff_eq(first, 0.01));
            assert!(
                flight
                    .defense
                    .position(Fielder::First)
                    .abs_diff_eq(first, 0.01)
            );
        }
    }

    #[test]
    fn a_committed_runner_contests_the_next_base_after_a_relay() {
        for (extra_time, expected) in [
            (0.0, Outcome::BaseOut(Fielder::Third)),
            (1.9, Outcome::Triple),
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
            flight.runner.advance(3.95);
            flight.runner.start_next_leg();
            flight.runner.advance(2.75);
            flight.runner.start_next_leg();
            flight.runner.advance(extra_time);
            flight.defense.positions[Fielder::Second as usize] = crate::defense::SECOND_BASE;
            flight.defense.positions[Fielder::Third as usize] = crate::defense::THIRD_BASE;
            flight.returning = Some(FieldReturn::new(
                Fielder::Second,
                vec3(0.0, 1.1, -38.8),
                &flight.defense,
                2,
            ));
            let (outcome, saw_third) = (0..600)
                .scan(false, |saw_third, _| {
                    let result = flight.advance(STEP);
                    *saw_third |= flight.returning.is_some_and(|throw| throw.base == 3);
                    Some((result, *saw_third))
                })
                .find_map(|(result, saw_third)| result.map(|outcome| (outcome, saw_third)))
                .unwrap();
            assert!(saw_third);
            assert_eq!(outcome, expected);
            if outcome == Outcome::Triple {
                assert_eq!(flight.runner.position(), crate::defense::THIRD_BASE);
            }
        }
    }

    #[test]
    fn runners_hold_at_second_or_contest_third_in_both_parks() {
        for (park, timing, offset) in [
            (Park::Central, -0.060, vec2(0.025, 0.024)),
            (Park::Sunset, -0.040, vec2(0.075, 0.096)),
        ] {
            let hit = Grade {
                contact: Contact::Touch,
                timing,
                offset,
            };
            for (mode, expected) in [
                (0, Outcome::Double),
                (1, Outcome::Triple),
                (2, Outcome::BaseOut(Fielder::Third)),
            ] {
                let mut flight = Flight::new(CENTER, hit, park);
                let result = (0..1800).find_map(|step| {
                    if mode > 0
                        && step % 30 == 0
                        && flight.can_boost_runner()
                        && (mode != 2 || flight.runner.target() < 3)
                    {
                        flight.runner.tap();
                    }
                    flight.advance(STEP)
                });
                assert_eq!(result, Some(expected), "{park:?} with mode={mode}");
                match expected {
                    Outcome::Double => {
                        assert_eq!(flight.runner.position(), crate::defense::SECOND_BASE)
                    }
                    Outcome::Triple => {
                        assert_eq!(flight.runner.position(), crate::defense::THIRD_BASE)
                    }
                    _ => assert_eq!(flight.returning.unwrap().base, 3),
                }
            }
        }
    }

    #[test]
    fn first_baseman_can_carry_the_ball_to_the_bag() {
        let mut flight = Flight::new(
            CENTER,
            Grade {
                contact: Contact::Touch,
                timing: 0.0,
                offset: Vec2::ZERO,
            },
            Park::Central,
        );
        flight.position = vec3(14.7, BALL_RADIUS, -20.7);
        flight.velocity = vec3(0.0, 0.0, -4.0);
        flight.rolling = true;
        flight.elapsed = 1.1;
        let outcome = (0..480).find_map(|_| {
            let outcome = flight.advance(STEP);
            if let Some(returning) = flight.returning {
                assert!(returning.carrying());
                assert!(!returning.airborne());
            }
            outcome
        });
        assert_eq!(outcome, Some(Outcome::GroundOut(Fielder::First)));
        assert!(
            flight
                .position
                .xz()
                .abs_diff_eq(crate::defense::FIRST_BASE, 0.01)
        );
    }

    #[test]
    fn an_early_glancing_swing_can_go_foul() {
        let early = Grade {
            contact: Contact::Touch,
            timing: -0.079,
            offset: Vec2::ZERO,
        };
        let (result, point) = simulate(1.0 / 60.0, early, Park::Central);
        assert_eq!(result, Outcome::Foul);
        assert!(point.x < point.z);
    }

    #[test]
    fn a_foul_landing_records_the_first_ground_contact_before_resolution() {
        let mut flight = Flight::new(
            CENTER,
            Grade {
                contact: Contact::Touch,
                timing: -0.079,
                offset: Vec2::ZERO,
            },
            Park::Central,
        );
        assert_eq!(
            (0..900).find_map(|_| flight.advance(STEP)),
            Some(Outcome::Foul)
        );
        let contact = flight
            .ground_contact
            .expect("a foul on the grass should mark its landing");
        assert_eq!(contact.position, flight.position);
        assert_eq!(contact.at, flight.elapsed);
    }
}
