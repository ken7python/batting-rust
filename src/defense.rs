use macroquad::prelude::*;

use crate::park::Park;

pub const FIRST_BASE: Vec2 = vec2(19.4, -19.4);

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
        let speed = if fielder.infield() { 5.3 } else { 6.0 };
        let toward_ball = target - self.positions[index];
        self.positions[index] +=
            toward_ball.normalize_or_zero() * toward_ball.length().min(speed * dt);
        self.chasing = Some(fielder);
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
