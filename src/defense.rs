use macroquad::prelude::*;

use crate::park::Park;

pub const MOUND: Vec3 = vec3(0.0, 0.0, -17.0);
pub const FIRST_BASE: Vec2 = vec2(19.4, -19.4);
pub const SECOND_BASE: Vec2 = vec2(0.0, -38.8);
pub const THIRD_BASE: Vec2 = vec2(-19.4, -19.4);

pub const PLAYER_SPEED: f32 = 11.0;
const INFIELDER_THROW_SPEED: f32 = 34.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fielder {
    First,
    Second,
    Shortstop,
    Third,
    Left,
    Center,
    Right,
    Pitcher,
}

impl Fielder {
    pub const ALL: [Self; 8] = [
        Self::First,
        Self::Second,
        Self::Shortstop,
        Self::Third,
        Self::Left,
        Self::Center,
        Self::Right,
        Self::Pitcher,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Pitcher => "投手",
            Self::First => "一塁手",
            Self::Second => "二塁手",
            Self::Shortstop => "遊撃手",
            Self::Third => "三塁手",
            Self::Left => "左翼手",
            Self::Center => "中堅手",
            Self::Right => "右翼手",
        }
    }

    pub fn infield(self) -> bool {
        matches!(
            self,
            Self::Pitcher | Self::First | Self::Second | Self::Shortstop | Self::Third
        )
    }
}

#[derive(Clone, Copy)]
pub struct Defense {
    pub positions: [Vec2; Fielder::ALL.len()],
    pub moving: [bool; Fielder::ALL.len()],
    pub chasing: Option<Fielder>,
    pub diving: Option<Fielder>,
}

impl Defense {
    pub fn new(park: Park) -> Self {
        let outfield_scale = park.fence().0 / Park::Central.fence().0;
        Self {
            positions: [
                vec2(15.0, -21.0),
                vec2(9.0, -28.0),
                vec2(-9.0, -28.0),
                vec2(-15.0, -21.0),
                vec2(-31.0, -44.0) * outfield_scale,
                vec2(0.0, -56.0) * outfield_scale,
                vec2(31.0, -44.0) * outfield_scale,
                MOUND.xz(),
            ],
            moving: [false; Fielder::ALL.len()],
            chasing: None,
            diving: None,
        }
    }

