use macroquad::prelude::*;

use crate::park::Park;

pub const FIRST_BASE: Vec2 = vec2(19.4, -19.4);
pub const SECOND_BASE: Vec2 = vec2(0.0, -38.8);
pub const THIRD_BASE: Vec2 = vec2(-19.4, -19.4);

const INFIELDER_SPEED: f32 = 6.0;
const OUTFIELDER_SPEED: f32 = 6.25;
const INFIELDER_THROW_SPEED: f32 = 27.0;
const OUTFIELDER_THROW_SPEED: f32 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fielder {
    First,
    Second,
    Shortstop,
    Third,
    Left,
    Center,
    Right,
}

impl Fielder {
    pub const ALL: [Self; 7] = [
        Self::First,
        Self::Second,
        Self::Shortstop,
        Self::Third,
        Self::Left,
        Self::Center,
        Self::Right,
    ];

    pub fn label(self) -> &'static str {
        match self {
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
            Self::First | Self::Second | Self::Shortstop | Self::Third
        )
    }
}

#[derive(Clone, Copy)]
pub struct Defense {
    pub positions: [Vec2; 7],
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
                vec2(-31.0, -49.0) * outfield_scale,
                vec2(0.0, -62.0) * outfield_scale,
                vec2(31.0, -49.0) * outfield_scale,
            ],
            chasing: None,
            diving: None,
        }
    }

    pub fn chase(
        &mut self,
        position: Vec3,
        velocity: Vec3,
        elapsed: f32,
        dt: f32,
        rolling: bool,
        runner_target: usize,
    ) {
        if elapsed < 0.27 {
            return;
        }
        let lead = if rolling {
            0.22
        } else {
            let height = (position.y - 1.25).max(0.0);
            ((velocity.y + (velocity.y * velocity.y + 19.62 * height).sqrt()) / 9.81)
                .clamp(0.0, 1.7)
                * 0.90
        };
        let target = position.xz() + velocity.xz() * lead;
        let index = self.closest(target);
        let fielder = Fielder::ALL[index];
        if !fielder.infield() && elapsed < 0.34 {
            return;
        }
        let speed = if fielder.infield() {
            INFIELDER_SPEED
        } else {
            OUTFIELDER_SPEED
        };
        self.move_toward(fielder, target, speed * dt);
        if rolling && fielder != Fielder::First {
            self.move_toward(Fielder::First, FIRST_BASE, INFIELDER_SPEED * dt);
        }
        if rolling && !fielder.infield() {
            self.move_toward(self.second_base_cover(), SECOND_BASE, INFIELDER_SPEED * dt);
            if runner_target >= 2 {
                self.move_toward(Fielder::Third, THIRD_BASE, INFIELDER_SPEED * dt);
            }
        }
        self.chasing = Some(fielder);
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
                    (true, true) => 1.30,
                    (true, false) => 1.20,
                    (false, _) => 0.94,
                };
                let distance = self.position(fielder).distance_squared(position);
                (distance <= reach * reach).then_some((fielder, distance))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(fielder, _)| fielder)
    }

    pub fn dive_for(&mut self, ball: Vec3) -> Option<Fielder> {
        let fielder = self.chasing?;
        let distance = self.position(fielder).distance(ball.xz());
        if (0.32..=1.05).contains(&ball.y) && (0.85..=2.15).contains(&distance) {
            self.diving = Some(fielder);
            Some(fielder)
        } else {
            None
        }
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

    fn closest(&self, target: Vec2) -> usize {
        (0..self.positions.len())
            .min_by(|&a, &b| {
                self.positions[a]
                    .distance_squared(target)
                    .total_cmp(&self.positions[b].distance_squared(target))
            })
            .expect("there is always a fielder")
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
    transfer: f32,
    travel: f32,
    elapsed: f32,
}

impl FieldReturn {
    pub fn new(fielder: Fielder, pickup: Vec3, defense: &Defense, base: usize) -> Self {
        let (receiver, target) = match base {
            1 => (Fielder::First, FIRST_BASE),
            2 => (defense.second_base_cover(), SECOND_BASE),
            3 => (Fielder::Third, THIRD_BASE),
            _ => unreachable!("there are three playable bases"),
        };
        let start = defense.position(fielder);
        let toward = (target - start).normalize_or_zero();
        let release = on_field(start + toward * 0.30, 1.65);
        let carrying = fielder == receiver;
        let (throw_speed, exchange) = if fielder.infield() {
            (INFIELDER_THROW_SPEED, 0.38)
        } else {
            (OUTFIELDER_THROW_SPEED, 0.28)
        };
        let travel = if carrying {
            start.distance(target) / INFIELDER_SPEED
        } else {
            (release.xz().distance(target) / throw_speed).max(0.12)
        };
        let transfer = if carrying {
            0.18
        } else {
            let cover = defense.position(receiver).distance(target) / INFIELDER_SPEED;
            (cover - travel).max(exchange)
        };
        Self {
            fielder,
            receiver,
            pickup,
            target,
            base,
            release,
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
        defense.move_toward(self.receiver, self.target, INFIELDER_SPEED * moved);
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

    pub fn transfer_progress(&self) -> f32 {
        (self.elapsed / self.transfer).min(1.0)
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
        let arc = (self.release.xz().distance(self.target) * 0.05).clamp(0.3, 2.8);
        self.release.lerp(on_field(self.target, 1.1), fraction)
            + Vec3::Y * (4.0 * fraction * (1.0 - fraction) * arc)
    }
}

fn on_field(point: Vec2, height: f32) -> Vec3 {
    vec3(point.x, height, point.y)
}
