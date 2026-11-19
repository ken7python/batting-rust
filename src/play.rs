use macroquad::prelude::*;

use crate::batting::{Aim, Contact, Grade, WINDUP, grade, in_zone};
use crate::flight::{Flight, Outcome};
use crate::park::Park;
use crate::pitch::Pitch;

pub struct ResultCard {
    pub outcome: Outcome,
    pub grade: Option<Grade>,
    pub landing: Option<Vec3>,
}

pub enum Phase {
    Pitch,
    Flight(Flight),
    Result(ResultCard),
}

pub struct Play {
    pub pitch: Pitch,
    pub park: Park,
    pub time: f64,
    pub result_time: f64,
    pub phase: Phase,
    pub aim: Aim,
    contact: Option<Grade>,
}

impl Play {
    pub fn new(pitch: Pitch, park: Park) -> Self {
        Self {
            pitch,
            park,
            time: -WINDUP,
            result_time: 0.0,
            phase: Phase::Pitch,
            aim: Aim::default(),
            contact: None,
        }
    }

    pub fn next_pitch(&mut self, pitch: Pitch) {
        self.pitch = pitch;
        self.time = -WINDUP;
        self.result_time = 0.0;
        self.phase = Phase::Pitch;
        self.aim.next_pitch();
        self.contact = None;
    }

    pub fn can_swing(&self) -> bool {
        matches!(self.phase, Phase::Pitch) && self.time < self.pitch.end_time()
    }

    pub fn advance(&mut self, dt: f64) {
        self.time += dt;
        match &mut self.phase {
            Phase::Pitch => {
                if self.time < self.pitch.end_time().min(self.pitch.duration) {
                    return;
                }
                let evaluated = self
                    .aim
                    .live_swing()
                    .map(|s| grade(s, self.pitch.duration, self.pitch.crossing));
                let bounced = self.pitch.bounce_time.is_some();
                if let Some(hit) = evaluated.filter(|g| !bounced && g.contact != Contact::Miss) {
                    self.contact = Some(hit);
                    let mut flight = Flight::new(self.pitch.crossing, hit, self.park);
                    flight.advance(self.time - self.pitch.duration);
                    self.phase = Phase::Flight(flight);
                } else if self.time >= self.pitch.end_time() {
                    let outcome = if let Some(g) = evaluated {
                        self.contact = Some(g);
                        Outcome::Whiff
                    } else if !bounced && in_zone(self.pitch.crossing) {
                        Outcome::Strike
                    } else {
                        Outcome::Ball
                    };
                    self.finish(outcome, None);
                }
            }
            Phase::Flight(flight) => {
                if let Some(outcome) = flight.advance(dt) {
                    let landing = flight.position;
                    self.finish(outcome, Some(landing));
                }
            }
            Phase::Result(_) => self.result_time += dt,
        }
    }

    fn finish(&mut self, outcome: Outcome, landing: Option<Vec3>) {
        self.aim.held = false;
        self.phase = Phase::Result(ResultCard {
            outcome,
            grade: self.contact,
            landing,
        });
        self.result_time = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batting::{CENTER, SWING_LEAD};
    use crate::pitch::{BreakSize, Delivery, PitchKind, PitchSpeed};

    fn fastball() -> Play {
        Play::new(
            Pitch::new(
                PitchKind::Fastball,
                Delivery::default(),
                CENTER,
                PitchSpeed::Normal,
                0.5,
                BreakSize::Normal,
            ),
            Park::Central,
        )
    }

    #[test]
    fn repeated_warmups_allow_a_live_hit_or_a_called_strike() {
        for take in [false, true] {
            let mut play = fastball();
            for time in [-0.55, -0.32, -0.001] {
                play.advance(time - play.time);
                play.aim.press();
                assert!(play.aim.release(play.time));
                assert!(play.aim.live_swing().is_none());
            }
            if take {
                play.advance(play.pitch.end_time() - play.time + 0.001);
                assert!(matches!(
                    play.phase,
                    Phase::Result(ResultCard {
                        outcome: Outcome::Strike,
                        grade: None,
                        ..
                    })
                ));
            } else {
                // Begin holding during the windup, then release at the ideal live timing.
                play.aim.press();
                play.advance(play.pitch.duration - SWING_LEAD - play.time);
                assert!(play.aim.release(play.time));
                play.advance(SWING_LEAD + 0.001);
                assert!(matches!(play.phase, Phase::Flight(_)));
            }
        }
    }

    #[test]
    fn a_swing_at_release_counts_once_and_resets_for_the_next_windup() {
        let mut play = fastball();
        play.aim.press();
        play.advance(WINDUP);
        assert!(play.aim.release(play.time));
        play.aim.press();
        assert!(!play.aim.release(0.25));
        assert_eq!(play.aim.live_swing().unwrap().time, 0.0);
        play.advance(play.pitch.end_time() + 0.001);
        assert!(matches!(
            play.phase,
            Phase::Result(ResultCard {
                outcome: Outcome::Whiff,
                ..
            })
        ));
        play.next_pitch(play.pitch);
        play.aim.press();
        assert!(play.aim.release(play.time));
        assert!(play.aim.live_swing().is_none());
    }
}