    pub fn chase(&mut self, position: Vec3, velocity: Vec3, elapsed: f32, dt: f32, rolling: bool) {
        let before = self.positions;
        let fly_ball = !rolling && position.y + velocity.y.max(0.0).powi(2) / 19.62 >= 5.5;
        let lead = if rolling {
            // Keep the target within pickup range so the fielder cannot shadow the ball forever.
            0.22_f32.min(0.75 / velocity.xz().length().max(0.01))
        } else {
            let height = (position.y - 1.25).max(0.0);
            ((velocity.y + (velocity.y * velocity.y + 19.62 * height).sqrt()) / 9.81)
                .clamp(0.0, 1.7)
                * 0.90
        };
        let direction = velocity.xz().normalize_or_zero();
        let corner_fly = fly_ball && direction.x.abs() >= 0.45 && direction.y < 0.0;
        let target = if corner_fly {
            // Include air drag when looking beyond the usual short intercept horizon.
            let height = position.y - 1.25;
            let hang =
                (velocity.y + (velocity.y * velocity.y + 19.62 * height).max(0.0).sqrt()) / 9.81;
            position.xz() + velocity.xz() * hang / (1.0 + 0.0023 * velocity.length() * hang)
        } else {
            position.xz() + velocity.xz() * lead
        };
        let depth = if rolling { position.z } else { target.y };
        let fielder = Fielder::ALL
            .into_iter()
            .filter(|&fielder| {
                // Infielders can reach just behind their spot without chasing into the outfield.
                !fielder.infield() || depth >= self.position(fielder).y - 3.0
            })
            .min_by(|&a, &b| {
                self.position(a)
                    .distance_squared(target)
                    .total_cmp(&self.position(b).distance_squared(target))
            })
            .expect("there is always an outfielder");
        // React early to high flies aimed near a defensive lane. Flies into the
        // alleys still require lateral coverage from the shallow alignment.
        let bearing = target.x.abs().atan2(-target.y);
        let direct_fly = fly_ball && (bearing <= 0.19 || (0.45..=0.71).contains(&bearing));
        let speed = match fielder {
            Fielder::Pitcher => PLAYER_SPEED,
            _ if fielder.infield() => 12.0,
            _ if direct_fly || corner_fly => 12.5,
            _ if !rolling && target.length() > self.position(fielder).length() + 1.0 => 8.0,
            _ if rolling => 14.0,
            _ => 11.5,
        };
        let spot = self.position(fielder);
        let low_drive = velocity.y.abs() <= velocity.xz().length() * 0.35;
        let target = if !fielder.infield() && (rolling || low_drive) && !corner_fly {
            // On a ball rolling toward the fielder, get in its path at the current depth.
            let direction = velocity.xz().normalize_or_zero();
            let ahead = (spot - position.xz()).dot(direction);
            let intercept = position.xz() + direction * ahead;
            let predicted = (target - position.xz()).dot(direction);
            if ahead > 0.0
                && (rolling || ahead >= predicted)
                && spot.distance_squared(intercept) <= 25.0
            {
                intercept
            } else {
                target
            }
        } else {
            target
        };
        let startup = if fielder.infield() || direct_fly || rolling {
            0.40
        } else {
            0.62
        };
        let stride = (elapsed / startup).clamp(0.0, 1.0);
        let pursuit_stride = if corner_fly { 1.0 } else { stride.max(0.30) };
        self.move_toward(fielder, target, speed * pursuit_stride * dt);
        if fielder.infield() && direction.y < 0.0 {
            // Back up the infielder along the ball's path at outfield depth.
            let backup = [Fielder::Left, Fielder::Center, Fielder::Right]
                .into_iter()
                .min_by(|&a, &b| {
                    let miss = |f| (self.position(f) - position.xz()).perp_dot(direction).abs();
                    miss(a).total_cmp(&miss(b))
                })
                .unwrap();
            let spot = self.position(backup);
            let ahead = (spot - position.xz()).dot(direction);
            if ahead > 0.0 {
                self.move_toward(backup, position.xz() + direction * ahead, 10.0 * dt);
            }
        }
        self.cover_bases(fielder, None, dt * stride);
        for role in Fielder::ALL {
            let spot = self.position(role);
            if role != fielder && spot == before[role as usize] {
                if !role.infield() {
                    // Converge behind the primary catch point, even while an infielder is chasing.
                    let depth = spot.length().max(target.length() + 5.0);
                    let backup = direction * depth;
                    self.move_toward(role, backup, 5.5 * dt);
                } else if !matches!(role, Fielder::First | Fielder::Third)
                    && spot != SECOND_BASE
                    && spot != FIRST_BASE
                {
                    let relay = target.clamp_length_max(32.0);
                    self.move_toward(role, relay, 2.0 * dt);
                }
            }
            self.moving[role as usize] = self.position(role) != before[role as usize];
        }
        self.chasing = Some(fielder);
    }

    fn cover_bases(&mut self, fielder: Fielder, receiver: Option<Fielder>, dt: f32) {
        let second = match fielder {
            Fielder::Shortstop => Fielder::Second,
            Fielder::Second => Fielder::Shortstop,
            _ => self.second_base_cover(),
        };
        let first = if fielder == Fielder::First {
            Fielder::Pitcher
        } else {
            Fielder::First
        };
        for (cover, bag) in [
            (first, FIRST_BASE),
            (second, SECOND_BASE),
            (Fielder::Third, THIRD_BASE),
        ] {
            if cover != fielder && Some(cover) != receiver {
                self.move_toward(cover, bag, PLAYER_SPEED * dt);
            }
        }
    }

    pub fn position(&self, fielder: Fielder) -> Vec2 {
        self.positions[fielder as usize]
    }

    fn move_toward(&mut self, fielder: Fielder, target: Vec2, distance: f32) {
        let position = &mut self.positions[fielder as usize];
        let toward = target - *position;
        *position += toward.clamp_length_max(distance);
    }

