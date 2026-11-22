use macroquad::prelude::*;

use crate::batting::{BARREL_RADIUS, Grade, on_time};
use crate::display::DisplayProfile;
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
const PREVIEW_LEAD: f64 = 0.50;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PreviewTiming {
    BeforeRelease,
    AfterRelease,
}

impl PreviewTiming {
    fn label(self) -> &'static str {
        match self {
            Self::BeforeRelease => "リリース前",
            Self::AfterRelease => "リリース後",
        }
    }

    pub fn visible(self, pitch_time: f64) -> bool {
        let start = match self {
            Self::BeforeRelease => -PREVIEW_LEAD,
            Self::AfterRelease => 0.0,
        };
        pitch_time >= start
    }
}

fn hit_accent(outcome: Outcome) -> Option<Color> {
    match outcome {
        Outcome::Fair | Outcome::InfieldHit { .. } => Some(AIM_CALL_COLOR),
        Outcome::HardFair => Some(Color::from_rgba(112, 239, 226, 255)),
        Outcome::Double => Some(Color::from_rgba(142, 200, 255, 255)),
        Outcome::Triple => Some(Color::from_rgba(255, 152, 107, 255)),
        Outcome::Homer => Some(AMBER),
        _ => None,
    }
}

#[derive(Clone, Copy)]
pub enum Action {
    Challenge,
    Practice,
    Pitch,
    Speed(PitchSpeed),
    Preview(PreviewTiming),
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
    History,
    Menu,
}

