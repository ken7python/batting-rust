use macroquad::prelude::*;

use crate::batting::{Grade, MEET_RADIUS};
use crate::park::Park;
use crate::pitch::{Delivery, PitchForm, PitchKind, PitchSpeed, ThrowingHand};
use crate::play::{Phase, ResultCard};
use crate::session::{CHALLENGE_LENGTH, Mode, Record, Session};

const INK: Color = Color::from_rgba(13, 32, 36, 255);
const PANEL: Color = Color::from_rgba(17, 44, 46, 250);
const LINE: Color = Color::from_rgba(74, 108, 98, 255);
const CREAM: Color = Color::from_rgba(245, 241, 219, 255);
const MUTED: Color = Color::from_rgba(179, 199, 183, 255);
const AMBER: Color = Color::from_rgba(255, 201, 95, 255);

#[derive(Clone, Copy)]
pub enum Action {
    Challenge,
    Practice,
    Pitch,
    Speed(PitchSpeed),
    Preview,
    Sensitivity,
    Hand,
    Form,
    Park,
    Pause,
    Resume,
    Menu,
}

pub struct Settings {
    pitch: usize,
    pub speed: PitchSpeed,
    pub preview: bool,
    sensitivity: usize,
    pub delivery: Delivery,
    pub park: Park,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pitch: 0,
            speed: PitchSpeed::default(),
            preview: false,
            sensitivity: 1,
            delivery: Delivery::default(),
            park: Park::default(),
        }
    }
}

impl Settings {
    fn selected_pitch(&self) -> Option<PitchKind> {
        PitchKind::ALL.get(self.pitch).copied()
    }

    pub fn practice(&self) -> Mode {
        Mode::Practice {
            kind: self.selected_pitch(),
            preview: self.preview,
        }
    }

    pub fn sensitivity(&self) -> f32 {
        [0.70, 1.0, 1.40][self.sensitivity]
    }

    pub fn change(&mut self, action: Action) {
        match action {
            Action::Pitch => self.pitch = (self.pitch + 1) % (PitchKind::ALL.len() + 1),
            Action::Speed(speed) => self.speed = speed,
            Action::Preview => self.preview = !self.preview,
            Action::Sensitivity => self.sensitivity = (self.sensitivity + 1) % 3,
            Action::Hand => {
                self.delivery.hand = if self.delivery.hand == ThrowingHand::Right {
                    ThrowingHand::Left
                } else {
                    ThrowingHand::Right
                }
            }
            Action::Form => {
                self.delivery.form = match self.delivery.form {
                    PitchForm::Overhand => PitchForm::Sidearm,
                    PitchForm::Sidearm => PitchForm::Overhand,
                };
            }
            Action::Park => {
                self.park = match self.park {
                    Park::Central => Park::Sunset,
                    Park::Sunset => Park::Central,
                };
            }
            _ => {}
        }
    }
}

pub struct Ui<'a> {
    font: &'a Font,
    scale: f32,
    mouse: Vec2,
    pressed: bool,
}

impl<'a> Ui<'a> {
    pub fn new(font: &'a Font) -> Self {
        Self {
            font,
            scale: (screen_width() / 980.0)
                .min(screen_height() / 650.0)
                .clamp(0.40, 1.7),
            mouse: Vec2::from(mouse_position()),
            pressed: is_mouse_button_pressed(MouseButton::Left),
        }
    }

    fn text(&self, label: &str, x: f32, baseline: f32, size: f32, color: Color) {
        draw_text_ex(
            label,
            x,
            baseline,
            TextParams {
                font: Some(self.font),
                font_size: (size * self.scale).round() as u16,
                color,
                ..Default::default()
            },
        );
    }

