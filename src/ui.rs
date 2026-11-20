use macroquad::prelude::*;

use crate::batting::{BARREL_RADIUS, BARREL_TIMING, Grade};
use crate::flight::Outcome;
use crate::park::Park;
use crate::pitch::{Delivery, PitchForm, PitchKind, PitchMix, PitchSpeed, ThrowingHand};
use crate::play::{Phase, ResultCard};
use crate::runner::Runner;
use crate::session::{CHALLENGE_LENGTH, Mode, Record, Session};

const INK: Color = Color::from_rgba(13, 32, 36, 255);
const PANEL: Color = Color::from_rgba(17, 44, 46, 250);
const LINE: Color = Color::from_rgba(74, 108, 98, 255);
const CREAM: Color = Color::from_rgba(245, 241, 219, 255);
const MUTED: Color = Color::from_rgba(179, 199, 183, 255);
const AMBER: Color = Color::from_rgba(255, 201, 95, 255);
const AIM_CALL_COLOR: Color = Color::from_rgba(161, 236, 154, 255);

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
    Repertoire,
    BreakSize,
    Music,
    Effects,
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
    pub pitch_mix: PitchMix,
    pub music: bool,
    pub effects: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pitch: 0,
            speed: PitchSpeed::default(),
            preview: true,
            sensitivity: 1,
            delivery: Delivery::default(),
            park: Park::default(),
            pitch_mix: PitchMix::default(),
            music: true,
            effects: true,
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
            Action::Repertoire => self.pitch_mix.repertoire = self.pitch_mix.repertoire.next(),
            Action::BreakSize => self.pitch_mix.break_size = self.pitch_mix.break_size.next(),
            Action::Music => self.music = !self.music,
            Action::Effects => self.effects = !self.effects,
            _ => {}
        }
    }
}

enum Draw {
    Text {
        label: String,
        position: Vec2,
        size: u16,
        color: Color,
    },
    Fill(Rect, Color),
    Border(Rect, Color),
    Line(Vec2, Vec2, Color),
    Dot(Vec2, f32, Color),
}

pub struct Ui<'a> {
    font: &'a Font,
    scale: f32,
    area: Rect,
    touch: bool,
    mouse: Vec2,
    pressed: bool,
    draws: Vec<Draw>,
}

impl<'a> Ui<'a> {
    pub fn new(font: &'a Font, area: Rect, press: Option<Vec2>, touch: bool) -> Self {
        Self {
            font,
            scale: (area.w / 980.0)
                .min(area.h / 680.0)
                .clamp(if touch { 0.01 } else { 0.40 }, 1.7),
            area,
            touch,
            mouse: press.unwrap_or_else(|| {
                if touch {
                    Vec2::splat(-1.0)
                } else {
                    Vec2::from(mouse_position())
                }
            }),
            pressed: press.is_some(),
            draws: Vec::new(),
        }
    }

    fn text(&mut self, label: &str, x: f32, baseline: f32, size: f32, color: Color) {
        self.draws.push(Draw::Text {
            label: label.into(),
            position: vec2(x, baseline),
            size: (size * self.scale).round() as u16,
            color,
        });
    }

    fn fill(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        self.draws.push(Draw::Fill(Rect::new(x, y, w, h), color));
    }

    fn panel(&mut self, rect: Rect) {
        self.draws.push(Draw::Fill(rect, PANEL));
        self.draws.push(Draw::Border(rect, LINE));
    }

    pub fn draw(self) {
        // Rasterize glyphs before drawing text so a growing atlas cannot invalidate
        // a texture already queued by an earlier label.
        for command in &self.draws {
            if let Draw::Text { label, size, .. } = command {
                measure_text(label, Some(self.font), *size, 1.0);
            }
        }
        for command in self.draws {
            match command {
                Draw::Text {
                    label,
                    position,
                    size,
                    color,
                } => {
                    draw_text_ex(
                        label,
                        position.x,
                        position.y,
                        TextParams {
                            font: Some(self.font),
                            font_size: size,
                            color,
                            ..Default::default()
                        },
                    );
                }
                Draw::Fill(r, color) => draw_rectangle(r.x, r.y, r.w, r.h, color),
                Draw::Border(r, color) => {
                    draw_rectangle_lines(r.x, r.y, r.w, r.h, self.scale, color)
                }
                Draw::Line(from, to, color) => {
                    draw_line(from.x, from.y, to.x, to.y, 1.5 * self.scale, color)
                }
                Draw::Dot(center, radius, color) => draw_circle(center.x, center.y, radius, color),
            }
        }
    }

