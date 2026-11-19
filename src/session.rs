use crate::flight::Outcome;
use crate::pitch::{Delivery, Pitch, PitchKind, PitchSpeed};
use crate::play::{Phase, Play};

pub const CHALLENGE_LENGTH: u32 = 10;
const RESULT_SECONDS: f64 = 1.45;

#[derive(Clone, Copy)]
pub enum Mode {
    Challenge,
    Practice {
        kind: Option<PitchKind>,
        preview: bool,
    },
}

impl Mode {
    fn next_pitch(self, completed: u32, delivery: Delivery, speed: PitchSpeed) -> Pitch {
        match self {
            Self::Challenge => Pitch::random(None, completed < 2, speed, delivery),
            Self::Practice { kind, .. } => Pitch::random(kind, false, speed, delivery),
        }
    }

    pub fn preview(self) -> bool {
        matches!(self, Self::Practice { preview: true, .. })
    }
}

#[derive(Clone, Copy, Default)]
pub struct Record {
    pub completed: u32,
    pub points: u32,
    pub fair: u32,
    pub home_runs: u32,
}

impl Record {
    fn record(&mut self, outcome: Outcome) {
        self.completed += 1;
        self.points += match outcome {
            Outcome::Homer => 500,
            Outcome::HardFair => 250,
            Outcome::Fair => 100,
            Outcome::Ball => 25,
            Outcome::Whiff | Outcome::Foul | Outcome::Strike => 0,
        };
        self.fair += u32::from(matches!(
            outcome,
            Outcome::Homer | Outcome::HardFair | Outcome::Fair
        ));
        self.home_runs += u32::from(outcome == Outcome::Homer);
    }
}

pub struct Session {
    pub mode: Mode,
    pub speed: PitchSpeed,
    pub record: Record,
    pub play: Play,
    pub paused: bool,
}

impl Session {
    pub fn new(mode: Mode, delivery: Delivery, speed: PitchSpeed) -> Self {
        Self {
            mode,
            speed,
            record: Record::default(),
            play: Play::new(mode.next_pitch(0, delivery, speed)),
            paused: false,
        }
    }

    pub fn pause(&mut self) {
        self.play.aim.held = false;
        self.paused = true;
    }

    pub fn tick(&mut self, dt: f64) -> Option<Record> {
        if self.paused {
            return None;
        }
        let was_resolved = matches!(self.play.phase, Phase::Result(_));
        self.play.advance(dt);
        if let Phase::Result(card) = &self.play.phase
            && !was_resolved
        {
            self.record.record(card.outcome);
        }
        if self.play.result_time < RESULT_SECONDS {
            return None;
        }
        if matches!(self.mode, Mode::Challenge) && self.record.completed >= CHALLENGE_LENGTH {
            return Some(self.record);
        }
        self.play.next_pitch(self.mode.next_pitch(
            self.record.completed,
            self.play.pitch.delivery,
            self.speed,
        ));
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::{PitchForm, ThrowingHand};

    #[test]
    fn both_modes_keep_real_speed_for_the_first_and_next_pitch() {
        for mode in [
            Mode::Challenge,
            Mode::Practice {
                kind: Some(PitchKind::Fastball),
                preview: false,
            },
        ] {
            let mut run = Session::new(mode, Delivery::default(), PitchSpeed::Real);
            for completed in 0..2 {
                assert_eq!(run.record.completed, completed);
                assert!((140.0..160.0).contains(&run.play.pitch.average_kmh()));
                // Take the pitch and wait through its result to reach the next delivery.
                run.tick(crate::batting::WINDUP + run.play.pitch.end_time() + 0.01);
                run.tick(RESULT_SECONDS + 0.01);
            }
        }
    }

    #[test]
    fn ten_pitch_round_records_each_pitch_once_including_takes() {
        let mut run = Session::new(Mode::Challenge, Delivery::default(), PitchSpeed::Normal);
        assert_eq!(run.play.pitch.kind, PitchKind::Fastball);
        let mut result = None;
        for _ in 0..4000 {
            if run.record.completed == 1 && matches!(run.play.phase, Phase::Pitch) {
                assert_eq!(run.play.pitch.kind, PitchKind::Fastball);
            }
            if let Some(final_record) = run.tick(1.0 / 60.0) {
                result = Some(final_record);
                break;
            }
        }
        let result = result.expect("round should reach results");
        assert_eq!(result.completed, 10);
        assert_eq!(result.fair, 0);
        assert_eq!(result.home_runs, 0);
    }

    #[test]
    fn practice_keeps_pitch_selection_and_pause_cancels_held_swing() {
        let delivery = Delivery {
            hand: ThrowingHand::Left,
            form: PitchForm::Sidearm,
        };
        let mut run = Session::new(
            Mode::Practice {
                kind: Some(PitchKind::Curve),
                preview: true,
            },
            delivery,
            PitchSpeed::Slow,
        );
        run.play.aim.press();
        run.pause();
        let previous_time = run.play.time;
        assert!(!run.play.aim.held);
        assert!(run.tick(0.05).is_none());
        assert_eq!(run.play.time, previous_time);
        run.paused = false;
        for _ in 0..4000 {
            assert!(run.tick(1.0 / 60.0).is_none());
            assert_eq!(run.play.pitch.kind, PitchKind::Curve);
            assert_eq!(run.play.pitch.delivery, delivery);
        }
        assert!(run.record.completed > 10);
    }

    #[test]
    fn a_homer_counts_toward_the_fair_total() {
        let mut record = Record::default();
        for outcome in [
            Outcome::Homer,
            Outcome::HardFair,
            Outcome::Fair,
            Outcome::Ball,
            Outcome::Foul,
            Outcome::Whiff,
            Outcome::Strike,
        ] {
            record.record(outcome);
        }
        assert_eq!(
            (
                record.completed,
                record.points,
                record.fair,
                record.home_runs
            ),
            (7, 875, 3, 1)
        );
    }
}
