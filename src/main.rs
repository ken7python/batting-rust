mod batting;
mod defense;
mod flight;
mod park;
mod pitch;
mod play;
mod scene;
mod session;
mod sound;
mod ui;

use defense::Defense;
use flight::Outcome;
use macroquad::input::utils::{register_input_subscriber, repeat_all_miniquad_input};
use macroquad::miniquad::EventHandler;
use macroquad::prelude::*;
use park::Park;
use pitch::Delivery;
use play::{Phase, Play, ResultCard};
use scene::*;
use session::{Mode, Record, Session};
use sound::Sounds;
use ui::{Action, Settings, Ui};

enum Screen {
    Menu,
    Game(Box<Session>),
    Summary(Record),
}

#[derive(Default)]
struct FrameInput {
    lost: bool,
    away: bool,
    drag_from: Option<Vec2>,
    actions: Vec<BatInput>,
}

enum BatInput {
    Press(Vec2),
    Drag(Vec2),
    Release,
}

impl EventHandler for FrameInput {
    fn update(&mut self) {}
    fn draw(&mut self) {}
    fn window_minimized_event(&mut self) {
        self.lost = true;
        self.away = true;
        self.drag_from = None;
    }

    fn window_restored_event(&mut self) {
        self.away = false;
    }

    fn mouse_button_down_event(&mut self, btn: MouseButton, x: f32, y: f32) {
        if btn == MouseButton::Left {
            let position = vec2(x, y);
            self.drag_from = Some(position);
            self.actions.push(BatInput::Press(position));
        }
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        if let Some(from) = &mut self.drag_from {
            let position = vec2(x, y);
            self.actions.push(BatInput::Drag(position - *from));
            *from = position;
        }
    }

