use macroquad::prelude::*;

use crate::batting::{BALL_RADIUS, CATCH_DELAY, ZONE_MAX, ZONE_MIN};

const BASE_ARC: Vec2 = vec2(0.0, 0.32);
const RELEASE_DELAY_MIN: f64 = 0.25;
const RELEASE_DELAY_MAX: f64 = 1.20;

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
            Self::Cutter => "カットボール",
            Self::Sinker => "シンカー",
            Self::Splitter => "スプリット",
            Self::Sweeper => "スイーパー",
            Self::Knuckleball => "ナックル",
        }
    }

    fn profile(self) -> ((f32, f32), Vec2, Vec2) {
        // The two break vectors are fractions of the zone width and height.
        match self {
            Self::Fastball => ((0.46, 0.52), Vec2::ZERO, vec2(0.025, -0.030)),
            Self::Slider => ((0.51, 0.59), vec2(0.060, 0.015), vec2(0.265, 0.065)),
            Self::Curve => ((0.61, 0.74), vec2(0.040, 0.170), vec2(0.055, 0.150)),
            Self::Forkball => ((0.53, 0.61), vec2(0.007, 0.020), vec2(0.020, 0.280)),
            Self::Changeup => ((0.60, 0.70), vec2(-0.090, 0.095), vec2(-0.100, 0.100)),
            Self::TwoSeam => ((0.47, 0.54), vec2(-0.070, 0.008), vec2(-0.150, 0.030)),
            Self::Cutter => ((0.48, 0.55), vec2(0.045, 0.010), vec2(0.145, 0.015)),
            Self::Sinker => ((0.50, 0.59), vec2(-0.080, 0.035), vec2(-0.195, 0.115)),
            Self::Splitter => ((0.53, 0.61), vec2(0.020, 0.027), vec2(-0.060, 0.235)),
            Self::Sweeper => ((0.56, 0.66), vec2(0.120, 0.020), vec2(0.230, 0.033)),
            Self::Knuckleball => ((0.68, 0.84), vec2(0.012, 0.105), vec2(-0.022, 0.112)),
        }
    }

    fn preview_direction(self) -> Vec2 {
        match self {
            Self::Fastball => Vec2::ZERO,
            Self::Slider | Self::Cutter | Self::Sweeper => Vec2::NEG_X,
            Self::TwoSeam => Vec2::X,
            Self::Forkball | Self::Splitter | Self::Knuckleball => Vec2::NEG_Y,
            Self::Curve => vec2(-0.32, -1.0),
            Self::Changeup => vec2(0.6, -1.0),
            Self::Sinker => vec2(1.0, -1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Repertoire {
    Fastballs,
    #[default]
    Basic,
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
    pub release_delay: f64,
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
            release_delay: 0.0,
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
        strike: bool,
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
        let crossing = if strike {
            vec2(
                rand::gen_range(ZONE_MIN.x, ZONE_MAX.x),
                rand::gen_range(ZONE_MIN.y, ZONE_MAX.y),
            )
        } else {
            // Miss by 4–7 cm: a ball cannot clip the zone, but stays close.
            let gap = rand::gen_range(BALL_RADIUS + 0.005, 0.07);
            match rand::gen_range(0, 4) {
                0 => vec2(ZONE_MIN.x - gap, rand::gen_range(ZONE_MIN.y, ZONE_MAX.y)),
                1 => vec2(ZONE_MAX.x + gap, rand::gen_range(ZONE_MIN.y, ZONE_MAX.y)),
                2 => vec2(rand::gen_range(ZONE_MIN.x, ZONE_MAX.x), ZONE_MAX.y + gap),
                _ => vec2(rand::gen_range(ZONE_MIN.x, ZONE_MAX.x), ZONE_MIN.y - gap),
            }
        };
        let mut pitch = Self::new(
            kind,
            delivery,
            crossing,
            speed,
            rand::gen_range(0.0, 1.0),
            mix.break_size,
        );
        pitch.release_delay = rand::gen_range(RELEASE_DELAY_MIN, RELEASE_DELAY_MAX);
        pitch
    }

    pub fn end_time(self) -> f64 {
        self.bounce_time.unwrap_or(self.duration + CATCH_DELAY)
    }

    pub fn average_kmh(self) -> f64 {
        f64::from(self.delivery.release().z.abs()) / self.duration * 3.6
    }

    pub fn preview_point(self, elapsed: f64) -> Vec3 {
        let end = self.bounce_time.unwrap_or(self.duration);
        let finish = self.position(end);
        if self.kind == PitchKind::Fastball || elapsed >= end {
            return finish;
        }
        let mut direction = self.kind.preview_direction().normalize_or_zero();
        let (_, early, late) = self.kind.profile();
        let distance =
            (-(early + late).dot(direction) * self.break_size.multiplier()).clamp(0.0, 0.50);
        direction.x *= self.delivery.hand.sign();
        let progress = (elapsed / end).max(0.0) as f32;
        let remaining = 1.0 - progress * progress;
        let shift = direction * (distance * remaining) * (ZONE_MAX - ZONE_MIN);
        finish - shift.extend(0.0)
    }

    fn baseline_at(self, t: f32) -> Vec3 {
        self.delivery.release().lerp(self.crossing.extend(0.0), t)
            + (BASE_ARC * (4.0 * t * (1.0 - t))).extend(0.0)
    }

    fn break_at(self, t: f32) -> Vec2 {
        let (_, early_break, late_break) = self.kind.profile();
        let arc = 4.0 * t * (1.0 - t);
        // The late curve peaks at 1.0 three quarters of the way to the plate.
        let late = (256.0 / 27.0) * t * t * t * (1.0 - t);
        let mut bend = early_break * arc + late_break * late;
        if self.kind == PitchKind::Knuckleball {
            let phase = self.wobble_phase + t * 19.0;
            bend += vec2(phase.sin() * 0.14, (phase * 0.74).cos() * 0.07) * arc;
        }
        let mut bend = (bend * self.break_size.multiplier()).clamp_length_max(1.0);
        bend.x *= self.delivery.hand.sign();
        bend
    }

    pub fn position(self, elapsed: f64) -> Vec3 {
        let u = (elapsed / self.duration).max(0.0) as f32;
        let shape =
            |t| self.baseline_at(t) + (self.break_at(t) * (ZONE_MAX - ZONE_MIN)).extend(0.0);
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
    use crate::batting::CENTER;

    #[test]
    fn random_pitches_match_the_requested_call() {
        for i in 0..10000 {
            let pitch = Pitch::random(
                None,
                i % 3 == 0,
                i % 2 == 0,
                PitchSpeed::Normal,
                Delivery::default(),
                PitchMix::default(),
            );
            assert!((RELEASE_DELAY_MIN..RELEASE_DELAY_MAX).contains(&pitch.release_delay));
            let p = pitch.crossing;
            let inside =
                p.x >= ZONE_MIN.x && p.x <= ZONE_MAX.x && p.y >= ZONE_MIN.y && p.y <= ZONE_MAX.y;
            assert_eq!(crate::batting::in_zone(p), i % 2 == 0);
            if !inside {
                let distance = (p - p.clamp(ZONE_MIN, ZONE_MAX)).length();
                assert!(distance > BALL_RADIUS && distance <= 0.071);
            }
            assert!(pitch.bounce_time.is_none());
        }
    }

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
            let random = Pitch::random(
                None,
                false,
                true,
                PitchSpeed::Normal,
                Delivery::default(),
                mix,
            );
            assert!(mix.repertoire.pool().contains(&random.kind));
        }
        let fixed = Pitch::random(
            Some(PitchKind::Knuckleball),
            false,
            true,
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
            + (BASE_ARC * (4.0 * progress * (1.0 - progress))).extend(0.0);
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
    fn pitch_breaks_stay_within_the_intended_zone_fractions() {
        for kind in PitchKind::ALL {
            for (size, limit) in [
                (BreakSize::Small, 0.22),
                (BreakSize::Normal, 1.0 / 3.0),
                (BreakSize::Large, 0.50),
            ] {
                for sample in (0..=16).map(|step| step as f32 / 16.0) {
                    let pitch = Pitch::new(
                        kind,
                        Delivery::default(),
                        CENTER,
                        PitchSpeed::Normal,
                        sample,
                        size,
                    );
                    let peak = (0..=128)
                        .map(|step| {
                            let t = step as f32 / 128.0;
                            let baseline = pitch.delivery.release().lerp(CENTER.extend(0.0), t)
                                + (BASE_ARC * (4.0 * t * (1.0 - t))).extend(0.0);
                            let offset = pitch.position(pitch.duration * f64::from(t)) - baseline;
                            (offset.truncate() / (ZONE_MAX - ZONE_MIN)).length()
                        })
                        .fold(0.0, f32::max);
                    assert!(peak <= limit, "{kind:?} at {size:?}: {peak}");
                    if size == BreakSize::Normal
                        && matches!(
                            kind,
                            PitchKind::Slider
                                | PitchKind::Curve
                                | PitchKind::Forkball
                                | PitchKind::Sinker
                                | PitchKind::Splitter
                                | PitchKind::Sweeper
                        )
                    {
                        assert!(peak >= 0.25, "{kind:?}: {peak}");
                    }
                }
            }
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
                > fastball.position(fastball.duration * 0.6).y + 0.12
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
        let early = pitch.preview_point(0.0);
        let later = pitch.preview_point(pitch.end_time() * 0.75);
        assert!(early.z < 0.0 && later.z < 0.0);
        assert!(early.distance(later) > 0.05);
        assert_eq!(pitch.preview_point(pitch.end_time()), bounce);
        assert!((bounce.y - BALL_RADIUS).abs() < 1e-5 && bounce.z < 0.0);
    }

    #[test]
    fn preview_moves_in_one_direction_and_finishes_at_the_plate() {
        for hand in [ThrowingHand::Right, ThrowingHand::Left] {
            for (kind, expected) in [
                (PitchKind::Forkball, Vec2::NEG_Y),
                (PitchKind::Slider, vec2(-hand.sign(), 0.0)),
                (PitchKind::Sinker, vec2(hand.sign(), -1.0)),
            ] {
                let pitch = Pitch::new(
                    kind,
                    Delivery {
                        hand,
                        ..Delivery::default()
                    },
                    CENTER,
                    PitchSpeed::Normal,
                    0.5,
                    BreakSize::Normal,
                );
                let start = pitch.preview_point(0.0).truncate();
                let finish = pitch.preview_point(pitch.duration);
                assert!(finish.abs_diff_eq(CENTER.extend(0.0), 1e-5));
                let direction = (finish.truncate() - start).normalize();
                assert!(direction.dot(expected.normalize()) > 0.97);
                let mut last = 0.0;
                for frame in 1..=20 {
                    let point = pitch.preview_point(pitch.duration * f64::from(frame) / 20.0);
                    let offset = point.truncate() - start;
                    assert!(offset.perp_dot(direction).abs() < 1e-5);
                    let along = offset.dot(direction);
                    assert!(along > last);
                    last = along;
                }
            }
        }
        let fastball = Pitch::new(
            PitchKind::Fastball,
            Delivery::default(),
            CENTER,
            PitchSpeed::Normal,
            0.5,
            BreakSize::Normal,
        );
        assert_eq!(
            fastball.preview_point(0.0),
            fastball.preview_point(fastball.duration * 0.8)
        );
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