pub struct Settings {
    pitch: usize,
    pub speed: PitchSpeed,
    pub preview: PreviewTiming,
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
            preview: PreviewTiming::BeforeRelease,
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
        [0.875, 1.25, 1.75][self.sensitivity]
    }

    pub fn change(&mut self, action: Action) {
        match action {
            Action::Pitch => self.pitch = (self.pitch + 1) % (PitchKind::ALL.len() + 1),
            Action::Speed(speed) => self.speed = speed,
            Action::Preview(timing) => self.preview = timing,
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
    display: DisplayProfile,
    mouse: Vec2,
    pressed: bool,
    draws: Vec<Draw>,
}

fn fit(area: Rect, width: f32, height: f32) -> f32 {
    (area.w / width).min(area.h / height)
}

fn menu_layout(area: Rect, display: DisplayProfile) -> (bool, Vec2, f32) {
    let vertical = fit(area, 456.0, 656.0);
    let wide = fit(area, 804.0, 346.0);
    if wide > vertical {
        (true, vec2(780.0, 322.0), display.ui_scale(wide))
    } else {
        (false, vec2(432.0, 632.0), display.ui_scale(vertical))
    }
}

impl<'a> Ui<'a> {
    pub fn new(font: &'a Font, area: Rect, press: Option<Vec2>, display: DisplayProfile) -> Self {
        Self {
            font,
            scale: display.ui_scale(fit(area, 440.0, 385.0)),
            area,
            display,
            mouse: press.unwrap_or_else(|| {
                if display.mobile() {
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
        let padding = 8.0 * self.scale;
        let available = rect.w - padding * 2.0;
        let mut pixels = (20.0 * self.scale).round().max(1.0) as u16;
        let mut width = measure_text(title, Some(self.font), pixels, 1.0).width;
        while width > available && pixels > 1 {
            pixels -= 1;
            width = measure_text(title, Some(self.font), pixels, 1.0).width;
        }
        self.draws.push(Draw::Text {
            label: title.into(),
            position: vec2(
                rect.center().x - width * 0.5,
                rect.center().y + f32::from(pixels) * 0.35,
            ),
            size: pixels,
            color: if primary { INK } else { CREAM },
        });
        hovered && self.pressed
    }

    pub fn menu(&mut self, settings: &Settings) -> Option<Action> {
        let (wide, size, s) = menu_layout(self.area, self.display);
        self.scale = s;
        let origin = self.area.center() - size * s * 0.5;
        let rect = |x, y, w, h| Rect::new(origin.x + x * s, origin.y + y * s, w * s, h * s);
        self.panel(rect(0.0, 0.0, size.x, size.y));
        self.fill(origin.x, origin.y, size.x * s, 4.0 * s, AMBER);
        self.text(
            "BATTING CLUB",
            origin.x + 20.0 * s,
            origin.y + 56.0 * s,
            43.0,
            CREAM,
        );
        self.text(
            "ドラッグで狙い、離して打つ。",
            origin.x + 20.0 * s,
            origin.y + 92.0 * s,
            20.0,
            MUTED,
        );

        let main_width = if wide { 318.0 } else { 392.0 };
        let challenge = self.button(rect(20.0, 112.0, main_width, 54.0), "10球チャレンジ", true);
        let practice = self.button(rect(20.0, 176.0, main_width, 54.0), "練習を始める", false);
        self.text(
            "球速",
            origin.x + 20.0 * s,
            origin.y + 256.0 * s,
            18.0,
            MUTED,
        );
        let mut action = if challenge {
            Some(Action::Challenge)
        } else {
            practice.then_some(Action::Practice)
        };
        let speed_width = (main_width - 18.0) / 4.0;
        for (index, speed) in PitchSpeed::ALL.into_iter().enumerate() {
            let bounds = rect(
                20.0 + index as f32 * (speed_width + 6.0),
                266.0,
                speed_width,
                46.0,
            );
            if self.button(bounds, speed.label(), speed == settings.speed) {
                action = Some(Action::Speed(speed));
            }
        }

        let pitch = settings
            .selected_pitch()
            .map_or("ランダム", PitchKind::label);
        let sensitivity = ["低い", "標準", "高い"][settings.sensitivity];
        let options = [
            (
                format!("予測  {}", PreviewTiming::BeforeRelease.label()),
                Action::Preview(PreviewTiming::BeforeRelease),
            ),
            (
                format!("予測  {}", PreviewTiming::AfterRelease.label()),
                Action::Preview(PreviewTiming::AfterRelease),
            ),
            (format!("球種  {pitch}"), Action::Pitch),
            (format!("感度  {sensitivity}"), Action::Sensitivity),
            (settings.delivery.hand.label().into(), Action::Hand),
            (settings.delivery.form.label().into(), Action::Form),
            (
                format!("{} {:.0}m", settings.park.label(), settings.park.fence().0),
                Action::Park,
            ),
            (
                format!("配球  {}", settings.pitch_mix.repertoire.label()),
                Action::Repertoire,
            ),
            (
                format!("変化量  {}", settings.pitch_mix.break_size.label()),
                Action::BreakSize,
            ),
            (
                if settings.music {
                    "BGM オン"
                } else {
                    "BGM オフ"
                }
                .into(),
                Action::Music,
            ),
            (
                if settings.effects {
                    "効果音 オン"
                } else {
                    "効果音 オフ"
                }
                .into(),
                Action::Effects,
            ),
        ];
        let (x, y, width) = if wide {
            (358.0, 14.0, 196.0)
        } else {
            (20.0, 327.0, 192.0)
        };
        for (index, (label, choice)) in options.into_iter().enumerate() {
            let bounds = rect(
                x + (index % 2) as f32 * (width + 8.0),
                y + (index / 2) as f32 * 50.0,
                width,
                44.0,
            );
            let selected = matches!(choice, Action::Preview(timing) if timing == settings.preview);
            if self.button(bounds, &label, selected) {
                action = Some(choice);
            }
        }
        action
    }

    pub fn pause_rect(&self) -> Rect {
        let s = self.scale;
        Rect::new(
            self.area.right() - 140.0 * s,
            self.area.y + 20.0 * s,
            120.0 * s,
            52.0 * s,
        )
    }

    fn corner_panel_scale(&self) -> f32 {
        if self.display.mobile() && self.area.w > self.area.h && self.area.w < 720.0 {
            self.scale.min(self.area.w / 890.0)
        } else {
            self.scale
        }
    }

    pub fn game(&mut self, session: &Session) -> Option<Action> {
        let s = self.scale;
        let x = self.area.x + 20.0 * s;
        let mut action = None;
        let corner_scale = self.corner_panel_scale();
        self.scale = corner_scale;
        {
            let s = corner_scale;
            let x = self.area.x + 20.0 * s;
            let y = self.area.y + 20.0 * s;
            let shown_pitch = session.record.completed
                + u32::from(!matches!(session.play.phase, Phase::Result(_)));
            let (mode, count) = match session.mode {
                Mode::Challenge => (
                    "10球チャレンジ",
                    format!("{shown_pitch:02} / {CHALLENGE_LENGTH:02}"),
                ),
                Mode::Practice { .. } => ("練習", format!("{shown_pitch:02} 球")),
            };
            self.panel(Rect::new(x, y, 276.0 * s, 81.0 * s));
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
        }
        self.scale = s;
        if !session.paused && self.button(self.pause_rect(), "一時停止", false) {
            action = Some(Action::Pause);
        }
        if let Phase::Result(card) = &session.play.phase {
            self.feedback(session, card);
            if let Some(color) = hit_accent(card.outcome) {
                let mut detail = card
                    .landing
                    .map_or_else(String::new, |point| format!("{:.0} m", point.xz().length()));
                if matches!(session.mode, Mode::Challenge) {
                    detail.push_str(&format!("    +{} 点", card.outcome.points()));
                }
                self.hit_banner(
                    card.outcome.label(),
                    &detail,
                    color,
                    session.play.result_time as f32,
                    card.outcome == Outcome::Homer,
                );
            }
            if session.play.result_time < 0.9 {
                match card.outcome {
                    Outcome::Foul => self.field_call("ファール", AMBER),
                    Outcome::LineOut(_) => {
                        self.field_call("ライナーアウト", Color::from_rgba(255, 151, 126, 255))
                    }
                    Outcome::FlyOut(_) | Outcome::GroundOut(_) | Outcome::BaseOut(_) => {
                        self.field_call("アウト", Color::from_rgba(255, 151, 126, 255));
                    }
                    _ => {}
                }
            }
            if let Some(view) = &card.fielding
                && view.returning.is_some()
            {
                self.runner_diamond(&view.runner, false, true);
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
                let label = if flight.home_run {
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
                if flight.home_run { AMBER } else { MUTED },
            );
            self.text(&detail, x + 13.0 * s, h - 30.0 * s, 26.0, CREAM);
            if !flight.home_run {
                self.runner_diamond(
                    &flight.runner,
                    !session.paused && flight.can_boost_runner(),
                    false,
                );
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
            } else if self.display.mobile() {
                "押したままドラッグ"
            } else {
                "左ボタンを押してドラッグ"
            };
            let h = self.area.bottom();
            self.scale = corner_scale;
            let x = self.area.x + 20.0 * corner_scale;
            self.panel(Rect::new(
                x,
                h - 59.0 * corner_scale,
                270.0 * corner_scale,
                40.0 * corner_scale,
            ));
            self.text(
                hint,
                x + 13.0 * corner_scale,
                h - 32.0 * corner_scale,
                17.0,
                CREAM,
            );
            self.scale = s;
        }

        if session.paused {
            self.fill(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(0.04, 0.10, 0.11, 0.63),
            );
            if session.history_open {
                return self
                    .paused_history(&session.record, matches!(session.mode, Mode::Challenge));
            }
            let x = self.area.center().x - 160.0 * s;
            let y = self.area.center().y - 137.0 * s;
            self.panel(Rect::new(x, y, 320.0 * s, 274.0 * s));
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
                "打席一覧",
                false,
            ) {
                action = Some(Action::History);
            }
            if self.button(
                Rect::new(x + 22.0 * s, y + 206.0 * s, 276.0 * s, 47.0 * s),
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
        let top = self.area.y
            + if self.area.w < 590.0 * s {
                86.0 * s
            } else {
                0.0
            };
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

    fn centered_text(&mut self, label: &str, x: f32, y: f32, size: f32, color: Color) {
        let width = measure_text(
            label,
            Some(self.font),
            (size * self.scale).round() as u16,
            1.0,
        )
        .width;
        self.text(label, x - width * 0.5, y, size, color);
    }

    fn hit_banner(&mut self, title: &str, detail: &str, color: Color, age: f32, homer: bool) {
        let s = self.scale;
        let center = self.area.center().x;
        let entry = (age / 0.24).clamp(0.0, 1.0);
        let settle = 1.0 - (1.0 - entry).powi(3);
        let width = (if homer { 404.0 } else { 330.0 }) * s;
        let x = center - width * 0.5;
        let y = self.area.y + (123.0 - 20.0 * (1.0 - settle)) * s;
        let height = 94.0 * s;
        let pulse = (1.0 - age / 0.72).max(0.0);
        if pulse > 0.0 {
            let origin = vec2(center, y + height * 0.5);
            let burst = 1.0 - pulse * pulse;
            let sparks = if homer { 24 } else { 14 };
            for index in 0..sparks {
                let theta = std::f32::consts::TAU * index as f32 / sparks as f32;
                let arrow = vec2(theta.cos(), theta.sin());
                let edge = origin
                    + arrow * vec2(width * (0.44 + 0.25 * burst), height * (0.4 + 0.36 * burst));
                let trail = edge - arrow * (9.0 + 16.0 * pulse) * s;
                let glow = Color {
                    a: 0.9 * pulse,
                    ..color
                };
                self.draws.push(Draw::Line(trail, edge, glow));
                self.draws
                    .push(Draw::Dot(edge, (1.5 + 2.0 * pulse) * s, glow));
            }
        }
        self.fill(x, y, width, height, Color::from_rgba(12, 30, 34, 250));
        self.draws
            .push(Draw::Border(Rect::new(x, y, width, height), color));
        self.fill(x, y, width, 4.0 * s, color);
        for side in [-1.0, 1.0] {
            let edge = center + side * (width * 0.5 - 16.0 * s);
            for stripe in 0..3 {
                let at = edge - side * stripe as f32 * 8.0 * s;
                self.draws.push(Draw::Line(
                    vec2(at, y + 18.0 * s),
                    vec2(at - side * 14.0 * s, y + 45.0 * s),
                    Color { a: 0.60, ..color },
                ));
            }
        }
        let title_size =
            (if homer { 42.0 } else { 40.0 }) + 7.0 * (entry * std::f32::consts::PI).sin();
        self.centered_text(title, center, y + 57.0 * s, title_size, color);
        self.centered_text(detail, center, y + 80.0 * s, 17.0, CREAM);
    }

    fn runner_diamond(&mut self, runner: &Runner, can_boost: bool, result: bool) {
        let clearance = if result && self.area.w < 700.0 * self.scale {
            128.0 * self.scale
        } else {
            0.0
        };
        let s = self.scale * if self.display.mobile() { 1.45 } else { 1.0 };
        let home = vec2(
            self.area.right() - 60.0 * s,
            self.area.bottom() - 41.0 * s - clearance,
        );
        let corner = 27.0 * s;
        let bases = [
            home,
            home + vec2(corner, -corner),
            home + vec2(0.0, -2.0 * corner),
            home + vec2(-corner, -corner),
        ];
        let panel = Rect::new(home.x - 48.0 * s, home.y - 64.0 * s, 96.0 * s, 76.0 * s);
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
        if can_boost && runner.boosted() {
            let pulse = (get_time() as f32 * 18.0).sin() * 0.5 + 0.5;
            self.draws.push(Draw::Dot(
                marker,
                (10.0 + pulse * 5.0) * s,
                Color::new(1.0, 0.78, 0.25, 0.20 + pulse * 0.20),
            ));
            for i in 1..=3 {
                let trail = marker - runner.direction() * (i as f32 * 5.0 * s);
                self.draws.push(Draw::Dot(
                    trail,
                    (4.0 - i as f32 * 0.7) * s,
                    Color::new(1.0, 0.78, 0.25, 0.6 / i as f32),
                ));
            }
        }
        self.draws.push(Draw::Dot(
            marker,
            if can_boost && runner.boosted() {
                7.0
            } else {
                6.0
            } * s,
            AMBER,
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

    fn history_table(&mut self, record: &Record, origin: Vec2, width: f32, show_points: bool) {
        let s = self.scale;
        let left = |fraction| origin.x + width * fraction;
        self.fill(
            origin.x,
            origin.y,
            width,
            25.0 * s,
            Color::from_rgba(25, 62, 62, 255),
        );
        for (fraction, label) in [
            (0.02, "打席"),
            (0.12, "球種"),
            (0.35, "球速"),
            (0.50, "結果"),
            (0.74, "飛距離"),
            (0.89, "得点"),
        ] {
            if show_points || label != "得点" {
                self.text(label, left(fraction), origin.y + 18.0 * s, 15.0, MUTED);
            }
        }
        if record.completed == 0 {
            self.centered_text(
                "まだ結果はありません",
                left(0.5),
                origin.y + 96.0 * s,
                20.0,
                MUTED,
            );
        }
        for (row, (number, entry)) in record.history().enumerate() {
            let top = origin.y + (27.0 + row as f32 * 26.0) * s;
            let background = if row % 2 == 0 {
                Color::from_rgba(29, 59, 58, 255)
            } else {
                INK
            };
            self.fill(origin.x, top, width, 24.0 * s, background);
            let baseline = top + 17.0 * s;
            self.text(&format!("{number:02}"), left(0.02), baseline, 16.0, MUTED);
            self.text(entry.pitch.label(), left(0.12), baseline, 16.0, CREAM);
            self.text(
                &format!("{:.0} km/h", entry.speed),
                left(0.35),
                baseline,
                15.0,
                CREAM,
            );
            let color = hit_accent(entry.outcome).unwrap_or(CREAM);
            if entry.outcome.is_hit() {
                self.fill(origin.x, top, 3.0 * s, 24.0 * s, color);
            }
            self.text(entry.outcome.label(), left(0.50), baseline, 16.0, color);
            let distance = entry
                .distance
                .map_or_else(|| "-".into(), |distance| format!("{distance:.0} m"));
            self.text(&distance, left(0.74), baseline, 15.0, MUTED);
            if show_points {
                self.text(
                    &entry.outcome.points().to_string(),
                    left(0.89),
                    baseline,
                    16.0,
                    color,
                );
            }
        }
    }

    fn history_panel(&mut self, height: f32) -> Rect {
        let width = if self.area.w / self.area.h < 1.15 {
            452.0
        } else {
            598.0
        };
        self.scale = self
            .display
            .ui_scale(fit(self.area, width + 16.0, height + 16.0));
        let size = vec2(width, height) * self.scale;
        let point = self.area.center() - size * 0.5;
        let panel = Rect::new(point.x, point.y, size.x, size.y);
        self.panel(panel);
        self.fill(panel.x, panel.y, panel.w, 4.0 * self.scale, AMBER);
        panel
    }

    fn paused_history(&mut self, record: &Record, show_points: bool) -> Option<Action> {
        let panel = self.history_panel(407.0);
        let s = self.scale;
        let x = panel.x + 14.0 * s;
        let y = panel.y;
        let width = panel.w - 28.0 * s;
        self.text("打席一覧", x, y + 36.0 * s, 25.0, CREAM);
        self.history_table(record, vec2(x, y + 48.0 * s), width, show_points);
        self.button(Rect::new(x, y + 345.0 * s, width, 48.0 * s), "戻る", true)
            .then_some(Action::History)
    }

    pub fn summary(&mut self, record: &Record) -> Option<Action> {
        let panel = self.history_panel(442.0);
        let s = self.scale;
        let x = panel.x + 14.0 * s;
        let y = panel.y;
        let width = panel.w - 28.0 * s;
        self.text("10球チャレンジ結果", x, y + 39.0 * s, 25.0, CREAM);
        self.centered_text(
            &format!("{} 点", record.points),
            x + width * 0.85,
            y + 40.0 * s,
            30.0,
            AMBER,
        );
        self.text(
            &format!(
                "安打  {}   二塁打  {}   三塁打  {}   本塁打  {}",
                record.fair, record.doubles, record.triples, record.home_runs
            ),
            x,
            y + 66.0 * s,
            17.0,
            MUTED,
        );
        self.history_table(record, vec2(x, y + 82.0 * s), width, true);
        let button_width = (width - 8.0 * s) * 0.5;
        let replay = self.button(
            Rect::new(x, y + 380.0 * s, button_width, 48.0 * s),
            "もう一度",
            true,
        );
        let menu = self.button(
            Rect::new(
                x + button_width + 8.0 * s,
                y + 380.0 * s,
                button_width,
                48.0 * s,
            ),
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
    let timing = if on_time(grade.timing) {
        "適正"
    } else if grade.timing < 0.0 {
        "早い"
    } else {
        "遅い"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_feedback_agrees_with_contact_at_both_edges_of_each_window() {
        use crate::batting::{CENTER, Contact, SWING_LEAD, Swing, grade};
        for sign in [-1.0, 1.0] {
            let off_time = if sign < 0.0 { "早い" } else { "遅い" };
            for (milliseconds, contact, label) in [
                (50, Contact::Barrel, "適正"),
                (51, Contact::Touch, off_time),
                (108, Contact::Touch, off_time),
                (109, Contact::Miss, off_time),
            ] {
                let swing = Swing {
                    time: 0.5 - SWING_LEAD + sign * f64::from(milliseconds) / 1000.0,
                    aim: CENTER,
                };
                let hit = grade(swing, 0.5, CENTER);
                assert_eq!(hit.contact, contact);
                assert!(feedback_label(hit).contains(label));
            }
        }
    }

    #[test]
    fn preview_modes_start_at_their_own_release_thresholds() {
        let mut settings = Settings::default();
        assert!(!settings.preview.visible(-crate::batting::WINDUP));
        assert!(!settings.preview.visible(-PREVIEW_LEAD - 0.001));
        assert!(settings.preview.visible(-PREVIEW_LEAD));
        assert!(settings.preview.visible(-0.01));
        assert!(settings.preview.visible(0.01));
        settings.change(Action::Preview(PreviewTiming::AfterRelease));
        assert!(!settings.preview.visible(-0.01));
        assert!(settings.preview.visible(0.0));
        assert!(settings.preview.visible(0.01));
        settings.change(Action::Preview(PreviewTiming::BeforeRelease));
        assert!(settings.preview.visible(-0.01));
    }

    #[test]
    fn desktop_menus_and_hud_keep_a_smaller_scale_at_the_same_resolution() {
        for (width, height) in [(1120.0, 720.0), (2560.0, 1440.0)] {
            let area = Rect::new(0.0, 0.0, width, height);
            let (_, _, desktop) = menu_layout(area, DisplayProfile::Desktop);
            let (_, _, mobile) = menu_layout(area, DisplayProfile::Mobile);
            assert_eq!(desktop, 1.0);
            assert!(mobile > desktop);
            let hud = fit(area, 440.0, 385.0);
            assert_eq!(DisplayProfile::Desktop.ui_scale(hud), 1.0);
            assert!(DisplayProfile::Mobile.ui_scale(hud) > desktop);
        }
    }

    #[test]
    fn menus_fit_small_phones_in_both_orientations_with_usable_targets() {
        for (width, height) in [
            (320.0, 568.0),
            (390.0, 844.0),
            (568.0, 320.0),
            (844.0, 390.0),
            (1024.0, 768.0),
            (1120.0, 720.0),
        ] {
            let area = Rect::new(0.0, 0.0, width, height);
            let (wide, size, scale) = menu_layout(area, DisplayProfile::Mobile);
            assert!(size.x * scale < width && size.y * scale < height);
            assert!(46.0 * scale >= 32.0);
            if (width - height).abs() > 240.0 && width > height {
                assert!(wide);
            }
        }
    }
}
