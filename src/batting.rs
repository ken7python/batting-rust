use macroquad::prelude::*;

pub const ZONE_MIN: Vec2 = vec2(-0.215, 0.52);
pub const ZONE_MAX: Vec2 = vec2(0.215, 1.16);
pub const CENTER: Vec2 = vec2(0.0, 0.84);
pub const MEET_RADIUS: Vec2 = vec2(0.18, 0.14);
pub const SWING_LEAD: f64 = 0.075;
pub const WINDUP: f64 = 0.68;
pub const CATCH_DELAY: f64 = 0.13;
pub const BALL_RADIUS: f32 = 0.037;

#[derive(Clone, Copy, Debug)]
pub struct Swing {
    pub time: f64,
    pub aim: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contact {
    Miss,
    Touch,
    Barrel,
}

#[derive(Clone, Copy, Debug)]
pub struct Grade {
    pub contact: Contact,
    pub timing: f64,
    pub offset: Vec2,
}

pub fn grade(swing: Swing, arrival: f64, crossing: Vec2) -> Grade {
    let timing = swing.time + SWING_LEAD - arrival;
    let offset = swing.aim - crossing;
    let distance = (offset / MEET_RADIUS).length();
    let contact = if timing.abs() > 0.065 + 1e-9 || distance > 1.0 {
        Contact::Miss
    } else if timing.abs() <= 0.030 + 1e-9 && distance <= 0.5 {
        Contact::Barrel
    } else {
        Contact::Touch
    };
    Grade {
        contact,
        timing,
        offset,
    }
}

pub fn in_zone(crossing: Vec2) -> bool {
    (crossing - crossing.clamp(ZONE_MIN, ZONE_MAX)).length() <= BALL_RADIUS
}

pub struct Aim {
    pub point: Vec2,
    pub held: bool,
    pub swing: Option<Swing>,
}

impl Default for Aim {
    fn default() -> Self {
        Self {
            point: CENTER,
            held: false,
            swing: None,
        }
    }
}

impl Aim {
    pub fn press(&mut self) {
        self.held = self.swing.is_none();
    }

    pub fn drag(&mut self, movement: Vec2) {
        if self.held {
            let margin = vec2(0.65, 0.49);
            self.point = (self.point + movement).clamp(ZONE_MIN - margin, ZONE_MAX + margin);
        }
    }

    pub fn release(&mut self, time: f64) {
        if self.held {
            self.swing = Some(Swing {
                time,
                aim: self.point,
            });
        }
        self.held = false;
    }

    pub fn next_pitch(&mut self) {
        self.held = false;
        self.swing = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_release_freezes_aim_and_next_pitch_requires_a_new_press() {
        let mut aim = Aim::default();
        aim.press();
        aim.drag(vec2(0.11, 0.08));
        aim.release(0.405);
        let frozen = aim.point;
        aim.press();
        aim.drag(vec2(-0.1, 0.0));
        aim.release(0.46);
        assert_eq!(aim.swing.unwrap().time, 0.405);
        assert_eq!(aim.swing.unwrap().aim, frozen);
        aim.next_pitch();
        aim.release(0.41);
        assert!(aim.swing.is_none());
        assert_eq!(aim.point, frozen);
    }

    #[test]
    fn timing_and_aim_both_affect_contact() {
        let swing = |time, aim| Swing { time, aim };
        assert_eq!(
            grade(swing(0.405, CENTER), 0.48, CENTER).contact,
            Contact::Barrel
        );
        assert_eq!(
            grade(swing(0.34, CENTER), 0.48, CENTER).contact,
            Contact::Touch
        );
        assert_eq!(
            grade(swing(0.48, CENTER), 0.48, CENTER).contact,
            Contact::Miss
        );
        assert_eq!(
            grade(swing(0.405, CENTER + vec2(0.0, 0.15)), 0.48, CENTER).contact,
            Contact::Miss
        );
    }

    #[test]
    fn ball_radius_is_checked_at_zone_edges_and_corners() {
        assert!(in_zone(vec2(ZONE_MAX.x + 0.03, CENTER.y)));
        assert!(!in_zone(ZONE_MAX + vec2(0.03, 0.03)));
    }

    #[test]
    fn held_aim_can_sweep_past_every_edge_of_the_zone() {
        let mut aim = Aim::default();
        aim.press();
        aim.drag(vec2(4.0, 4.0));
        assert!(aim.point.cmpgt(ZONE_MAX).all());
        aim.drag(vec2(-8.0, -8.0));
        assert!(aim.point.cmplt(ZONE_MIN).all());
        aim.release(0.4);
        assert_eq!(aim.swing.unwrap().aim, aim.point);
    }
}
