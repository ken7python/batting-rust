use macroquad::prelude::*;

use crate::batting::{Aim, Contact, Grade, WINDUP, grade, in_zone};
use crate::flight::{Flight, Outcome};
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
    pub time: f64,
    pub result_time: f64,
    pub phase: Phase,
    pub aim: Aim,
    contact: Option<Grade>,
}

impl Play {
    pub fn new(pitch: Pitch) -> Self {
        Self {
            pitch,
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
                    .swing
                    .map(|s| grade(s, self.pitch.duration, self.pitch.crossing));
                let bounced = self.pitch.bounce_time.is_some();
                if let Some(hit) = evaluated.filter(|g| !bounced && g.contact != Contact::Miss) {
                    self.contact = Some(hit);
                    let mut flight = Flight::new(self.pitch.crossing, hit);
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
