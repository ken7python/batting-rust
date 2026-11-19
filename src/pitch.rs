use macroquad::prelude::*;

use crate::batting::{BALL_RADIUS, CATCH_DELAY, CENTER, ZONE_MAX, ZONE_MIN};
use crate::scene::RELEASE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PitchKind {
    Fastball,
    Slider,
    Curve,
    Forkball,
    Changeup,
}

impl PitchKind {
    pub const ALL: [Self; 5] = [
        Self::Fastball,
        Self::Slider,
        Self::Curve,
        Self::Forkball,
        Self::Changeup,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Fastball => "ストレート",
            Self::Slider => "スライダー",
            Self::Curve => "カーブ",
            Self::Forkball => "フォーク",
            Self::Changeup => "チェンジアップ",
        }
    }

    fn profile(self) -> ((f32, f32), Vec2, Vec2) {
        match self {
            Self::Fastball => ((0.46, 0.52), vec2(0.0, 0.32), vec2(0.02, -0.035)),
            Self::Slider => ((0.51, 0.59), vec2(0.13, 0.41), vec2(0.79, 0.22)),
            Self::Curve => ((0.61, 0.74), vec2(0.12, 0.94), vec2(0.25, 0.68)),
            Self::Forkball => ((0.53, 0.61), vec2(0.02, 0.34), vec2(0.04, 0.91)),
            Self::Changeup => ((0.60, 0.70), vec2(-0.10, 0.48), vec2(-0.23, 0.17)),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Pitch {
    pub kind: PitchKind,
    pub crossing: Vec2,
    pub duration: f64,
    pub bounce_time: Option<f64>,
}

impl Pitch {
    pub fn new(kind: PitchKind, crossing: Vec2, speed: f64, sample: f32) -> Self {
        let ((min, max), _, _) = kind.profile();
        let mut pitch = Self {
            kind,
            crossing,
            duration: f64::from(min + (max - min) * sample) / speed,
            bounce_time: None,
        };
        if crossing.y < BALL_RADIUS {
            let (mut low, mut high) = (0.0, pitch.duration);
            for _ in 0..24 {
                let middle = (low + high) * 0.5;
                if pitch.position(middle).y <= BALL_RADIUS {
                    high = middle;
                } else {
                    low = middle;
                }
            }
            pitch.bounce_time = Some(high);
        }
        pitch
    }

    pub fn random(fixed: Option<PitchKind>, opening: bool, speed: f64) -> Self {
        let kind = if opening {
            PitchKind::Fastball
        } else {
            fixed.unwrap_or_else(|| PitchKind::ALL[rand::gen_range(0, PitchKind::ALL.len())])
        };
        let crossing = if opening {
            CENTER + vec2(rand::gen_range(-0.09, 0.09), rand::gen_range(-0.10, 0.10))
        } else if rand::gen_range(0.0, 1.0) < 0.73 {
            vec2(
                rand::gen_range(ZONE_MIN.x, ZONE_MAX.x),
                rand::gen_range(ZONE_MIN.y, ZONE_MAX.y),
            )
        } else {
            match rand::gen_range(0, 4) {
                0 => vec2(rand::gen_range(-0.51, -0.27), rand::gen_range(0.49, 1.19)),
                1 => vec2(rand::gen_range(0.27, 0.51), rand::gen_range(0.49, 1.19)),
                2 => vec2(rand::gen_range(-0.31, 0.31), rand::gen_range(1.23, 1.45)),
                _ if kind == PitchKind::Forkball && rand::gen_range(0.0, 1.0) < 0.45 => {
                    vec2(rand::gen_range(-0.30, 0.30), rand::gen_range(-0.35, -0.05))
                }
                _ => vec2(rand::gen_range(-0.31, 0.31), rand::gen_range(0.16, 0.44)),
            }
        };
        Self::new(kind, crossing, speed, rand::gen_range(0.0, 1.0))
    }

    pub fn end_time(self) -> f64 {
        self.bounce_time.unwrap_or(self.duration + CATCH_DELAY)
    }

    pub fn position(self, elapsed: f64) -> Vec3 {
        let u = (elapsed / self.duration).max(0.0) as f32;
        let (_, arc, late_break) = self.kind.profile();
        let shape = |t: f32| {
            RELEASE.lerp(self.crossing.extend(0.0), t)
                + (arc * (4.0 * t * (1.0 - t)) + late_break * (6.0 * t.powi(3) * (1.0 - t)))
                    .extend(0.0)
        };
        if u <= 1.0 {
            shape(u)
        } else {
            let end = shape(1.0);
            let velocity = (end - shape(0.99)) * 100.0;
            let extended = end + velocity * (u - 1.0);
            vec3(extended.x, extended.y.max(BALL_RADIUS), extended.z)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_pitch_releases_from_the_hand_and_crosses_the_selected_target() {
        let target = vec2(-0.12, 0.73);
        for kind in PitchKind::ALL {
            let pitch = Pitch::new(kind, target, 1.0, 0.5);
            assert_eq!(pitch.position(0.0), RELEASE);
            assert!(
                pitch
                    .position(pitch.duration)
                    .abs_diff_eq(target.extend(0.0), 1e-5)
            );
        }
        let fastball = Pitch::new(PitchKind::Fastball, target, 1.0, 0.5);
        let curve = Pitch::new(PitchKind::Curve, target, 1.0, 0.5);
        assert!(
            curve.position(curve.duration * 0.6).y
                > fastball.position(fastball.duration * 0.6).y + 0.5
        );
        assert!(curve.duration > fastball.duration);
    }

    #[test]
    fn a_forkball_in_the_dirt_ends_before_the_plate() {
        let pitch = Pitch::new(PitchKind::Forkball, vec2(0.0, -0.3), 1.0, 0.5);
        assert!(pitch.end_time() < pitch.duration);
        let bounce = pitch.position(pitch.end_time());
        assert!((bounce.y - BALL_RADIUS).abs() < 1e-5 && bounce.z < 0.0);
    }
}
