use macroquad::prelude::*;

use crate::batting::{BALL_RADIUS, Contact, Grade, HIT_TIMING, MEET_RADIUS};
use crate::defense::{Defense, FieldReturn, Fielder};
use crate::park::Park;
use crate::runner::{Runner, SLIDE_SECONDS};

pub const STEP: f64 = 1.0 / 120.0;
// Speed up ball travel without increasing its range or speeding up runners.
const BALL_TEMPO: f32 = 1.35;
const SAFE_ADVANCE_MARGIN: f64 = 0.015;
const CLOSE_ADVANCE_MARGIN: f64 = 0.22;

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

    pub fn is_hit(self) -> bool {
        matches!(
            self,
            Self::Homer | Self::Triple | Self::Double | Self::HardFair | Self::Fair
        )
    }

    pub fn points(self) -> u32 {
        match self {
            Self::Homer => 500,
            Self::Triple => 400,
            Self::Double => 300,
            Self::HardFair => 250,
            Self::Fair => 100,
            Self::Ball => 25,
            _ => 0,
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
    pub try_close_advance: bool,
    pub try_dive: bool,
    pub confident: bool,
    pub home_run: bool,
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
        let velocity = vec3(
            horizontal.sin() * vertical.cos(),
            vertical.sin(),
            -horizontal.cos() * vertical.cos(),
        ) * speed;
        let landing = airborne_landing(crossing.extend(0.0), velocity);
        let confident = strong && fair(landing) && landing.xz().length() >= 115.0;
        Self {
            position: crossing.extend(0.0),
            velocity,
            elapsed: 0.0,
            strong,
            try_close_advance: false,
            try_dive: false,
            confident,
            home_run: false,
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
        !self.home_run
            && !self.confident
            && self.runner.moving()
            && !self.returning.is_some_and(|returning| {
                returning.arrived() && returning.base == self.runner.target()
            })
    }

    fn step(&mut self) -> Option<Outcome> {
        let dt = STEP as f32;
        self.elapsed += dt;
        let was_running = self.runner.moving();
        if was_running
            && self.runner.target() >= 2
            && let Some(returning) = self.returning
            && returning.base == self.runner.target()
        {
            let runner_time = self.runner.remaining_time();
            let ball_time = f64::from(returning.remaining());
            if runner_time <= SLIDE_SECONDS && (ball_time - runner_time).abs() <= 0.22 {
                self.runner.sliding = true;
            }
        }
        if self.confident {
            self.runner.advance(STEP * 0.38);
        } else if !self.home_run {
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
            && !self.home_run
        {
            let ball_time = self.time_to_defend_next_base();
            let runner_time = self.runner.next_leg_time();
            if ball_time >= runner_time + SAFE_ADVANCE_MARGIN {
                self.runner.start_next_leg();
            } else if self.try_close_advance
                && ball_time >= runner_time - CLOSE_ADVANCE_MARGIN
                && ball_time >= self.runner.fastest_next_leg_time() + STEP
            {
                // Commit once on a marginal play that steady tapping can still win.
                self.try_close_advance = false;
                self.runner.start_next_leg();
            }
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
            if forecast.step_batted_ball(STEP as f32).is_some() || forecast.home_run {
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
        let ball_dt = dt * BALL_TEMPO;
        self.velocity = air_velocity(self.velocity, ball_dt);
        let next = old + self.velocity * ball_dt;

        let (fence_radius, fence_height) = self.park.fence();
        let radius_sq = |p: Vec3| p.x * p.x + p.z * p.z;
        let ground = ground_fraction(old, next);
        let wall = if !self.home_run && radius_sq(next) >= fence_radius * fence_radius {
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
                self.home_run = true;
            } else {
                let normal = vec3(crossing.x, 0.0, crossing.z).normalize();
                self.position = crossing - normal * 0.03;
                self.velocity = (self.velocity - 2.0 * self.velocity.dot(normal) * normal) * 0.22;
                self.velocity.y = self.velocity.y.min(0.0);
                self.hit_wall = true;
                return None;
            }
        }
        if !self.home_run {
            let reachable = old.lerp(next, ground.unwrap_or(1.0));
            if fair(reachable) {
                self.defense.chase(
                    reachable,
                    self.velocity,
                    self.elapsed,
                    dt,
                    false,
                    self.runner.target(),
                );
                if !self.hit_wall && self.velocity.y < 0.0 {
                    let catch = if (0.45..=2.35).contains(&reachable.y) {
                        self.defense.within_reach(reachable.xz(), true)
                    } else {
                        None
                    };
                    let catch = catch.or_else(|| {
                        if self.try_dive {
                            self.defense.dive_for(reachable)
                        } else {
                            None
                        }
                    });
                    if let Some(fielder) = catch {
                        self.position = reachable;
                        return Some(Outcome::FlyOut(fielder));
                    }
                }
            }
        }
        if let Some(fraction) = ground {
            let point = old.lerp(next, fraction);
            self.position = point;
            if self.home_run {
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
        self.defense.chase(
            self.position,
            self.velocity,
            self.elapsed,
            dt,
            true,
            self.runner.target(),
        );
        if let Some(fielder) = self.defense.within_reach(self.position.xz(), false) {
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

fn air_velocity(velocity: Vec3, dt: f32) -> Vec3 {
    velocity + (vec3(0.0, -9.81, 0.0) - velocity * velocity.length() * 0.0032) * dt
}

fn ground_fraction(old: Vec3, next: Vec3) -> Option<f32> {
    (next.y <= BALL_RADIUS).then(|| ((old.y - BALL_RADIUS) / (old.y - next.y)).clamp(0.0, 1.0))
}

fn airborne_landing(mut pos: Vec3, mut velocity: Vec3) -> Vec3 {
    let dt = STEP as f32 * BALL_TEMPO;
    loop {
        velocity = air_velocity(velocity, dt);
        let next = pos + velocity * dt;
        if let Some(fraction) = ground_fraction(pos, next) {
            return pos.lerp(next, fraction);
        }
        pos = next;
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
    fn a_big_homer_walks_while_the_ball_travels_at_full_tempo() {
        let mut flight = Flight::new(
            CENTER,
            Grade {
                contact: Contact::Barrel,
                timing: 0.0,
                offset: Vec2::ZERO,
            },
            Park::Central,
        );
        assert!(flight.confident);
        flight.advance(0.5);
        assert!(flight.runner.position().length() > 0.5);
        assert!(flight.runner.position().length() < 2.0);
        assert!(flight.position.z < -30.0);
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
            if flight.home_run {
                assert!(flight.position.y > Park::Central.fence().1);
                assert!(flight.position.xz().length() >= Park::Central.fence().0);
                let result = (0..1200).find_map(|_| flight.advance(STEP));
                assert_eq!(result, Some(Outcome::Homer));
                assert!(flight.position.xz().length() >= 115.0);
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
            flight.confident = false; // This fixture replaces the initially predicted homer.
            flight.position = vec3(0.0, 2.8, -76.5);
            flight.velocity = vec3(0.0, 0.0, -31.0);
            let result = (0..1200).find_map(|_| flight.advance(STEP));
            assert_eq!(result, Some(expected));
            assert!(flight.position.xz().length() < 115.0);
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
                vec3(3.8, 7.5, -53.0),
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
    fn a_dive_only_catches_a_low_fly_just_beyond_normal_reach() {
        let hit = Grade {
            contact: Contact::Touch,
            timing: 0.0,
            offset: Vec2::ZERO,
        };
        for (allow_dive, height, gap, caught, dived) in [
            (true, 0.7, 1.7, true, true),
            (false, 0.7, 1.7, false, false),
            (true, 0.7, 0.7, true, false),
            (true, 0.7, 2.5, false, false),
            (true, 1.5, 1.7, false, false),
            (true, BALL_RADIUS, 1.7, false, false),
        ] {
            let mut flight = Flight::new(CENTER, hit, Park::Central);
            flight.try_dive = allow_dive;
            flight.elapsed = 1.0;
            flight.position = vec3(gap, height, -62.0);
            flight.velocity = vec3(0.0, -4.0, 0.0);
            let outcome = flight.advance(STEP);
            assert_eq!(outcome, caught.then_some(Outcome::FlyOut(Fielder::Center)));
            assert_eq!(flight.defense.diving, dived.then_some(Fielder::Center));
            if dived {
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
            flight.runner.advance(3.10);
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
    fn a_runner_slides_only_on_close_plays_at_second_or_third() {
        for (base, first, second) in [(1, 0.0, 0.0), (2, 3.95, 0.0), (3, 3.95, 3.10)] {
            for (runner_time, ball_time, slides) in [(0.26, 0.21, base > 1), (0.26, 0.65, false)] {
                let mut flight = Flight::new(
                    CENTER,
                    Grade {
                        contact: Contact::Touch,
                        timing: 0.0,
                        offset: Vec2::ZERO,
                    },
                    Park::Central,
                );
                for leg in [first, second] {
                    if leg != 0.0 {
                        flight.runner.advance(leg);
                        flight.runner.start_next_leg();
                    }
                }
                let leg = [3.95, 3.10, 3.25][base - 1];
                flight.runner.advance(leg - runner_time);
                let bag = [
                    crate::defense::FIRST_BASE,
                    crate::defense::SECOND_BASE,
                    crate::defense::THIRD_BASE,
                ][base - 1];
                flight.defense.positions[Fielder::Center as usize] = bag + Vec2::Y * 35.0;
                let mut returning =
                    FieldReturn::new(Fielder::Center, bag.extend(0.0), &flight.defense, base);
                returning.advance(&mut flight.defense, returning.remaining() - ball_time);
                flight.returning = Some(returning);
                assert!(flight.advance(STEP).is_none());
                assert_eq!(
                    flight.runner.sliding, slides,
                    "base {base} with ball time {ball_time}"
                );
            }
        }
    }

    #[test]
    fn a_close_attempt_can_be_lost_or_saved_by_tapping() {
        for (chance, tapping, ball_time, target, expected) in [
            (false, false, 2.91, 1, Outcome::Fair),
            (true, false, 2.91, 2, Outcome::BaseOut(Fielder::Second)),
            (true, true, 2.91, 2, Outcome::Double),
            (true, true, 2.70, 1, Outcome::Fair),
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
            flight.try_close_advance = chance;
            flight.runner.advance(3.95 - STEP);
            flight.defense.positions[Fielder::Second as usize] = crate::defense::SECOND_BASE;
            flight.defense.positions[Fielder::Center as usize] =
                crate::defense::SECOND_BASE - Vec2::Y * 110.0;
            let mut returning =
                FieldReturn::new(Fielder::Center, vec3(0.0, 1.1, -148.8), &flight.defense, 2);
            returning.advance(&mut flight.defense, returning.remaining() - ball_time);
            flight.returning = Some(returning);
            if ball_time < 2.8 {
                // Even with acceleration already active, this throw is too fast.
                flight.runner.tap();
            }
            assert!(flight.advance(STEP).is_none());
            assert_eq!(flight.runner.target(), target);
            let outcome = (0..600).find_map(|step| {
                if tapping && step % 28 == 0 && flight.can_boost_runner() {
                    flight.runner.tap();
                }
                flight.advance(STEP)
            });
            assert_eq!(outcome, Some(expected));
        }
    }

    #[test]
    fn runners_hold_at_second_or_contest_third_in_both_parks() {
        for (park, timing, offset) in [
            (Park::Central, -0.060, vec2(0.025, 0.089)),
            (Park::Sunset, -0.060, vec2(0.025, 0.089)),
        ] {
            let hit = Grade {
                contact: Contact::Touch,
                timing,
                offset,
            };
            for (tap, expected) in [(false, Outcome::Double), (true, Outcome::Triple)] {
                let mut flight = Flight::new(CENTER, hit, park);
                let result = (0..1800).find_map(|step| {
                    if tap && step % 30 == 0 && flight.can_boost_runner() {
                        flight.runner.tap();
                    }
                    flight.advance(STEP)
                });
                assert_eq!(result, Some(expected), "{park:?} with tap={tap}");
                let base = if tap {
                    crate::defense::THIRD_BASE
                } else {
                    crate::defense::SECOND_BASE
                };
                assert_eq!(flight.runner.position(), base);
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