    fn mouse_button_up_event(&mut self, btn: MouseButton, x: f32, y: f32) {
        if btn == MouseButton::Left {
            self.mouse_motion_event(x, y);
            self.drag_from = None;
            self.actions.push(BatInput::Release);
        }
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Batting Club".into(),
        window_width: 1120,
        window_height: 720,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let font = load_ttf_font_from_bytes(include_bytes!("../assets/BattingSans.ttf"))
        .expect("bundled UI font must be a valid TTF");
    let mut sounds = Sounds::load().await;
    let mut audio_started = false;
    let mut screen = Screen::Menu;
    let mut settings = Settings::default();
    let subscriber = register_input_subscriber();
    let mut input = FrameInput::default();
    let mut previous_size = Vec2::ZERO;

    loop {
        repeat_all_miniquad_input(&mut input, subscriber);
        audio_started |= is_mouse_button_pressed(MouseButton::Left);
        let dt = f64::from(get_frame_time().min(0.1));
        let camera = batter_camera();
        let size = vec2(screen_width(), screen_height());
        let mut ui = Ui::new(&font);

        if let Screen::Game(session) = &mut screen {
            if input.lost {
                session.pause();
            } else if is_key_pressed(KeyCode::Escape) {
                if session.paused {
                    session.paused = false;
                } else {
                    session.pause();
                }
            } else if !session.paused && session.play.can_swing() {
                let aim = &mut session.play.aim;
                let at = session.play.time + dt;
                let dpi = screen_dpi_scale();
                for action in &input.actions {
                    match *action {
                        BatInput::Press(p) if !ui.pause_rect().contains(p / dpi) => aim.press(),
                        BatInput::Drag(delta) if size == previous_size => {
                            aim.drag(aim_movement(&camera, delta / dpi) * settings.sensitivity());
                        }
                        BatInput::Release => {
                            let swung = at < session.play.pitch.end_time() && aim.release(at);
                            if swung && settings.effects {
                                sounds.swing();
                            }
                        }
                        _ => {}
                    }
                }
                if !is_mouse_button_down(MouseButton::Left) {
                    aim.held = false;
                }
            }
            let was_pitched = matches!(session.play.phase, Phase::Pitch);
            let was_winding_up = was_pitched && session.play.time < 0.0;
            let was_resolved = matches!(session.play.phase, Phase::Result(_));
            let fence_was_cleared =
                matches!(&session.play.phase, Phase::Flight(flight) if flight.cleared_fence);
            let summary = session.tick(dt);
            if settings.effects {
                if was_winding_up && session.play.time >= 0.0 {
                    sounds.pitch();
                }
                if let Phase::Flight(flight) = &session.play.phase {
                    if was_pitched {
                        sounds.contact(flight.strong);
                    }
                    if flight.cleared_fence && !fence_was_cleared {
                        sounds.result(Outcome::Homer);
                    }
                }
                if !was_resolved
                    && let Phase::Result(card) = &session.play.phase
                    && !(fence_was_cleared && card.outcome == Outcome::Homer)
                {
                    sounds.result(card.outcome);
                }
            }
            if let Some(record) = summary {
                screen = Screen::Summary(record);
            }
        }
        input.lost = false;
        input.actions.clear();
        previous_size = size;
        show_mouse(!matches!(&screen, Screen::Game(run) if run.play.aim.held));

        let action = match &screen {
            Screen::Menu => {
                draw_idle(&camera, settings.delivery, settings.park);
                ui.menu(&settings)
            }
            Screen::Game(run) => {
                draw_play(&run.play, &camera, settings.preview);
                ui.game(run)
            }
            Screen::Summary(record) => {
                draw_idle(&camera, settings.delivery, settings.park);
                ui.summary(*record)
            }
        };

        ui.draw();
        if let Some(action) = action {
            match action {
                Action::Challenge => {
                    screen = Screen::Game(Box::new(Session::new(
                        Mode::Challenge,
                        settings.delivery,
                        settings.speed,
                        settings.park,
                        settings.pitch_mix,
                    )))
                }
                Action::Practice => {
                    screen = Screen::Game(Box::new(Session::new(
                        settings.practice(),
                        settings.delivery,
                        settings.speed,
                        settings.park,
                        settings.pitch_mix,
                    )))
                }
                Action::Menu => screen = Screen::Menu,
                Action::Pitch
                | Action::Speed(_)
                | Action::Preview
                | Action::Sensitivity
                | Action::Hand
                | Action::Form
                | Action::Park
                | Action::Repertoire
                | Action::Music
                | Action::Effects
                | Action::BreakSize => settings.change(action),
                Action::Pause | Action::Resume => {
                    if let Screen::Game(run) = &mut screen {
                        if matches!(action, Action::Pause) {
                            run.pause();
                        } else {
                            run.paused = false;
                        }
                    }
                }
            }
        }
        let paused = matches!(&screen, Screen::Game(session) if session.paused);
        sounds.set_music(audio_started && settings.music && !paused && !input.away);
        next_frame().await;
    }
}

fn draw_idle(camera: &Camera3D, delivery: Delivery, park: Park) {
    set_camera(camera);
    field(park);
    fielders(&Defense::new(park));
    pitcher(-batting::WINDUP, delivery);
    set_default_camera();
}

fn draw_play(play: &Play, camera: &Camera3D, preview: bool) {
    match &play.phase {
        Phase::Flight(flight) => {
            let follow = flight_camera(flight.position, flight.velocity, flight.elapsed);
            set_camera(&follow);
            field(play.park);
            fielders(&flight.defense);
            let trail_step = flight.elapsed.min(0.13) / 7.0;
            for i in 0..7 {
                let start = flight.position - flight.velocity * (i as f32 * trail_step);
                let end = start - flight.velocity * trail_step;
                draw_line_3d(
                    start,
                    end,
                    Color::new(1.0, 0.88, 0.58, 0.9 - i as f32 * 0.11),
                );
            }
            ball(flight.position);
        }
        Phase::Result(ResultCard {
            fielding: Some(view),
            landing: Some(position),
            ..
        }) => {
            let follow = flight_camera(*position, view.velocity, view.elapsed);
            set_camera(&follow);
            field(play.park);
            fielders(&view.defense);
            ball(*position);
        }
        _ => {
            set_camera(camera);
            field(play.park);
            fielders(&Defense::new(play.park));
            pitcher(play.time, play.pitch.delivery);
            bat(play.time, &play.aim);
            if matches!(play.phase, Phase::Pitch) && play.time >= 0.0 {
                ball(play.pitch.position(play.time));
            }
        }
    }
    set_default_camera();
    match &play.phase {
        Phase::Pitch => {
            reticle(camera, &play.aim);
            if preview {
                let p = project(camera, play.pitch.preview_point());
                let radius = (screen_height() / 720.0).clamp(0.75, 2.0) * 5.0;
                let color = if play.pitch.bounce_time.is_some() {
                    GOLD
                } else {
                    SKYBLUE
                };
                draw_circle(p.x, p.y, radius + 1.5, BLACK);
                draw_circle(p.x, p.y, radius, color);
            }
        }
        Phase::Result(card) if card.fielding.is_none() => {
            if play.pitch.bounce_time.is_none() {
                let p = project(camera, on_plate(play.pitch.crossing));
                draw_circle(p.x, p.y, 4.5, GOLD);
            }
            if let Some(swing) = play.aim.live_swing() {
                let p = project(camera, on_plate(swing.aim));
                draw_line(p.x - 7.0, p.y, p.x + 7.0, p.y, 2.0, AIM_GREEN);
                draw_line(p.x, p.y - 7.0, p.x, p.y + 7.0, 2.0, AIM_GREEN);
            }
        }
        Phase::Flight(_) | Phase::Result(_) => {}
    }
}
