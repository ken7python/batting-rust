use macroquad::prelude::*;

use crate::park::Park;

pub const FIRST_BASE: Vec2 = vec2(19.4, -19.4);
pub const SECOND_BASE: Vec2 = vec2(0.0, -38.8);
pub const RUNNER_TO_FIRST: f32 = 4.25;

const INFIELDER_SPEED: f32 = 5.3;
const THROW_SPEED: f32 = 26.0;

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
        }
    }

    pub fn chase(&mut self, position: Vec3, velocity: Vec3, elapsed: f32, dt: f32, rolling: bool) {
        if elapsed < 0.24 {
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
        let speed = if fielder.infield() {
            INFIELDER_SPEED
        } else {
            6.0
        };
        self.move_toward(fielder, target, speed * dt);
        if rolling && fielder != Fielder::First {
            self.move_toward(Fielder::First, FIRST_BASE, INFIELDER_SPEED * dt);
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

    pub fn within_reach(&self, position: Vec2, radius: f32) -> Option<Fielder> {
        let index = self.closest(position);
        (self.positions[index].distance_squared(position) <= radius * radius)
            .then_some(Fielder::ALL[index])
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
    release: Vec3,
    transfer: f32,
    travel: f32,
    elapsed: f32,
}

impl FieldReturn {
    pub fn new(fielder: Fielder, pickup: Vec3, defense: &Defense) -> Self {
        let (receiver, target) = if fielder.infield() {
            (Fielder::First, FIRST_BASE)
        } else {
            let covering = [Fielder::Shortstop, Fielder::Second]
                .into_iter()
                .min_by(|&a, &b| {
                    defense
                        .position(a)
                        .distance_squared(SECOND_BASE)
                        .total_cmp(&defense.position(b).distance_squared(SECOND_BASE))
                })
                .expect("there are two infielders to cover second");
            (covering, SECOND_BASE)
        };
        let start = defense.position(fielder);
        let toward = (target - start).normalize_or_zero();
        let release = on_field(start + toward * 0.30, 1.65);
        let carrying = fielder == receiver;
        let travel = if carrying {
            start.distance(target) / INFIELDER_SPEED
        } else {
            (release.xz().distance(target) / THROW_SPEED).max(0.12)
        };
        let transfer = if carrying {
            0.18
        } else {
            let cover = defense.position(receiver).distance(target) / INFIELDER_SPEED;
            (cover - travel).max(0.38)
        };
        Self {
            fielder,
            receiver,
            pickup,
            target,
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
        self.elapsed >= self.transfer + self.travel
    }

    pub fn carrying(&self) -> bool {
        self.fielder == self.receiver
    }

    pub fn target_label(&self) -> &'static str {
        if self.fielder.infield() {
            "一塁"
        } else {
            "二塁"
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
