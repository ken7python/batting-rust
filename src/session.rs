use crate::flight::Outcome;
use crate::park::Park;
use crate::pitch::{Delivery, Pitch, PitchKind, PitchMix, PitchSpeed};
use crate::play::{Phase, Play};

pub const CHALLENGE_LENGTH: u32 = 10;
const RESULT_SECONDS: f64 = 1.45;

#[derive(Clone, Copy)]
pub struct AtBat {
    pub pitch: PitchKind,
    pub speed: f64,
    pub outcome: Outcome,
    pub distance: Option<f32>,
}

#[derive(Clone, Copy)]
pub enum Mode {
    Challenge,
    Practice { kind: Option<PitchKind> },
}

impl Mode {
    fn next_pitch(
        self,
        completed: u32,
        delivery: Delivery,
        speed: PitchSpeed,
        mix: PitchMix,
    ) -> Pitch {
        match self {
            Self::Challenge => Pitch::random(None, completed < 2, speed, delivery, mix),
            Self::Practice { kind, .. } => Pitch::random(kind, false, speed, delivery, mix),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Record {
    pub completed: u32,
    pub points: u32,
    pub fair: u32,
    pub home_runs: u32,
    pub doubles: u32,
    pub triples: u32,
    history: [Option<AtBat>; CHALLENGE_LENGTH as usize],
}

impl Record {
    fn record(&mut self, at_bat: AtBat) {
        self.history[self.completed as usize % self.history.len()] = Some(at_bat);
        self.completed += 1;
        let outcome = at_bat.outcome;
        self.points += outcome.points();
        self.fair += u32::from(outcome.is_hit());
        self.home_runs += u32::from(outcome == Outcome::Homer);
        self.doubles += u32::from(outcome == Outcome::Double);
        self.triples += u32::from(outcome == Outcome::Triple);
    }

    pub fn history(&self) -> impl Iterator<Item = (u32, AtBat)> + '_ {
        (self.completed.saturating_sub(CHALLENGE_LENGTH)..self.completed).filter_map(|index| {
            self.history[index as usize % self.history.len()].map(|entry| (index + 1, entry))
        })
    }
}

pub struct Session {
    pub mode: Mode,
    pub speed: PitchSpeed,
    pub record: Record,
    pub play: Play,
    pub paused: bool,
    pub history_open: bool,
    mix: PitchMix,
}

impl Session {
    pub fn new(
        mode: Mode,
        delivery: Delivery,
        speed: PitchSpeed,
        park: Park,
        mix: PitchMix,
    ) -> Self {
        Self {
            mode,
            speed,
            record: Record::default(),
            play: Play::new(mode.next_pitch(0, delivery, speed, mix), park),
            paused: false,
            history_open: false,
            mix,
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
            self.record.record(AtBat {
                pitch: self.play.pitch.kind,
                speed: self.play.pitch.average_kmh(),
                outcome: card.outcome,
                distance: card
                    .landing
                    .map(|point| (point.x * point.x + point.z * point.z).sqrt()),
            });
        }
        let result_seconds = match &self.play.phase {
            Phase::Result(card) if card.outcome == Outcome::Homer => 2.05,
            Phase::Result(card) if card.outcome.is_hit() => 1.75,
            _ => RESULT_SECONDS,
        };
        if self.play.result_time < result_seconds {
            return None;
        }
        if matches!(self.mode, Mode::Challenge) && self.record.completed >= CHALLENGE_LENGTH {
            return Some(self.record);
        }
        self.play.next_pitch(self.mode.next_pitch(
            self.record.completed,
            self.play.pitch.delivery,
            self.speed,
            self.mix,
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
            },
        ] {
            let mut run = Session::new(
                mode,
                Delivery::default(),
                PitchSpeed::Real,
                Park::Central,
                PitchMix::default(),
            );
            for completed in 0..2 {
                assert_eq!(run.record.completed, completed);
                assert!((129.0..147.0).contains(&run.play.pitch.average_kmh()));
                // Take the pitch and wait through its result to reach the next delivery.
                run.tick(crate::batting::WINDUP + run.play.pitch.end_time() + 0.01);
                run.tick(RESULT_SECONDS + 0.01);
            }
        }
    }

    #[test]
    fn ten_pitch_round_records_each_pitch_once_including_takes() {
        let mut run = Session::new(
            Mode::Challenge,
            Delivery::default(),
            PitchSpeed::Normal,
            Park::Central,
            PitchMix::default(),
        );
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
        let entries: Vec<_> = result.history().collect();
        assert_eq!(entries.len(), CHALLENGE_LENGTH as usize);
        assert_eq!(
            entries
                .iter()
                .map(|(number, _)| *number)
                .collect::<Vec<_>>(),
            (1..=CHALLENGE_LENGTH).collect::<Vec<_>>()
        );
        assert_eq!(entries[0].1.pitch, PitchKind::Fastball);
        assert!(entries.iter().all(|(_, entry)| {
            matches!(entry.outcome, Outcome::Strike | Outcome::Ball) && entry.distance.is_none()
        }));
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
            },
            delivery,
            PitchSpeed::Slow,
            Park::Sunset,
            PitchMix::default(),
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
        let numbers: Vec<_> = run.record.history().map(|(number, _)| number).collect();
        assert_eq!(
            numbers,
            ((run.record.completed - CHALLENGE_LENGTH + 1)..=run.record.completed)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn extra_base_hits_count_toward_the_hit_total() {
        let mut record = Record::default();
        for outcome in [
            Outcome::Homer,
            Outcome::HardFair,
            Outcome::Fair,
            Outcome::Double,
            Outcome::Triple,
            Outcome::Ball,
            Outcome::Foul,
            Outcome::Whiff,
            Outcome::Strike,
            Outcome::FlyOut(crate::defense::Fielder::Center),
            Outcome::GroundOut(crate::defense::Fielder::First),
        ] {
            record.record(AtBat {
                pitch: PitchKind::Fastball,
                speed: 105.0,
                outcome,
                distance: None,
            });
        }
        assert_eq!(
            (
                record.completed,
                record.points,
                record.fair,
                record.home_runs,
                record.doubles,
                record.triples
            ),
            (11, 1575, 5, 1, 1, 1)
        );
    }
}