    fn button(&mut self, rect: Rect, title: &str, primary: bool) -> bool {
        let hovered = rect.contains(self.mouse);
        let bg = match (primary, hovered) {
            (true, true) => CREAM,
            (true, false) => AMBER,
            (false, true) => Color::from_rgba(44, 81, 70, 255),
            (false, false) => INK,
        };
        self.fill(rect.x, rect.y, rect.w, rect.h, bg);
        if !primary {
            self.draws.push(Draw::Border(rect, LINE));
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

    pub fn menu(&mut self, settings: &Settings) -> Option<Action> {
        let s = self.scale;
        let audio_x = self.area.right() - 160.0 * s;
        let music = self.button(
            Rect::new(audio_x, self.area.y + 20.0 * s, 140.0 * s, 42.0 * s),
            if settings.music {
                "BGM オン"
            } else {
                "BGM オフ"
            },
            false,
        );
        let effects = self.button(
            Rect::new(audio_x, self.area.y + 72.0 * s, 140.0 * s, 42.0 * s),
            if settings.effects {
                "効果音 オン"
            } else {
                "効果音 オフ"
            },
            false,
        );
        let x = self.area.x + 28.0 * s;
        let y = self.area.center().y - 657.0 * s * 0.5;
        let w = 436.0 * s;
        let rect = |x0, y0, width, height| Rect::new(x + x0 * s, y + y0 * s, width * s, height * s);
        self.panel(Rect::new(x, y, w, 657.0 * s));
        self.fill(x, y, w, 4.0 * s, AMBER);
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
        let repertoire = self.button(
            rect(24.0, 554.0, 188.0, 44.0),
            &format!("配球  {}", settings.pitch_mix.repertoire.label()),
            false,
        );
        let break_size = self.button(
            rect(224.0, 554.0, 188.0, 44.0),
            &format!("変化量  {}", settings.pitch_mix.break_size.label()),
            false,
        );
        self.text(
            if self.touch {
                "操作  ドラッグで照準を動かす"
            } else {
                "操作  左ボタンを押したままマウスを動かす"
            },
            x + 24.0 * s,
            y + 635.0 * s,
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
            (repertoire, Action::Repertoire),
            (break_size, Action::BreakSize),
            (music, Action::Music),
            (effects, Action::Effects),
        ]
        .into_iter()
        .find_map(|(hit, action)| hit.then_some(action))
        .or(speed_choice)
    }

    pub fn pause_rect(&self) -> Rect {
        let s = self.scale;
        Rect::new(
            self.area.right() - 140.0 * s,
            self.area.y + 20.0 * s,
            120.0 * s,
            42.0 * s,
        )
    }

    pub fn game(&mut self, session: &Session) -> Option<Action> {
        let s = self.scale;
        let x = self.area.x + 20.0 * s;
        let y = self.area.y + 20.0 * s;
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
            if session.play.result_time < 0.9 {
                match card.outcome {
                    Outcome::Foul => self.field_call("ファール", AMBER),
                    Outcome::FlyOut(_) | Outcome::GroundOut(_) | Outcome::BaseOut(_) => {
                        self.field_call("アウト", Color::from_rgba(255, 151, 126, 255));
                    }
                    _ => {}
                }
            }
            if let Some(view) = &card.fielding
                && view.returning.is_some()
            {
                self.runner_diamond(&view.runner, false);
            }
        } else if let Phase::Flight(flight) = &session.play.phase {
            let h = self.area.bottom();
            self.panel(Rect::new(x, h - 89.0 * s, 195.0 * s, 68.0 * s));
            let (label, detail) = if let Some(returning) = flight.returning {
                let label = if returning.carrying() {
                    "ベースカバー"
                } else if returning.airborne() {
                    "送球"
                } else {
                    "捕球"
                };
                (
                    label,
                    format!(
                        "{} → {}",
                        returning.fielder.label(),
                        returning.target_label()
                    ),
                )
            } else {
                let label = if flight.cleared_fence {
                    "ホームラン"
                } else {
                    "飛距離"
                };
                (label, format!("{:.0} m", flight.position.xz().length()))
            };
            self.text(
                label,
                x + 13.0 * s,
                h - 64.0 * s,
                17.0,
                if flight.cleared_fence { AMBER } else { MUTED },
            );
            self.text(&detail, x + 13.0 * s, h - 30.0 * s, 26.0, CREAM);
            if !flight.cleared_fence {
                self.runner_diamond(&flight.runner, !session.paused && flight.can_boost_runner());
            }
            if flight
                .ground_contact
                .is_some_and(|contact| flight.elapsed - contact.at < 0.8)
            {
                self.field_call("フェア", AIM_CALL_COLOR);
            }
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
            } else if self.touch {
                "押したままドラッグ"
            } else {
                "左ボタンを押してドラッグ"
            };
            let h = self.area.bottom();
            self.panel(Rect::new(x, h - 59.0 * s, 270.0 * s, 40.0 * s));
            self.text(hint, x + 13.0 * s, h - 32.0 * s, 17.0, CREAM);
        }

        if session.paused {
            self.fill(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(0.04, 0.10, 0.11, 0.63),
            );
            let x = self.area.center().x - 160.0 * s;
            let y = self.area.center().y - 109.0 * s;
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

    fn field_call(&mut self, label: &str, color: Color) {
        let s = self.scale;
        let center = self.area.center().x;
        let top = self.area.y;
        let width = 145.0 * s;
        let x = center - width * 0.5;
        self.panel(Rect::new(x, top + 23.0 * s, width, 46.0 * s));
        self.fill(x, top + 23.0 * s, width, 3.0 * s, color);
        let size = 24.0;
        let measured = measure_text(label, Some(self.font), (size * s).round() as u16, 1.0);
        self.text(
            label,
            center - measured.width * 0.5,
            top + 55.0 * s,
            size,
            color,
        );
    }

    fn runner_diamond(&mut self, runner: &Runner, can_boost: bool) {
        let s = self.scale;
        let home = vec2(self.area.right() - 67.0 * s, self.area.bottom() - 94.0 * s);
        let corner = 23.0 * s;
        let bases = [
            home,
            home + vec2(corner, -corner),
            home + vec2(0.0, -2.0 * corner),
            home + vec2(-corner, -corner),
        ];
        let panel = Rect::new(home.x - 45.0 * s, home.y - 58.0 * s, 90.0 * s, 70.0 * s);
        self.panel(panel);
        for (index, point) in bases.into_iter().enumerate() {
            let next = bases[(index + 1) % bases.len()];
            self.draws.push(Draw::Line(point, next, LINE));
        }
        for (index, point) in bases.into_iter().enumerate() {
            let color = if index > 0 && index <= runner.reached() {
                AMBER
            } else {
                CREAM
            };
            self.fill(
                point.x - 2.5 * s,
                point.y - 2.5 * s,
                5.0 * s,
                5.0 * s,
                color,
            );
        }
        let marker = home + runner.position() * (corner / crate::defense::FIRST_BASE.x);
        self.draws.push(Draw::Dot(
            marker,
            if can_boost && runner.boosted() {
                7.0
            } else {
                5.0
            } * s,
            if can_boost && runner.boosted() {
                AMBER
            } else {
                Color::from_rgba(232, 116, 67, 255)
            },
        ));
        if can_boost {
            self.text(
                "タップで加速",
                panel.x,
                panel.bottom() + 18.0 * s,
                14.0,
                CREAM,
            );
        }
    }

    fn feedback(&mut self, session: &Session, result: &ResultCard) {
        let s = self.scale;
        let x = self.area.center().x - 211.0 * s;
        let y = self.area.bottom() - 149.0 * s;
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
        let fielder = result.outcome.fielder().map_or("", |role| role.label());
        self.text(
            &format!(
                "{pitch}  {:.0} km/h{location}  {fielder}",
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

    pub fn summary(&mut self, record: Record) -> Option<Action> {
        let s = self.scale;
        let x = self.area.center().x - 195.0 * s;
        let y = self.area.center().y - 192.5 * s;
        self.panel(Rect::new(x, y, 390.0 * s, 385.0 * s));
        self.fill(x, y, 390.0 * s, 4.0 * s, AMBER);
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
            &format!("安打  {}    本塁打  {}", record.fair, record.home_runs),
            x + 23.0 * s,
            y + 189.0 * s,
            21.0,
            MUTED,
        );
        self.text(
            &format!("二塁打  {}    三塁打  {}", record.doubles, record.triples),
            x + 23.0 * s,
            y + 216.0 * s,
            18.0,
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
    let timing = if grade.timing < -BARREL_TIMING {
        "早い"
    } else if grade.timing > BARREL_TIMING {
        "遅い"
    } else {
        "適正"
    };
    let offset = grade.offset / BARREL_RADIUS;
    let position = if offset.length() <= 1.0 {
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
