use macroquad::prelude::*;

use crate::batting::{Aim, Contact, Grade, WINDUP, grade, in_zone};
use crate::defense::{Defense, FieldReturn};
use crate::flight::{Flight, GroundContact, Outcome};
use crate::park::Park;
use crate::pitch::Pitch;
use crate::runner::Runner;

pub struct ResultCard {
    pub outcome: Outcome,
    pub grade: Option<Grade>,
    pub landing: Option<Vec3>,
    pub fielding: Option<FieldingView>,
}

pub struct FieldingView {
    pub defense: Defense,
    pub returning: Option<FieldReturn>,
    pub ball: Vec3,
    pub velocity: Vec3,
    pub elapsed: f32,
    pub runner: Runner,
    pub ground_contact: Option<GroundContact>,
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
            time: -WINDUP - pitch.release_delay,
            result_time: 0.0,
            phase: Phase::Pitch,
            aim: Aim::default(),
            contact: None,
        }
    }

    pub fn next_pitch(&mut self, pitch: Pitch) {
        self.pitch = pitch;
        self.time = -WINDUP - pitch.release_delay;
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
                    // Only this subset may take an extra base on a marginal forecast.
                    // Roll once per batted ball so the chance is independent of the frame rate.
                    flight.try_dive = rand::gen_range(0, 32) == 0;
                    self.phase = match flight.advance(self.time - self.pitch.duration) {
                        Some(outcome) => {
                            Phase::Result(batted_ball_result(outcome, self.contact, &flight))
                        }
                        None => Phase::Flight(flight),
                    };
                } else if self.time >= self.pitch.end_time() {
                    let outcome = if let Some(g) = evaluated {
                        self.contact = Some(g);
                        Outcome::Whiff
                    } else if !bounced && in_zone(self.pitch.crossing) {
                        Outcome::Strike
                    } else {
                        Outcome::Ball
                    };
                    self.finish(ResultCard {
                        outcome,
                        grade: self.contact,
                        landing: None,
                        fielding: None,
                    });
                }
            }
            Phase::Flight(flight) => {
                if let Some(outcome) = flight.advance(dt) {
                    let result = batted_ball_result(outcome, self.contact, flight);
                    self.finish(result);
                }
            }
            Phase::Result(_) => self.result_time += dt,
        }
    }

    fn finish(&mut self, result: ResultCard) {
        self.aim.held = false;
        self.phase = Phase::Result(result);
        self.result_time = 0.0;
    }
}

fn batted_ball_result(outcome: Outcome, grade: Option<Grade>, flight: &Flight) -> ResultCard {
    let fielder = outcome
        .fielder()
        .or(flight.returning.map(|returning| returning.fielder));
    let fielding = (fielder.is_some()
        || flight.ground_contact.is_some()
        || outcome == Outcome::Homer)
        .then(|| {
            let mut defense = flight.defense;
            defense.chasing = fielder;
            FieldingView {
                defense,
                returning: flight.returning,
                ball: flight.position,
                velocity: flight.velocity,
                elapsed: flight.elapsed,
                runner: flight.runner,
                ground_contact: flight.ground_contact,
            }
        });
    ResultCard {
        outcome,
        grade,
        landing: Some(flight.batted_point()),
        fielding,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batting::{CENTER, SWING_LEAD, ZONE_MAX};
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
    fn each_delivery_waits_its_own_release_delay_before_windup() {
        let mut run = fastball();
        for delay in [0.25, 0.61, 1.19] {
            let mut pitch = run.pitch;
            pitch.release_delay = delay;
            run.next_pitch(pitch);
            assert!(run.time < -WINDUP);
            run.advance(delay);
            assert!((run.time + WINDUP).abs() < 1e-9);
            run.advance(WINDUP - 0.001);
            assert!(run.time < 0.0);
            run.advance(0.002);
            assert!(run.time > 0.0);
        }
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

    #[test]
    fn a_ball_just_outside_the_zone_cannot_start_baserunning() {
        let mut play = fastball();
        play.pitch = Pitch::new(
            PitchKind::Fastball,
            Delivery::default(),
            vec2(ZONE_MAX.x + 0.05, CENTER.y),
            PitchSpeed::Normal,
            0.5,
            BreakSize::Normal,
        );
        play.aim.press();
        play.aim.drag(Vec2::X);
        play.advance(play.pitch.duration - SWING_LEAD - play.time);
        assert!(play.aim.release(play.time));
        play.advance(play.pitch.end_time() - play.time + 0.001);
        assert!(matches!(
            play.phase,
            Phase::Result(ResultCard {
                outcome: Outcome::Whiff,
                fielding: None,
                ..
            })
        ));
    }
}
