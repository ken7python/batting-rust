use macroquad::prelude::*;

use crate::batting::{
    BALL_RADIUS, BARREL_RADIUS, Contact, Grade, HIT_TIMING, MEET_RADIUS, ON_TIME_WINDOW,
};
use crate::defense::{Defense, FieldReturn, Fielder};
use crate::park::Park;
use crate::runner::{Runner, SLIDE_SECONDS};

pub const STEP: f64 = 1.0 / 120.0;
// Speed up ball travel without increasing its range or speeding up runners.
const BALL_TEMPO: f32 = 1.50;
const GROUND_SPEED_RETENTION: f32 = 0.85;
const SAFE_ADVANCE_MARGIN: f64 = 0.10;

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
            Self::HardFair | Self::Fair => "ヒット",
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
    pub try_dive: bool,
    pub try_infield_dive: bool,
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
        let aim_error = (grade.offset / MEET_RADIUS).length().min(1.0);
        let timing_fraction = (grade.timing / HIT_TIMING) as f32;
        let quality = (1.0 - 0.65 * aim_error - 0.315 * timing_fraction.abs()).clamp(0.0, 1.0);
        let contact_speed = 29.0 + 10.0 * quality * quality - 3.0 * aim_error;
        let lift = (grade.offset.y / 0.015).clamp(0.0, 1.0);
        // Exit speed meets the ordinary contact speed at both edges of the barrel window.
        let speed = if strong {
            let centered = (1.0 - grade.offset.length() / BARREL_RADIUS).max(0.0);
            let timed = (1.0 - (grade.timing.abs() / ON_TIME_WINDOW) as f32).max(0.0);
            contact_speed + (2.0 + 10.0 * lift) * centered * timed.powi(3)
        } else {
            contact_speed
        };
        let horizontal = (timing_fraction * 0.945 + grade.offset.x * 0.48).clamp(-1.18, 1.18);
        // Swing offsets are cursor minus ball. A ball above the cursor lifts.
        let height = (-grade.offset.y / MEET_RADIUS).clamp(-1.0, 1.0);
        let ordinary_angle = if height < 0.0 {
            (18.0 + 84.0 * height).max(-24.0)
        } else {
            18.0 + 48.0 * height
        };
        // In the core, aim high to lift the ball or hold center or lower for a line drive.
        let angle_degrees = if strong {
            11.0 + 17.0 * lift
        } else {
            ordinary_angle
        };
        // Soften low drives while leaving ground balls and high arcs their own pace.
        let liner =
            (angle_degrees / 8.0).clamp(0.0, 1.0) * ((27.0 - angle_degrees) / 12.0).clamp(0.0, 1.0);
        let speed = speed * (1.0 - 0.12 * liner);
        let vertical = angle_degrees.to_radians();
        let velocity = vec3(
            horizontal.sin() * vertical.cos(),
            vertical.sin(),
            -horizontal.cos() * vertical.cos(),
        ) * speed;
        let landing = airborne_landing(crossing.extend(0.0), velocity);
        let confident = strong && fair(landing) && landing.xz().length() >= 120.0;
        Self {
            position: crossing.extend(0.0),
            velocity,
            elapsed: 0.0,
            strong,
            try_dive: false,
            try_infield_dive: false,
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
            self.runner.advance(STEP * 0.30);
        } else if !self.home_run {
            self.runner.advance(STEP);
        }
        let fielded = self.returning.is_some();
        let result = if fielded {
            self.step_throw(dt)
        } else {
            self.step_batted_ball(dt)
        };
        if result.is_none()
            && (was_running || (!fielded && self.returning.is_some()))
            && !self.runner.moving()
            && self.runner.target() < 3
            && !self.home_run
        {
            let mut advancing = self.runner;
            advancing.start_next_leg();
            if advancing.target() == 3 && self.outfield_has_not_thrown() {
                self.runner = advancing;
                return None;
            }
            if self.time_to_defend_next_base() >= advancing.remaining_time() + SAFE_ADVANCE_MARGIN {
                self.runner = advancing;
            }
        }
        result
    }

    fn outfield_has_not_thrown(&self) -> bool {
        match self.returning {
            Some(throw) => !throw.fielder.infield() && !throw.released(),
            None => {
                self.rolling
                    && self.position.xz().length_squared()
                        >= crate::defense::SECOND_BASE.length_squared()
            }
        }
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
                self.defense
                    .chase(reachable, self.velocity, self.elapsed, dt, false);
                if !self.hit_wall && self.velocity.y < 0.0 {
                    let catch = if (0.45..=2.35).contains(&reachable.y) {
                        self.defense.within_reach(reachable.xz(), true)
                    } else {
                        None
                    };
                    let catch = catch.or_else(|| {
                        self.defense
                            .dive_for(reachable, self.try_infield_dive, self.try_dive)
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
            self.velocity.y = 0.0;
            self.velocity *= GROUND_SPEED_RETENTION;
            self.rolling = true;
            return None;
        }
        self.position = next;
        None
    }

    fn roll(&mut self, dt: f32) {
        self.position += self.velocity * dt;
        let fence_radius = self.park.fence().0;
        if self.position.xz().length_squared() >= fence_radius * fence_radius {
            let stop = self.position.xz().normalize() * fence_radius;
            self.position = vec3(stop.x, BALL_RADIUS, stop.y);
            self.velocity = Vec3::ZERO;
        }
        self.defense
            .chase(self.position, self.velocity, self.elapsed, dt, true);
        if let Some(fielder) = self.defense.within_reach(self.position.xz(), false) {
            self.defense.chasing = Some(fielder);
            let base = self.runner.target().max(
                if !fielder.infield() || self.position.z < crate::defense::SECOND_BASE.y {
                    2
                } else {
                    1
                },
            );
            let urgent = self.runner.moving()
                && (self.runner.target() == base
                    || (!fielder.infield()
                        && self.position.xz().length() < self.park.fence().0 * 0.66));
            let mut throw =
                FieldReturn::with_urgency(fielder, self.position, &self.defense, base, urgent);
            // Concede the single if a throw across the diamond would be clearly late.
            if base == 1 && f64::from(throw.remaining()) > self.runner.remaining_time() + 0.20 {
                throw = FieldReturn::with_urgency(fielder, self.position, &self.defense, 2, false);
            }
            self.returning = Some(throw);
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
    fn aim_height_selects_grounder_drive_home_run_or_high_fly() {
        use crate::batting::{SWING_LEAD, Swing, grade};
        for (height, angles, distances, homer) in [
            (-0.85, -25.0..-23.0, 0.0..4.0, false),
            (-0.15, 27.0..29.0, 120.0..140.0, true),
            (0.15, 10.0..12.0, 40.0..46.0, false),
            (0.85, 57.0..60.0, 40.0..75.0, false),
        ] {
            let swing = Swing {
                time: 0.5 - SWING_LEAD,
                aim: CENTER - Vec2::Y * MEET_RADIUS * height,
            };
            let hit = grade(swing, 0.5, CENTER);
            let flight = Flight::new(CENTER, hit, Park::Central);
            let angle = flight
                .velocity
                .y
                .atan2(flight.velocity.xz().length())
                .to_degrees();
            assert!(angles.contains(&angle), "height {height}, angle {angle}");
            let carry = airborne_landing(flight.position, flight.velocity)
                .xz()
                .length();
            assert!(
                distances.contains(&carry),
                "height {height}, carry {carry} m"
            );
            for park in [Park::Central, Park::Sunset] {
                let (result, _) = simulate(1.0 / 60.0, hit, park);
                assert_eq!(result == Outcome::Homer, homer, "{park:?}, height {height}");
            }
        }
    }

    #[test]
    fn cursor_accuracy_changes_carry_at_the_same_swing_time() {
        use crate::batting::{SWING_LEAD, Swing, grade};
        for (error, expected_carry) in [(0.0, 46.0..50.0), (0.5, 31.0..35.0), (0.9, 30.0..45.0)] {
            let swing = Swing {
                time: 0.5 - SWING_LEAD,
                aim: CENTER + Vec2::X * MEET_RADIUS * error,
            };
            let hit = grade(swing, 0.5, CENTER);
            let flight = Flight::new(CENTER, hit, Park::Central);
            let carry = airborne_landing(flight.position, flight.velocity)
                .xz()
                .length();
            assert!(
                expected_carry.contains(&carry),
                "offset {error}, carry {carry} m"
            );
        }
    }

    #[test]
    fn a_liner_toward_center_is_cut_off_near_the_fielders_starting_depth() {
        use crate::batting::{SWING_LEAD, Swing, grade};
        for park in [Park::Central, Park::Sunset] {
            let swing = Swing {
                time: 0.5 - SWING_LEAD + 0.015,
                aim: CENTER,
            };
            let mut flight = Flight::new(CENTER, grade(swing, 0.5, CENTER), park);
            let starting_depth = flight.defense.position(Fielder::Center).length();
            let result = (0..1200).find_map(|_| flight.advance(STEP));
            assert_eq!(result, Some(Outcome::HardFair), "{park:?}");
            assert_eq!(flight.returning.unwrap().fielder, Fielder::Center);
            assert!(
                flight.batted_point().xz().length() <= starting_depth + 5.0,
                "{park:?}: {:?}",
                flight.batted_point()
            );
        }
    }

    #[test]
    fn on_time_drives_can_split_either_outfield_gap_and_stay_inside_the_fence() {
        use crate::batting::{SWING_LEAD, Swing, grade};
        for park in [Park::Central, Park::Sunset] {
            for timing in [-0.030, 0.030] {
                let swing = Swing {
                    time: 0.5 - SWING_LEAD + timing,
                    aim: CENTER + Vec2::Y * 0.004,
                };
                let hit = grade(swing, 0.5, CENTER);
                assert_eq!(hit.contact, Contact::Barrel);
                let mut flight = Flight::new(CENTER, hit, park);
                let outcome = (0..1200).find_map(|_| flight.advance(STEP)).unwrap();
                assert!(
                    matches!(outcome, Outcome::HardFair | Outcome::Double),
                    "{park:?} {timing}: {outcome:?}"
                );
                assert!(!flight.returning.unwrap().fielder.infield());
                let pickup = flight.first_pickup.unwrap().xz();
                assert!(
                    pickup.length() <= park.fence().0 + 0.001,
                    "{park:?}: {pickup:?}"
                );
                assert!(pickup.x * timing.signum() as f32 > 5.0);
                assert!(!flight.hit_wall);
            }
        }
    }

    #[test]
    fn a_big_homer_walks_while_the_ball_travels_at_full_tempo() {
        let mut flight = Flight::new(
            CENTER,
            Grade {
                contact: Contact::Barrel,
                timing: 0.0,
                offset: Vec2::Y * 0.015,
            },
            Park::Central,
        );
        assert!(flight.confident);
        flight.advance(0.5);
        assert!(flight.runner.position().length() > 0.5);
        assert!(flight.runner.position().length() < 2.0);
        assert!(flight.position.z < -28.0);
    }

    #[test]
    fn ideal_contact_clears_the_fence_at_any_render_rate() {
        let ideal = Grade {
            contact: Contact::Barrel,
            timing: 0.0,
            offset: Vec2::Y * 0.015,
        };
        let (a, p) = simulate(1.0 / 30.0, ideal, Park::Central);
        let (b, q) = simulate(1.0 / 144.0, ideal, Park::Central);
        assert_eq!(a, Outcome::Homer);
        assert_eq!(a, b);
        assert_eq!(p, q);
        assert!((p.y - BALL_RADIUS).abs() < 1e-5);
        assert!(p.xz().length() > Park::Central.fence().0 + 25.0);
    }

    #[test]
    fn confident_homers_land_near_120_to_140_metres_in_both_parks() {
        use crate::batting::{SWING_LEAD, Swing, grade};
        for park in [Park::Central, Park::Sunset] {
            let mut longest = 0.0_f32;
            let mut shortest = f32::INFINITY;
            for crossing in [CENTER, vec2(0.12, 0.60), vec2(-0.12, 1.15)] {
                for y in -30..=30 {
                    for ms in -5..=5 {
                        let timing = f64::from(ms) / 1000.0;
                        let swing = Swing {
                            time: 0.5 - SWING_LEAD + timing,
                            aim: crossing - Vec2::Y * (y as f32 * 0.002),
                        };
                        let hit = grade(swing, 0.5, crossing);
                        let mut flight = Flight::new(crossing, hit, park);
                        if flight.confident {
                            let outcome = (0..1200).find_map(|_| flight.advance(STEP));
                            assert_eq!(outcome, Some(Outcome::Homer));
                            let distance = flight.position.xz().length();
                            longest = longest.max(distance);
                            shortest = shortest.min(distance);
                        }
                    }
                }
            }
            assert!((125.0..=140.0).contains(&longest), "{park:?}: {longest} m");
            assert!(shortest >= 120.0, "{park:?}: {shortest} m");
        }
    }

    #[test]
    fn clearing_the_fence_keeps_the_ball_in_flight_until_landing() {
        let ideal = Grade {
            contact: Contact::Barrel,
            timing: 0.0,
            offset: Vec2::Y * 0.015,
        };
        let mut flight = Flight::new(CENTER, ideal, Park::Central);
        for _ in 0..240 {
            assert!(flight.advance(STEP).is_none());
            if flight.home_run {
                assert!(flight.position.y > Park::Central.fence().1);
                assert!(flight.position.xz().length() >= Park::Central.fence().0);
                let result = (0..1200).find_map(|_| flight.advance(STEP));
                assert_eq!(result, Some(Outcome::Homer));
                assert!(flight.position.xz().length() >= 120.0);
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
            (Park::Central, Outcome::BaseOut(Fielder::Third)),
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
                (0.052, Vec2::ZERO, Contact::Touch),
                (0.0, vec2(0.064, 0.0), Contact::Barrel),
                (0.0, vec2(0.066, 0.0), Contact::Touch),
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
                if timing == 0.0 && offset == Vec2::ZERO {
                    assert_ne!(outcome, Outcome::Homer);
                } else if expected_contact == Contact::Touch {
                    assert!(matches!(
                        outcome,
                        Outcome::Fair
                            | Outcome::Double
                            | Outcome::Triple
                            | Outcome::FlyOut(_)
                            | Outcome::GroundOut(_)
                            | Outcome::BaseOut(_)
                    ));
                    assert!(end.xz().length() <= park.fence().0 + 1e-4);
                }
            }
        }
    }

    #[test]
    fn right_fielder_breaks_back_at_contact_and_catches_a_high_fly() {
        for park in [Park::Central, Park::Sunset] {
            let mut flight = Flight::new(
                CENTER,
                Grade {
                    contact: Contact::Touch,
                    timing: 0.0,
                    offset: Vec2::ZERO,
                },
                park,
            );
            flight.velocity = vec3(13.5, 17.0, -20.0);
            let start = flight.defense.position(Fielder::Right);
            assert_eq!(flight.advance(STEP), None);
            assert!(flight.defense.position(Fielder::Right).length() > start.length());
            assert_eq!(
                (0..1200).find_map(|_| flight.advance(STEP)),
                Some(Outcome::FlyOut(Fielder::Right))
            );
        }
    }

    #[test]
    fn left_field_backup_holds_grounders_through_the_infield_to_singles() {
        use crate::batting::{SWING_LEAD, Swing, grade};
        for park in [Park::Central, Park::Sunset] {
            for (timing, aim_height, expected, base) in [
                (-0.070, 0.035, Outcome::Fair, crate::defense::FIRST_BASE),
                (-0.055, 0.035, Outcome::Fair, crate::defense::FIRST_BASE),
            ] {
                let swing = Swing {
                    time: 0.5 - SWING_LEAD + timing,
                    aim: CENTER + Vec2::Y * aim_height,
                };
                let mut flight = Flight::new(CENTER, grade(swing, 0.5, CENTER), park);
                assert_eq!(
                    (0..1400).find_map(|_| flight.advance(STEP)),
                    Some(expected),
                    "{park:?}, timing {timing}"
                );
                assert!(!flight.returning.unwrap().fielder.infield());
                assert_eq!(flight.runner.position(), base);
            }
        }
    }

    #[test]
    fn an_infield_gap_grounder_stays_a_single_and_a_deep_fly_can_be_a_double() {
        use crate::batting::{SWING_LEAD, Swing, grade};
        for park in [Park::Central, Park::Sunset] {
            for (timing, aim_offset, expected) in [
                (-0.065, vec2(0.0, 0.04), Outcome::Fair),
                (
                    -0.037,
                    vec2(0.02, 0.025),
                    if park == Park::Sunset {
                        Outcome::BaseOut(Fielder::Third)
                    } else {
                        Outcome::Double
                    },
                ),
            ] {
                let swing = Swing {
                    time: 0.5 - SWING_LEAD + timing,
                    aim: CENTER + aim_offset,
                };
                let mut flight = Flight::new(CENTER, grade(swing, 0.5, CENTER), park);
                assert_eq!(
                    (0..1200).find_map(|_| flight.advance(STEP)),
                    Some(expected),
                    "{park:?}"
                );
                let landing = flight.ground_contact.unwrap().position.xz().length();
                if expected == Outcome::Fair {
                    assert!(landing < 35.0);
                    assert_eq!(flight.runner.position(), crate::defense::FIRST_BASE);
                } else {
                    assert!(landing >= 70.0);
                    assert!(flight.runner.reached() >= 2);
                }
                assert!(flight.first_pickup.unwrap().xz().length() > 38.0);
            }
        }
    }

    #[test]
    fn center_fielder_retreats_from_shallow_alignment_to_catch_a_high_fly() {
        use crate::batting::{SWING_LEAD, Swing, grade};
        for park in [Park::Central, Park::Sunset] {
            let swing = Swing {
                time: 0.5 - SWING_LEAD,
                aim: CENTER - Vec2::Y * 0.075,
            };
            let mut flight = Flight::new(CENTER, grade(swing, 0.5, CENTER), park);
            let initial_depth = flight.defense.position(Fielder::Center).length();
            let result = (0..1200).find_map(|_| flight.advance(STEP));
            assert_eq!(result, Some(Outcome::FlyOut(Fielder::Center)), "{park:?}");
            assert!(flight.defense.position(Fielder::Center).length() > initial_depth + 6.0);
            assert!(flight.ground_contact.is_none());
        }
    }

    #[test]
    fn a_caught_fly_is_out_and_a_shallow_outfield_hit_stops_at_first() {
        for (position, velocity, expected) in [
            (
                vec3(3.8, 7.5, -53.0),
                vec3(0.0, -2.0, -7.0),
                Outcome::FlyOut(Fielder::Center),
            ),
            (vec3(29.0, 0.6, -42.0), vec3(0.0, -2.0, -5.0), Outcome::Fair),
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
            flight.runner.advance(f64::from(flight.elapsed));
            let mut picked_up_at = None;
            let outcome = (0..1000).find_map(|_| {
                let result = flight.advance(STEP);
                if picked_up_at.is_none() && flight.first_pickup.is_some() {
                    picked_up_at = Some(flight.elapsed);
                }
                result
            });
            assert_eq!(outcome, Some(expected));
            if expected == Outcome::Fair {
                assert_eq!(flight.runner.target(), 1);
                assert_eq!(flight.runner.position(), crate::defense::FIRST_BASE);
                let contact = flight
                    .ground_contact
                    .expect("a hit should record its first landing");
                assert!(picked_up_at.unwrap() - contact.at <= 0.8);
                assert!((contact.position.y - BALL_RADIUS).abs() < 1e-5);
                assert!(contact.position.xz().distance(flight.batted_point().xz()) > 0.0);
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
    fn shortstop_can_catch_at_full_reach_or_dive_for_a_lower_ball() {
        for (gap, allow_dive, caught) in
            [(1.5, false, true), (2.25, false, false), (2.25, true, true)]
        {
            let mut flight = Flight::new(
                CENTER,
                Grade {
                    contact: Contact::Touch,
                    timing: 0.0,
                    offset: Vec2::ZERO,
                },
                Park::Central,
            );
            let shortstop = flight.defense.position(Fielder::Shortstop);
            flight.elapsed = 1.0;
            flight.position = vec3(shortstop.x - gap, 0.8, shortstop.y);
            flight.velocity = vec3(0.0, -4.0, 0.0);
            flight.try_infield_dive = allow_dive;
            assert_eq!(
                flight.advance(STEP),
                caught.then_some(Outcome::FlyOut(Fielder::Shortstop))
            );
            assert_eq!(
                flight.defense.diving,
                allow_dive.then_some(Fielder::Shortstop)
            );
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
            let center = flight.defense.position(Fielder::Center);
            flight.position = vec3(center.x + gap, height, center.y);
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
            (0.7, Outcome::GroundOut(Fielder::Second)),
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
            let throw = flight.returning.unwrap();
            assert_eq!(throw.base, if expected == Outcome::Fair { 2 } else { 1 });
            assert!(flight.position.xz().abs_diff_eq(throw.target, 0.01));
            assert!(
                flight
                    .defense
                    .position(throw.receiver)
                    .abs_diff_eq(throw.target, 0.01)
            );
        }
    }

    #[test]
    fn a_grounder_fielded_by_third_can_be_an_infield_hit() {
        use crate::batting::{SWING_LEAD, Swing, grade};
        let swing = Swing {
            time: 0.5 - SWING_LEAD - 0.0936,
            aim: CENTER + vec2(0.072, 0.020),
        };
        let mut flight = Flight::new(CENTER, grade(swing, 0.5, CENTER), Park::Central);
        let result = (0..1200).find_map(|step| {
            if step % 24 == 0 && flight.can_boost_runner() {
                flight.runner.tap();
            }
            flight.advance(STEP)
        });
        assert_eq!(result, Some(Outcome::Fair));
        let throw = flight.returning.unwrap();
        assert_eq!((throw.fielder, throw.base), (Fielder::Third, 1));
        assert_eq!(flight.runner.position(), crate::defense::FIRST_BASE);
    }

    #[test]
    fn reaching_second_before_the_outfield_return_triggers_a_try_for_third() {
        for released in [None, Some(false), Some(true)] {
            let mut flight = Flight::new(
                CENTER,
                Grade {
                    contact: Contact::Touch,
                    timing: 0.0,
                    offset: Vec2::ZERO,
                },
                Park::Central,
            );
            flight.runner.advance(flight.runner.remaining_time());
            flight.runner.start_next_leg();
            flight
                .runner
                .advance(flight.runner.remaining_time() - STEP * 0.5);
            flight.position = vec3(0.0, BALL_RADIUS, -72.0);
            flight.rolling = true;
            flight.elapsed = 5.0;
            if let Some(released) = released {
                flight.defense.positions[Fielder::Center as usize] = flight.position.xz();
                let mut returning = FieldReturn::with_urgency(
                    Fielder::Center,
                    flight.position,
                    &flight.defense,
                    2,
                    false,
                );
                if released {
                    returning.advance(&mut flight.defense, 0.8);
                    assert!(returning.airborne());
                }
                flight.returning = Some(returning);
            }

            assert!(flight.advance(STEP).is_none());
            let target = if released == Some(true) { 2 } else { 3 };
            assert_eq!(flight.runner.target(), target);
            let result = (0..1200).find_map(|_| flight.advance(STEP)).unwrap();
            if target == 3 {
                assert!(matches!(result, Outcome::Triple | Outcome::BaseOut(_)));
            } else {
                assert_eq!(result, Outcome::Double);
            }
        }
    }

    #[test]
    fn a_committed_runner_contests_the_next_base_after_a_relay() {
        for (extra_time, expected) in [
            (0.0, Outcome::BaseOut(Fielder::Third)),
            (2.4, Outcome::Triple),
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
            while flight.runner.target() < 3 {
                flight.runner.advance(flight.runner.remaining_time());
                flight.runner.start_next_leg();
            }
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
        for base in 1..=3 {
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
                while flight.runner.target() < base {
                    flight.runner.advance(flight.runner.remaining_time());
                    flight.runner.start_next_leg();
                }
                flight
                    .runner
                    .advance(flight.runner.remaining_time() - runner_time);
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
    fn close_pitcher_flip_is_decided_by_the_race_to_first() {
        for park in [Park::Central, Park::Sunset] {
            for (runner_time, expected) in [
                (1.20, Outcome::GroundOut(Fielder::Pitcher)),
                (1.48, Outcome::Fair),
            ] {
                let mut flight = Flight::new(
                    CENTER,
                    Grade {
                        contact: Contact::Touch,
                        timing: 0.0,
                        offset: Vec2::ZERO,
                    },
                    park,
                );
                flight.rolling = true;
                flight.position = vec3(4.0, BALL_RADIUS, -18.0);
                flight.velocity = vec3(1.5, 0.0, -1.2);
                flight.defense.positions[Fielder::Pitcher as usize] = flight.position.xz();
                flight.elapsed = 1.48;
                flight.runner.advance(runner_time);

                let outcome = (0..1200).find_map(|_| flight.advance(STEP));
                assert_eq!(outcome, Some(expected), "{park:?}");
                let play = flight.returning.unwrap();
                assert_eq!(play.base, 1);
                assert!(play.underhand());
            }
        }
    }

    #[test]
    fn pitcher_covers_first_while_first_baseman_chases_the_ball() {
        use crate::batting::{SWING_LEAD, Swing, grade};
        let crossing = vec2(-0.2, CENTER.y);
        let hit = grade(
            Swing {
                time: 0.5 - SWING_LEAD + 0.065,
                aim: crossing + vec2(0.0, 0.020),
            },
            0.5,
            crossing,
        );
        let mut flight = Flight::new(crossing, hit, Park::Central);
        let mut covered_before_pickup = false;
        let result = (0..1200).find_map(|_| {
            let result = flight.advance(STEP);
            if flight.returning.is_none() && flight.defense.chasing == Some(Fielder::First) {
                covered_before_pickup |= flight.defense.position(Fielder::Pitcher).x > 0.5;
            }
            result
        });
        assert!(covered_before_pickup);
        assert_eq!(result, Some(Outcome::GroundOut(Fielder::First)));
    }

    #[test]
    fn grounders_and_airborne_hits_slow_at_landing_then_roll_at_constant_speed() {
        for (height, vertical_speed) in [(0.08, -4.0), (0.20, -15.0)] {
            let mut flight = Flight::new(
                CENTER,
                Grade {
                    contact: Contact::Touch,
                    timing: 0.0,
                    offset: Vec2::ZERO,
                },
                Park::Central,
            );
            flight.position = vec3(2.0, height, -2.0);
            flight.velocity = vec3(2.0, vertical_speed, -25.0);
            let expected = air_velocity(flight.velocity, STEP as f32 * BALL_TEMPO).xz()
                * GROUND_SPEED_RETENTION;
            assert!(flight.advance(STEP).is_none());
            assert!(flight.ground_contact.is_some());
            let landed = flight.position;
            assert_eq!(flight.velocity.xz(), expected);
            assert_eq!(flight.velocity.y, 0.0);

            assert!(flight.advance(10.0 * STEP).is_none());
            assert_eq!(flight.velocity.xz(), expected);
            assert!(
                flight
                    .position
                    .xz()
                    .abs_diff_eq(landed.xz() + expected * (10.0 * STEP) as f32, 1e-5)
            );
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
            timing: -0.106,
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
                timing: -0.106,
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
