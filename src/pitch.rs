use macroquad::prelude::*;

use crate::batting::{BALL_RADIUS, CATCH_DELAY, CENTER, ZONE_MAX, ZONE_MIN};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PitchSpeed {
    Slow,
    #[default]
    Normal,
    High,
    Real,
}

impl PitchSpeed {
    pub const ALL: [Self; 4] = [Self::Slow, Self::Normal, Self::High, Self::Real];

    pub fn label(self) -> &'static str {
        match self {
            Self::Slow => "スロー",
            Self::Normal => "通常",
            Self::High => "ハイ",
            Self::Real => "リアル",
        }
    }

    fn multiplier(self) -> f64 {
        match self {
            Self::Slow => 0.60,
            Self::Normal => 0.88,
            Self::High => 1.04,
            Self::Real => 1.15,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThrowingHand {
    #[default]
    Right,
    Left,
}

impl ThrowingHand {
    pub fn sign(self) -> f32 {
        if self == Self::Right { 1.0 } else { -1.0 }
    }

    pub fn label(self) -> &'static str {
        if self == Self::Right {
            "右投手"
        } else {
            "左投手"
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PitchForm {
    #[default]
    Overhand,
    Sidearm,
}

impl PitchForm {
    pub fn label(self) -> &'static str {
        match self {
            Self::Overhand => "上手投げ",
            Self::Sidearm => "横手投げ",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Delivery {
    pub hand: ThrowingHand,
    pub form: PitchForm,
}

impl Delivery {
    pub fn release(self) -> Vec3 {
        let (reach, height) = match self.form {
            PitchForm::Overhand => (0.40, 1.91),
            PitchForm::Sidearm => (0.81, 1.30),
        };
        vec3(reach * self.hand.sign(), height, -16.25)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PitchKind {
    Fastball,
    Slider,
    Curve,
    Forkball,
    Changeup,
    TwoSeam,
    Cutter,
    Sinker,
    Splitter,
    Sweeper,
    Knuckleball,
}

impl PitchKind {
    pub const ALL: [Self; 11] = [
        Self::Fastball,
        Self::Slider,
        Self::Curve,
        Self::Forkball,
        Self::Changeup,
        Self::TwoSeam,
        Self::Cutter,
        Self::Sinker,
        Self::Splitter,
        Self::Sweeper,
        Self::Knuckleball,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Fastball => "ストレート",
            Self::Slider => "スライダー",
            Self::Curve => "カーブ",
            Self::Forkball => "フォーク",
            Self::Changeup => "チェンジアップ",
            Self::TwoSeam => "ツーシーム",
            Self::Cutter => "カッター",
            Self::Sinker => "シンカー",
            Self::Splitter => "スプリット",
            Self::Sweeper => "スイーパー",
            Self::Knuckleball => "ナックル",
        }
    }

    fn profile(self) -> ((f32, f32), Vec2, Vec2) {
        match self {
            Self::Fastball => ((0.46, 0.52), vec2(0.0, 0.32), vec2(0.02, -0.035)),
            Self::Slider => ((0.51, 0.59), vec2(0.13, 0.41), vec2(0.79, 0.22)),
            Self::Curve => ((0.61, 0.74), vec2(0.12, 0.94), vec2(0.25, 0.68)),
            Self::Forkball => ((0.53, 0.61), vec2(0.02, 0.34), vec2(0.04, 0.91)),
            Self::Changeup => ((0.60, 0.70), vec2(-0.10, 0.48), vec2(-0.23, 0.17)),
            Self::TwoSeam => ((0.47, 0.54), vec2(-0.09, 0.33), vec2(-0.37, 0.05)),
            Self::Cutter => ((0.48, 0.55), vec2(0.06, 0.34), vec2(0.32, 0.04)),
            Self::Sinker => ((0.50, 0.59), vec2(-0.14, 0.39), vec2(-0.62, 0.42)),
            Self::Splitter => ((0.53, 0.61), vec2(0.04, 0.35), vec2(-0.15, 0.64)),
            Self::Sweeper => ((0.56, 0.66), vec2(0.48, 0.41), vec2(1.18, 0.14)),
            Self::Knuckleball => ((0.68, 0.84), vec2(0.02, 0.57), vec2(-0.05, 0.29)),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Repertoire {
    Fastballs,
    Basic,
    #[default]
    Varied,
}

impl Repertoire {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fastballs => "直球中心",
            Self::Basic => "基本5種",
            Self::Varied => "全11種",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Fastballs => Self::Basic,
            Self::Basic => Self::Varied,
            Self::Varied => Self::Fastballs,
        }
    }

    fn pool(self) -> &'static [PitchKind] {
        use PitchKind::{Changeup, Fastball, Slider};
        match self {
            Self::Fastballs => &[Fastball, Fastball, Fastball, Slider, Changeup],
            Self::Basic => &PitchKind::ALL[..5],
            Self::Varied => &PitchKind::ALL,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BreakSize {
    Small,
    #[default]
    Normal,
    Large,
}

impl BreakSize {
    pub fn label(self) -> &'static str {
        match self {
            Self::Small => "小",
            Self::Normal => "標準",
            Self::Large => "大",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Small => Self::Normal,
            Self::Normal => Self::Large,
            Self::Large => Self::Small,
        }
    }

    fn multiplier(self) -> f32 {
        match self {
            Self::Small => 0.65,
            Self::Normal => 1.0,
            Self::Large => 1.5,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PitchMix {
    pub repertoire: Repertoire,
    pub break_size: BreakSize,
}

#[derive(Clone, Copy)]
pub struct Pitch {
    pub kind: PitchKind,
    pub delivery: Delivery,
    pub crossing: Vec2,
    pub duration: f64,
    pub bounce_time: Option<f64>,
    break_size: BreakSize,
    wobble_phase: f32,
}

impl Pitch {
    pub fn new(
        kind: PitchKind,
        delivery: Delivery,
        crossing: Vec2,
        speed: PitchSpeed,
        sample: f32,
        break_size: BreakSize,
    ) -> Self {
        let ((min, max), _, _) = kind.profile();
        let mut pitch = Self {
            kind,
            delivery,
            crossing,
            duration: f64::from(min + (max - min) * sample) / speed.multiplier(),
            bounce_time: None,
            break_size,
            wobble_phase: sample * std::f32::consts::TAU,
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

    pub fn random(
        fixed: Option<PitchKind>,
        opening: bool,
        speed: PitchSpeed,
        delivery: Delivery,
        mix: PitchMix,
    ) -> Self {
        let kind = if opening {
            PitchKind::Fastball
        } else {
            fixed.unwrap_or_else(|| {
                let pool = mix.repertoire.pool();
                pool[rand::gen_range(0, pool.len())]
            })
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
                _ if matches!(kind, PitchKind::Forkball | PitchKind::Splitter)
                    && rand::gen_range(0.0, 1.0) < 0.45 =>
                {
                    vec2(rand::gen_range(-0.30, 0.30), rand::gen_range(-0.35, -0.05))
                }
                _ => vec2(rand::gen_range(-0.31, 0.31), rand::gen_range(0.16, 0.44)),
            }
        };
        Self::new(
            kind,
            delivery,
            crossing,
            speed,
            rand::gen_range(0.0, 1.0),
            mix.break_size,
        )
    }

    pub fn end_time(self) -> f64 {
        self.bounce_time.unwrap_or(self.duration + CATCH_DELAY)
    }

    pub fn average_kmh(self) -> f64 {
        f64::from(self.delivery.release().z.abs()) / self.duration * 3.6
    }

    pub fn position(self, elapsed: f64) -> Vec3 {
        let u = (elapsed / self.duration).max(0.0) as f32;
        let (_, arc, late_break) = self.kind.profile();
        let shape = |t: f32| {
            let taper = t * (1.0 - t);
            let base_arc = vec2(0.0, 0.32);
            let mut pitch_break =
                (arc - base_arc) * (4.0 * taper) + late_break * (6.0 * t * t * taper);
            if self.kind == PitchKind::Knuckleball {
                let phase = self.wobble_phase + t * 19.0;
                pitch_break +=
                    vec2(phase.sin() * 0.22, (phase * 0.74).cos() * 0.11) * (4.0 * taper);
            }
            let mut movement =
                base_arc * (4.0 * taper) + pitch_break * self.break_size.multiplier();
            movement.x *= self.delivery.hand.sign();
            self.delivery.release().lerp(self.crossing.extend(0.0), t) + movement.extend(0.0)
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
    fn repertories_limit_random_pitches_but_allow_the_chosen_practice_pitch() {
        let mix = PitchMix {
            repertoire: Repertoire::Fastballs,
            break_size: BreakSize::Large,
        };
        assert_eq!(
            mix.repertoire
                .pool()
                .iter()
                .filter(|&&kind| kind == PitchKind::Fastball)
                .count(),
            3
        );
        assert_eq!(Repertoire::Basic.pool().len(), 5);
        assert_eq!(Repertoire::Varied.pool().len(), 11);
        for _ in 0..20 {
            let random = Pitch::random(None, false, PitchSpeed::Normal, Delivery::default(), mix);
            assert!(mix.repertoire.pool().contains(&random.kind));
        }
        let fixed = Pitch::random(
            Some(PitchKind::Knuckleball),
            false,
            PitchSpeed::Normal,
            Delivery::default(),
            mix,
        );
        assert_eq!(fixed.kind, PitchKind::Knuckleball);
        assert_eq!(fixed.break_size, BreakSize::Large);
    }

    #[test]
    fn break_size_changes_movement_while_keeping_the_arrival_time_and_target() {
        let delivery = Delivery::default();
        let target = vec2(-0.12, 0.73);
        let progress = 0.75;
        let baseline = delivery.release().lerp(target.extend(0.0), progress)
            + vec3(0.0, 0.32 * 4.0 * progress * (1.0 - progress), 0.0);
        for kind in PitchKind::ALL {
            let pitches = [BreakSize::Small, BreakSize::Normal, BreakSize::Large].map(|strength| {
                Pitch::new(kind, delivery, target, PitchSpeed::High, 0.5, strength)
            });
            let deviations = pitches.map(|pitch| {
                assert_eq!(pitch.duration, pitches[1].duration);
                assert!(
                    pitch
                        .position(pitch.duration)
                        .abs_diff_eq(target.extend(0.0), 1e-5)
                );
                pitch
                    .position(pitch.duration * f64::from(progress))
                    .distance(baseline)
            });
            assert!(
                deviations[0] < deviations[1] && deviations[1] < deviations[2],
                "{kind:?}"
            );
        }
    }

    #[test]
    fn faster_presets_shorten_every_pitch_without_moving_its_break() {
        for kind in PitchKind::ALL {
            let pitches = PitchSpeed::ALL.map(|speed| {
                Pitch::new(
                    kind,
                    Delivery::default(),
                    CENTER,
                    speed,
                    0.5,
                    BreakSize::Normal,
                )
            });
            for pair in pitches.windows(2) {
                assert!(pair[0].duration > pair[1].duration);
                assert!(
                    pair[0]
                        .position(pair[0].duration * 0.75)
                        .abs_diff_eq(pair[1].position(pair[1].duration * 0.75), 1e-5)
                );
            }
        }
        for (speed, expected_kmh) in PitchSpeed::ALL.into_iter().zip([72.0, 105.0, 124.0, 137.0]) {
            let fastball = Pitch::new(
                PitchKind::Fastball,
                Delivery::default(),
                CENTER,
                speed,
                0.5,
                BreakSize::Normal,
            );
            assert_eq!(fastball.average_kmh().round(), expected_kmh, "{speed:?}");
        }
    }

    #[test]
    fn each_pitch_releases_from_the_hand_and_crosses_the_selected_target() {
        let target = vec2(-0.12, 0.73);
        for kind in PitchKind::ALL {
            let pitch = Pitch::new(
                kind,
                Delivery::default(),
                target,
                PitchSpeed::Normal,
                0.5,
                BreakSize::Normal,
            );
            assert_eq!(pitch.position(0.0), Delivery::default().release());
            assert!(
                pitch
                    .position(pitch.duration)
                    .abs_diff_eq(target.extend(0.0), 1e-5)
            );
        }
        let fastball = Pitch::new(
            PitchKind::Fastball,
            Delivery::default(),
            target,
            PitchSpeed::Normal,
            0.5,
            BreakSize::Normal,
        );
        let curve = Pitch::new(
            PitchKind::Curve,
            Delivery::default(),
            target,
            PitchSpeed::Normal,
            0.5,
            BreakSize::Normal,
        );
        assert!(
            curve.position(curve.duration * 0.6).y
                > fastball.position(fastball.duration * 0.6).y + 0.5
        );
        assert!(curve.duration > fastball.duration);
    }

    #[test]
    fn a_forkball_in_the_dirt_ends_before_the_plate() {
        let pitch = Pitch::new(
            PitchKind::Forkball,
            Delivery::default(),
            vec2(0.0, -0.3),
            PitchSpeed::Normal,
            0.5,
            BreakSize::Normal,
        );
        assert!(pitch.end_time() < pitch.duration);
        let bounce = pitch.position(pitch.end_time());
        assert!((bounce.y - BALL_RADIUS).abs() < 1e-5 && bounce.z < 0.0);
    }

    #[test]
    fn sidearm_has_a_lower_wider_release_for_both_hands() {
        for hand in [ThrowingHand::Right, ThrowingHand::Left] {
            let sidearm = Delivery {
                hand,
                form: PitchForm::Sidearm,
            };
            let overhand = Delivery {
                hand,
                form: PitchForm::Overhand,
            };
            let pitch = Pitch::new(
                PitchKind::Slider,
                sidearm,
                CENTER,
                PitchSpeed::Normal,
                0.5,
                BreakSize::Normal,
            );
            let release = pitch.position(0.0);
            assert!(release.y < overhand.release().y);
            assert!(release.x * hand.sign() > overhand.release().x * hand.sign());
            assert!(
                pitch
                    .position(pitch.duration)
                    .abs_diff_eq(CENTER.extend(0.0), 1e-5)
            );
        }
    }

    #[test]
    fn switching_pitcher_hands_mirrors_the_break() {
        for kind in PitchKind::ALL {
            let right = Pitch::new(
                kind,
                Delivery::default(),
                CENTER,
                PitchSpeed::Normal,
                0.4,
                BreakSize::Normal,
            );
            let left = Pitch::new(
                kind,
                Delivery {
                    hand: ThrowingHand::Left,
                    ..Default::default()
                },
                CENTER,
                PitchSpeed::Normal,
                0.4,
                BreakSize::Normal,
            );
            let a = right.position(right.duration * 0.73);
            let b = left.position(left.duration * 0.73);
            assert!(a.abs_diff_eq(vec3(-b.x, b.y, b.z), 1e-5));
        }
    }
}
