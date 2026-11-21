use macroquad::prelude::*;

pub const ZONE_MIN: Vec2 = vec2(-0.215, 0.52);
pub const ZONE_MAX: Vec2 = vec2(0.215, 1.16);
pub const CENTER: Vec2 = vec2(0.0, 0.84);
pub const CURSOR_RADIUS: f32 = (ZONE_MAX.x - ZONE_MIN.x) * 0.14;
pub const CORE_RADIUS: f32 = CURSOR_RADIUS * 0.32;
pub const MEET_RADIUS: f32 = CURSOR_RADIUS + DRAWN_BALL_RADIUS;
pub const BARREL_RADIUS: f32 = CORE_RADIUS + DRAWN_BALL_RADIUS;
pub const HIT_TIMING: f64 = 0.080;
pub const BARREL_TIMING: f64 = 0.024;
pub const SWING_LEAD: f64 = 0.075;
pub const WINDUP: f64 = 0.68;
pub const CATCH_DELAY: f64 = 0.13;
pub const BALL_RADIUS: f32 = 0.037;
pub const DRAWN_BALL_RADIUS: f32 = BALL_RADIUS * 1.25;

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
    let distance = offset.length();
    let contact = if !in_zone(crossing)
        || timing.abs() > HIT_TIMING + 1e-9
        || distance > MEET_RADIUS + 1e-6
    {
        Contact::Miss
    } else if timing.abs() <= BARREL_TIMING + 1e-9 && distance <= BARREL_RADIUS + 1e-6 {
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
    pub last_swing: Option<Swing>,
}

impl Default for Aim {
    fn default() -> Self {
        Self {
            point: CENTER,
            held: false,
            last_swing: None,
        }
    }
}

impl Aim {
    /// The pitch clock is zero when the ball leaves the pitcher's hand.
    pub fn live_swing(&self) -> Option<Swing> {
        self.last_swing.filter(|s| s.time >= 0.0)
    }

    pub fn press(&mut self) {
        self.held = self.live_swing().is_none();
    }

    pub fn drag(&mut self, movement: Vec2) {
        if self.held {
            self.point = (self.point + movement).clamp(ZONE_MIN, ZONE_MAX);
        }
    }

    pub fn release(&mut self, time: f64) -> bool {
        if !self.held {
            return false;
        }
        self.last_swing = Some(Swing {
            time,
            aim: self.point,
        });
        self.held = false;
        true
    }

    pub fn next_pitch(&mut self) {
        self.held = false;
        self.last_swing = None;
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
        assert_eq!(aim.live_swing().unwrap().time, 0.405);
        assert_eq!(aim.live_swing().unwrap().aim, frozen);
        aim.next_pitch();
        aim.release(0.41);
        assert!(aim.last_swing.is_none());
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
            grade(swing(0.33, CENTER), 0.48, CENTER).contact,
            Contact::Touch
        );
        assert_eq!(
            grade(swing(0.405, CENTER + vec2(0.0, MEET_RADIUS)), 0.48, CENTER).contact,
            Contact::Touch
        );
        assert_eq!(
            grade(swing(0.49, CENTER), 0.48, CENTER).contact,
            Contact::Miss
        );
        assert_eq!(
            grade(
                swing(0.405, CENTER + vec2(0.0, MEET_RADIUS + 0.001)),
                0.48,
                CENTER
            )
            .contact,
            Contact::Miss
        );
    }

    #[test]
    fn overlapping_the_green_cursor_makes_contact_at_every_edge() {
        for direction in [Vec2::X, Vec2::Y, vec2(-1.0, 1.0).normalize()] {
            let swing = Swing {
                time: 0.405,
                aim: CENTER,
            };
            for (distance, expected) in [
                (CORE_RADIUS + DRAWN_BALL_RADIUS - 0.0001, Contact::Barrel),
                (CURSOR_RADIUS + DRAWN_BALL_RADIUS, Contact::Touch),
                (CURSOR_RADIUS + DRAWN_BALL_RADIUS + 0.001, Contact::Miss),
            ] {
                assert_eq!(
                    grade(swing, 0.48, CENTER + direction * distance).contact,
                    expected
                );
            }
        }
    }

    #[test]
    fn ball_radius_is_checked_at_zone_edges_and_corners() {
        assert!(in_zone(vec2(ZONE_MAX.x + 0.03, CENTER.y)));
        assert!(!in_zone(ZONE_MAX + vec2(0.03, 0.03)));
    }

    #[test]
    fn held_aim_stops_at_every_edge_of_the_zone() {
        let mut aim = Aim::default();
        aim.press();
        aim.drag(vec2(4.0, 4.0));
        assert_eq!(aim.point, ZONE_MAX);
        aim.drag(vec2(-8.0, -8.0));
        assert_eq!(aim.point, ZONE_MIN);
        aim.drag(vec2(0.0, 0.2));
        assert!((aim.point.y - (ZONE_MIN.y + 0.2)).abs() < 1e-6);
        aim.release(0.4);
        assert_eq!(aim.live_swing().unwrap().aim, aim.point);
    }

    #[test]
    fn an_edge_swing_meets_a_pitch_clipping_the_zone_but_misses_a_ball() {
        let swing = Swing {
            time: 0.405,
            aim: vec2(ZONE_MAX.x, CENTER.y),
        };
        for (distance, expected) in [(0.03, Contact::Barrel), (0.05, Contact::Miss)] {
            let crossing = swing.aim + Vec2::X * distance;
            assert_eq!(grade(swing, 0.48, crossing).contact, expected);
        }
    }
}