    fn panel(&self, rect: Rect) {
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, PANEL);
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, self.scale, LINE);
    }

    fn button(&self, rect: Rect, title: &str, primary: bool) -> bool {
        let hovered = rect.contains(self.mouse);
        let bg = match (primary, hovered) {
            (true, true) => CREAM,
            (true, false) => AMBER,
            (false, true) => Color::from_rgba(44, 81, 70, 255),
            (false, false) => INK,
        };
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
        if !primary {
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, self.scale, LINE);
        }
        self.text(
            title,
            rect.x + 14.0 * self.scale,
            rect.y + rect.h * 0.63,
            20.0,
            if primary { INK } else { CREAM },
        );
        hovered && self.pressed
    }

    pub fn menu(&self, settings: &Settings) -> Option<Action> {
        let s = self.scale;
        let x = 28.0 * s;
        let y = (screen_height() - 608.0 * s) * 0.5;
        let w = 436.0 * s;
        let rect = |x0, y0, width, height| Rect::new(x + x0 * s, y + y0 * s, width * s, height * s);
        self.panel(Rect::new(x, y, w, 608.0 * s));
        draw_rectangle(x, y, w, 4.0 * s, AMBER);
        self.text("3D BASEBALL", x + 24.0 * s, y + 43.0 * s, 16.0, AMBER);
        self.text("BATTING CLUB", x + 22.0 * s, y + 97.0 * s, 47.0, CREAM);
        self.text(
            "ドラッグで狙い、離して打つ。",
            x + 24.0 * s,
            y + 135.0 * s,
            21.0,
            MUTED,
        );
        let challenge = self.button(rect(24.0, 163.0, 388.0, 51.0), "10球チャレンジ", true);
        let practice = self.button(rect(24.0, 229.0, 388.0, 51.0), "練習を始める", false);
        self.text("球速", x + 24.0 * s, y + 313.0 * s, 18.0, MUTED);
        let mut speed_choice = None;
        for (index, speed) in PitchSpeed::ALL.into_iter().enumerate() {
            let bounds = rect(24.0 + index as f32 * 99.0, 324.0, 91.0, 44.0);
            if self.button(bounds, speed.label(), speed == settings.speed) {
                speed_choice = Some(Action::Speed(speed));
            }
        }
        let pitch_label = settings
            .selected_pitch()
            .map_or("ランダム", PitchKind::label);
        let pitch = self.button(
            rect(24.0, 383.0, 238.0, 44.0),
            &format!("球種  {pitch_label}"),
            false,
        );
        let preview = self.button(
            rect(270.0, 383.0, 142.0, 44.0),
            if settings.preview {
                "予告  あり"
            } else {
                "予告  なし"
            },
            false,
        );
        let sensitivity_label = ["低い", "標準", "高い"][settings.sensitivity];
        let sensitivity = self.button(
            rect(24.0, 443.0, 188.0, 44.0),
            &format!("感度  {sensitivity_label}"),
            false,
        );
        let hand = self.button(
            rect(224.0, 443.0, 188.0, 44.0),
            settings.delivery.hand.label(),
            false,
        );
        let form = self.button(
            rect(24.0, 500.0, 188.0, 44.0),
            settings.delivery.form.label(),
            false,
        );
        let park = self.button(
            rect(224.0, 500.0, 188.0, 44.0),
            &format!("{} {:.0}m", settings.park.label(), settings.park.fence().0),
            false,
        );
        self.text(
            "操作  左ボタンを押したままマウスを動かす",
            x + 24.0 * s,
            y + 584.0 * s,
            16.0,
            MUTED,
        );
        [
            (challenge, Action::Challenge),
            (practice, Action::Practice),
            (pitch, Action::Pitch),
            (preview, Action::Preview),
            (sensitivity, Action::Sensitivity),
            (hand, Action::Hand),
            (form, Action::Form),
            (park, Action::Park),
        ]
        .into_iter()
        .find_map(|(hit, action)| hit.then_some(action))
        .or(speed_choice)
    }

    pub fn pause_rect(&self) -> Rect {
        let s = self.scale;
        Rect::new(screen_width() - 140.0 * s, 20.0 * s, 120.0 * s, 42.0 * s)
    }

    pub fn game(&self, session: &Session) -> Option<Action> {
        let s = self.scale;
        let x = 20.0 * s;
        let y = 20.0 * s;
        let mut action = None;
        let shown_pitch =
            session.record.completed + u32::from(!matches!(session.play.phase, Phase::Result(_)));
        let (mode, count) = match session.mode {
            Mode::Challenge => (
                "10球チャレンジ",
                format!("{shown_pitch:02} / {CHALLENGE_LENGTH:02}"),
            ),
            Mode::Practice { .. } => ("練習", format!("{shown_pitch:02} 球")),
        };
        self.panel(Rect::new(x, y, 288.0 * s, 81.0 * s));
        self.text(mode, x + 14.0 * s, y + 25.0 * s, 16.0, MUTED);
        self.text(
            &format!("球速  {}", session.speed.label()),
            x + 154.0 * s,
            y + 25.0 * s,
            16.0,
            MUTED,
        );
        self.text(&count, x + 14.0 * s, y + 62.0 * s, 27.0, CREAM);
        if matches!(session.mode, Mode::Challenge) {
            self.text(
                &format!("{} 点", session.record.points),
                x + 154.0 * s,
                y + 62.0 * s,
                27.0,
                AMBER,
            );
        }
        if !session.paused && self.button(self.pause_rect(), "一時停止", false) {
            action = Some(Action::Pause);
        }
        if let Phase::Result(card) = &session.play.phase {
            self.feedback(session, card);
        } else if let Phase::Flight(flight) = &session.play.phase {
            let h = screen_height();
            self.panel(Rect::new(x, h - 89.0 * s, 195.0 * s, 68.0 * s));
            let label = if flight.cleared_fence {
                "ホームラン"
            } else {
                "飛距離"
            };
            self.text(
                label,
                x + 13.0 * s,
                h - 64.0 * s,
                17.0,
                if flight.cleared_fence { AMBER } else { MUTED },
            );
            self.text(
                &format!("{:.0} m", flight.position.xz().length()),
                x + 13.0 * s,
                h - 30.0 * s,
                26.0,
                CREAM,
            );
        } else if matches!(session.play.phase, Phase::Pitch) {
            let hint = if session.play.time < 0.0 {
                if session.play.aim.held {
                    "離して素振り"
                } else {
                    "投球前は何度でも素振り"
                }
            } else if session.play.aim.live_swing().is_some() {
                "スイング済み"
            } else if session.play.aim.held {
                "離してスイング"
            } else {
                "左ボタンを押してドラッグ"
            };
            let h = screen_height();
            self.panel(Rect::new(x, h - 59.0 * s, 270.0 * s, 40.0 * s));
            self.text(hint, x + 13.0 * s, h - 32.0 * s, 17.0, CREAM);
        }

        if session.paused {
            draw_rectangle(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(0.04, 0.10, 0.11, 0.63),
            );
            let x = (screen_width() - 320.0 * s) * 0.5;
            let y = (screen_height() - 218.0 * s) * 0.5;
            self.panel(Rect::new(x, y, 320.0 * s, 218.0 * s));
            self.text("一時停止", x + 22.0 * s, y + 49.0 * s, 32.0, CREAM);
            if self.button(
                Rect::new(x + 22.0 * s, y + 78.0 * s, 276.0 * s, 47.0 * s),
                "再開する",
                true,
            ) {
                action = Some(Action::Resume);
            }
            if self.button(
                Rect::new(x + 22.0 * s, y + 142.0 * s, 276.0 * s, 47.0 * s),
                "タイトルへ",
                false,
            ) {
                action = Some(Action::Menu);
            }
        }
        action
    }

    fn feedback(&self, session: &Session, result: &ResultCard) {
        let s = self.scale;
        let x = (screen_width() - 422.0 * s) * 0.5;
        let y = screen_height() - 149.0 * s;
        self.panel(Rect::new(x, y, 422.0 * s, 128.0 * s));
        let pitch = session.play.pitch.kind.label();
        let location = if session.play.pitch.bounce_time.is_some() {
            "  ワンバウンド"
        } else {
            ""
        };
        let distance = result
            .landing
            .map_or_else(String::new, |p| format!("   {:.0} m", p.xz().length()));
        self.text(
            &format!(
                "{pitch}  {:.0} km/h{location}",
                session.play.pitch.average_kmh()
            ),
            x + 15.0 * s,
            y + 26.0 * s,
            17.0,
            MUTED,
        );
        self.text(
            &format!("{}{distance}", result.outcome.label()),
            x + 15.0 * s,
            y + 67.0 * s,
            29.0,
            AMBER,
        );
        if let Some(grade) = result.grade {
            self.text(
                &feedback_label(grade),
                x + 15.0 * s,
                y + 106.0 * s,
                17.0,
                CREAM,
            );
        } else {
            self.text("見送り", x + 15.0 * s, y + 106.0 * s, 17.0, CREAM);
        }
    }

    pub fn summary(&self, record: Record) -> Option<Action> {
        let s = self.scale;
        let x = (screen_width() - 390.0 * s) * 0.5;
        let y = (screen_height() - 385.0 * s) * 0.5;
        self.panel(Rect::new(x, y, 390.0 * s, 385.0 * s));
        draw_rectangle(x, y, 390.0 * s, 4.0 * s, AMBER);
        self.text(
            "10球チャレンジ結果",
            x + 23.0 * s,
            y + 50.0 * s,
            26.0,
            CREAM,
        );
        self.text(
            &format!("{} 点", record.points),
            x + 20.0 * s,
            y + 126.0 * s,
            57.0,
            AMBER,
        );
        self.text(
            &format!(
                "フェア打球  {}    本塁打  {}",
                record.fair, record.home_runs
            ),
            x + 23.0 * s,
            y + 189.0 * s,
            21.0,
            MUTED,
        );
        let replay = self.button(
            Rect::new(x + 23.0 * s, y + 232.0 * s, 344.0 * s, 49.0 * s),
            "もう一度",
            true,
        );
        let menu = self.button(
            Rect::new(x + 23.0 * s, y + 302.0 * s, 344.0 * s, 49.0 * s),
            "タイトルへ",
            false,
        );
        if replay {
            Some(Action::Challenge)
        } else {
            menu.then_some(Action::Menu)
        }
    }
}

fn feedback_label(grade: Grade) -> String {
    let timing = if grade.timing < -0.03 {
        "早い"
    } else if grade.timing > 0.03 {
        "遅い"
    } else {
        "適正"
    };
    let offset = grade.offset / MEET_RADIUS;
    let position = if offset.length() <= 0.5 {
        "芯付近"
    } else if offset.y.abs() >= offset.x.abs() {
        if offset.y > 0.0 {
            "照準が高い"
        } else {
            "照準が低い"
        }
    } else if offset.x > 0.0 {
        "照準が右"
    } else {
        "照準が左"
    };
    format!("タイミング {timing}  /  {position}")
}