    pub fn within_reach(&self, position: Vec2, airborne: bool) -> Option<Fielder> {
        Fielder::ALL
            .into_iter()
            .filter_map(|fielder| {
                let reach: f32 = match (fielder.infield(), airborne) {
                    (true, true) => 1.60,
                    (true, false) => 1.80,
                    (false, true) => 1.04,
                    (false, false) => 1.65,
                };
                let distance = self.position(fielder).distance_squared(position);
                (distance <= reach * reach).then_some((fielder, distance))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(fielder, _)| fielder)
    }

    pub fn dive_for(&mut self, ball: Vec3, infield: bool, outfield: bool) -> Option<Fielder> {
        let point = ball.xz();
        let infielder = if infield && (0.25..=1.25).contains(&ball.y) {
            [
                Fielder::First,
                Fielder::Second,
                Fielder::Shortstop,
                Fielder::Third,
            ]
            .into_iter()
            .filter_map(|fielder| {
                let distance = self.position(fielder).distance_squared(point);
                (distance <= 2.55_f32.powi(2)).then_some((fielder, distance))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(fielder, _)| fielder)
        } else {
            None
        };
        let diver = infielder.or_else(|| {
            self.chasing.filter(|&fielder| {
                outfield
                    && !fielder.infield()
                    && (0.32..=1.05).contains(&ball.y)
                    && (0.85..=2.15).contains(&self.position(fielder).distance(point))
            })
        });
        self.diving = diver;
        diver
    }

    fn second_base_cover(&self) -> Fielder {
        if self.position(Fielder::Second).distance_squared(SECOND_BASE)
            < self
                .position(Fielder::Shortstop)
                .distance_squared(SECOND_BASE)
        {
            Fielder::Second
        } else {
            Fielder::Shortstop
        }
    }
}

#[derive(Clone, Copy)]
pub struct FieldReturn {
    pub fielder: Fielder,
    pub receiver: Fielder,
    pub pickup: Vec3,
    pub target: Vec2,
    pub base: usize,
    release: Vec3,
    underhand: bool,
    transfer: f32,
    travel: f32,
    elapsed: f32,
}

impl FieldReturn {
    pub fn new(fielder: Fielder, pickup: Vec3, defense: &Defense, base: usize) -> Self {
        Self::with_urgency(fielder, pickup, defense, base, true)
    }

    pub fn with_urgency(
        fielder: Fielder,
        pickup: Vec3,
        defense: &Defense,
        base: usize,
        urgent: bool,
    ) -> Self {
        let (receiver, target) = match base {
            1 => {
                let first_distance = defense.position(Fielder::First).distance(FIRST_BASE);
                let pitcher_distance = defense.position(Fielder::Pitcher).distance(FIRST_BASE);
                let receiver = if fielder == Fielder::First
                    && first_distance > 2.0
                    && pitcher_distance < first_distance
                {
                    Fielder::Pitcher
                } else {
                    Fielder::First
                };
                (receiver, FIRST_BASE)
            }
            2 => (defense.second_base_cover(), SECOND_BASE),
            3 => (Fielder::Third, THIRD_BASE),
            _ => unreachable!("there are three playable bases"),
        };
        let start = defense.position(fielder);
        let toward = (target - start).normalize_or_zero();
        let distance = start.distance(target);
        // A pitcher who fields toward first can flip the ball from nearby.
        let underhand =
            fielder == Fielder::Pitcher && base == 1 && start.x >= 3.0 && distance <= 16.0;
        let release_height = if underhand { 0.78 } else { 1.65 };
        let release = on_field(start + toward * 0.30, release_height);
        let carrying = fielder == receiver;
        let (throw_speed, exchange) = if underhand {
            (19.0, 0.23)
        } else if fielder.infield() {
            (INFIELDER_THROW_SPEED, 0.28)
        } else {
            // Distance calls for more arm even on a routine return. A contested base
            // calls for a quicker transfer and extra velocity.
            let distance_factor = ((distance - 12.0) / 26.0).clamp(0.0, 1.0);
            if urgent {
                (32.0 + 10.0 * distance_factor, 0.38)
            } else {
                (18.0 + 14.0 * distance_factor, 0.60)
            }
        };
        let travel = if carrying {
            start.distance(target) / PLAYER_SPEED
        } else {
            (release.xz().distance(target) / throw_speed).max(0.12)
        };
        let transfer = if carrying {
            0.18
        } else {
            let cover = defense.position(receiver).distance(target) / PLAYER_SPEED;
            (cover - travel).max(exchange)
        };
        Self {
            fielder,
            receiver,
            pickup,
            target,
            base,
            release,
            underhand,
            transfer,
            travel,
            elapsed: 0.0,
        }
    }

    pub fn advance(&mut self, defense: &mut Defense, dt: f32) -> bool {
        let moved = if self.carrying() {
            (self.elapsed + dt - self.transfer).clamp(0.0, dt)
        } else {
            dt
        };
        defense.cover_bases(self.fielder, Some(self.receiver), dt);
        defense.move_toward(self.receiver, self.target, PLAYER_SPEED * moved);
        self.elapsed += dt;
        self.arrived()
    }

    pub fn arrived(&self) -> bool {
        self.elapsed >= self.transfer + self.travel
    }

    pub fn remaining(&self) -> f32 {
        (self.transfer + self.travel - self.elapsed).max(0.0)
    }

    pub fn carrying(&self) -> bool {
        self.fielder == self.receiver
    }

    pub fn target_label(&self) -> &'static str {
        match self.base {
            1 => "一塁",
            2 => "二塁",
            _ => "三塁",
        }
    }

    pub fn underhand(&self) -> bool {
        self.underhand
    }

    pub fn transfer_progress(&self) -> f32 {
        (self.elapsed / self.transfer).min(1.0)
    }

    pub fn released(&self) -> bool {
        self.elapsed >= self.transfer
    }

    pub fn airborne(&self) -> bool {
        !self.carrying()
            && self.elapsed >= self.transfer
            && self.elapsed < self.transfer + self.travel
    }

    pub fn ball_position(&self, defense: &Defense) -> Vec3 {
        if self.elapsed < self.transfer {
            let ready = if self.carrying() {
                on_field(defense.position(self.fielder), 1.1)
            } else {
                self.release
            };
            return self.pickup.lerp(ready, self.transfer_progress());
        }
        if self.carrying() {
            return on_field(defense.position(self.fielder), 1.1);
        }
        let fraction = ((self.elapsed - self.transfer) / self.travel).clamp(0.0, 1.0);
        let distance = self.release.xz().distance(self.target);
        let arc = if self.underhand {
            (distance * 0.035).clamp(0.35, 0.75)
        } else {
            (distance * 0.05).clamp(0.3, 2.8)
        };
        self.release.lerp(on_field(self.target, 1.1), fraction)
            + Vec3::Y * (4.0 * fraction * (1.0 - fraction) * arc)
    }
}

fn on_field(point: Vec2, height: f32) -> Vec3 {
    vec3(point.x, height, point.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_outfielders_break_together_on_both_gap_flies() {
        for (side, corner) in [(-1.0, Fielder::Left), (1.0, Fielder::Right)] {
            let mut defense = Defense::new(Park::Central);
            let before = defense;
            let dt = 1.0 / 120.0;
            defense.chase(
                vec3(0.0, 1.0, -0.1),
                vec3(side * 10.0, 14.0, -28.0),
                dt,
                dt,
                false,
            );
            for role in [corner, Fielder::Center] {
                assert!(defense.moving[role as usize]);
                assert!(
                    defense.position(role).distance(vec2(side * 21.0, -64.0))
                        < before.position(role).distance(vec2(side * 21.0, -64.0))
                );
            }
        }
    }

    #[test]
    fn outfield_return_speed_increases_with_distance_and_urgency() {
        let mut defense = Defense::new(Park::Central);
        defense.positions[Fielder::Shortstop as usize] = SECOND_BASE;
        let mut speeds = Vec::new();

        for distance in [12.0, 42.0] {
            let position = SECOND_BASE - Vec2::Y * distance;
            defense.positions[Fielder::Center as usize] = position;
            let pickup = on_field(position, 0.0);
            let relaxed = FieldReturn::with_urgency(Fielder::Center, pickup, &defense, 2, false);
            let urgent = FieldReturn::with_urgency(Fielder::Center, pickup, &defense, 2, true);
            let travel_distance = relaxed.release.xz().distance(SECOND_BASE);
            let relaxed_speed = travel_distance / relaxed.travel;
            let urgent_speed = travel_distance / urgent.travel;

            assert!(urgent.remaining() < relaxed.remaining());
            assert!(urgent_speed > relaxed_speed);
            speeds.push((relaxed_speed, urgent_speed));
        }

        assert!(speeds[0].0 < 20.0);
        assert!(speeds[0].0 < speeds[1].0);
        assert!(speeds[0].1 < speeds[1].1);
        assert!(speeds[1].1 > 38.0);
    }
}
