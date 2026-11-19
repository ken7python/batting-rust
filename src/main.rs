mod batting;
mod flight;
mod pitch;
mod play;
mod scene;
mod session;
mod sound;
mod ui;

use macroquad::input::utils::{register_input_subscriber, repeat_all_miniquad_input};
use macroquad::miniquad::EventHandler;
use macroquad::prelude::*;
use pitch::ThrowingHand;
use play::{Phase, Play};
use scene::*;
use session::{Mode, Record, Session};
use sound::Sounds;
use ui::{Action, Settings, Ui};

const SKY: Color = Color::from_rgba(147, 198, 218, 255);

enum Screen {
    Menu,
    Game(Session),
    Summary(Record),
}

#[derive(Default)]
struct Focus {
    lost: bool,
}

impl EventHandler for Focus {
    fn update(&mut self) {}
    fn draw(&mut self) {}
    fn window_minimized_event(&mut self) {
        self.lost = true;
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
    let sounds = Sounds::load().await;
    let mut screen = Screen::Menu;
    let mut settings = Settings::default();
    let subscriber = register_input_subscriber();
    let mut focus = Focus::default();
    let mut previous_mouse = Vec2::from(mouse_position());
    let mut previous_size = Vec2::ZERO;

    loop {
        repeat_all_miniquad_input(&mut focus, subscriber);
        let dt = f64::from(get_frame_time().min(0.1));
        let camera = batter_camera();
        let mouse = Vec2::from(mouse_position());
        let size = vec2(screen_width(), screen_height());
        let ui = Ui::new(&font);

        if let Screen::Game(session) = &mut screen {
            if focus.lost {
                session.pause();
            } else if is_key_pressed(KeyCode::Escape) {
                if session.paused {
                    session.paused = false;
                } else {
                    session.pause();
                }
            } else if !session.paused && session.play.can_swing() {
                let aim = &mut session.play.aim;
                if is_mouse_button_pressed(MouseButton::Left) {
                    if !ui.pause_rect().contains(mouse) {
                        aim.press();
                    }
                } else if size == previous_size {
                    aim.drag(
                        aim_movement(&camera, mouse - previous_mouse) * settings.sensitivity(),
                    );
                }
                let at = session.play.time + dt;
                if is_mouse_button_released(MouseButton::Left) && at < session.play.pitch.end_time()
                {
                    if aim.held {
                        sounds.swing();
                    }
                    aim.release(at);
                } else if !is_mouse_button_down(MouseButton::Left) {
                    aim.held = false;
                }
            }
            let was_pitched = matches!(session.play.phase, Phase::Pitch);
            let summary = session.tick(dt);
            if was_pitched && let Phase::Flight(flight) = &session.play.phase {
                sounds.contact(flight.strong);
            }
            if let Some(record) = summary {
                screen = Screen::Summary(record);
            }
        }
        focus.lost = false;
        previous_mouse = mouse;
        previous_size = size;
        show_mouse(!matches!(&screen, Screen::Game(run) if run.play.aim.held));

        clear_background(SKY);
        let action = match &screen {
            Screen::Menu => {
                draw_idle(&camera, settings.hand);
                ui.menu(&settings)
            }
            Screen::Game(run) => {
                draw_play(&run.play, &camera, run.mode.preview());
                ui.game(run)
            }
            Screen::Summary(record) => {
                draw_idle(&camera, settings.hand);
                ui.summary(*record)
            }
        };

        if let Some(action) = action {
            match action {
                Action::Challenge => {
                    screen = Screen::Game(Session::new(Mode::Challenge, settings.hand))
                }
                Action::Practice => {
                    screen = Screen::Game(Session::new(settings.practice(), settings.hand))
                }
                Action::Menu => screen = Screen::Menu,
                Action::Pitch
                | Action::Speed
                | Action::Preview
                | Action::Sensitivity
                | Action::Hand => settings.change(action),
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
        next_frame().await;
    }
}

fn draw_idle(camera: &Camera3D, hand: ThrowingHand) {
    set_camera(camera);
    field();
    pitcher(-batting::WINDUP, hand);
    set_default_camera();
}

fn draw_play(play: &Play, camera: &Camera3D, preview: bool) {
    match &play.phase {
        Phase::Flight(flight) => {
            let follow = flight_camera(flight.position, flight.velocity, flight.elapsed);
            set_camera(&follow);
            field();
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
        _ => {
            set_camera(camera);
            field();
            pitcher(play.time, play.pitch.hand);
            bat(play.time, &play.aim);
            if matches!(play.phase, Phase::Pitch) && play.time >= 0.0 {
                ball(play.pitch.position(play.time));
            }
        }
    }
    set_default_camera();
    match &play.phase {
        Phase::Pitch => {
            if preview && play.pitch.bounce_time.is_none() {
                let p = project(camera, on_plate(play.pitch.crossing));
                draw_circle_lines(p.x, p.y, 7.0, 1.5, SKYBLUE);
            }
            reticle(camera, &play.aim);
        }
        Phase::Result(_) => {
            if play.pitch.bounce_time.is_none() {
                let p = project(camera, on_plate(play.pitch.crossing));
                draw_circle(p.x, p.y, 4.5, GOLD);
            }
            if let Some(swing) = play.aim.swing {
                let p = project(camera, on_plate(swing.aim));
                draw_line(p.x - 7.0, p.y, p.x + 7.0, p.y, 2.0, WHITE);
                draw_line(p.x, p.y - 7.0, p.x, p.y + 7.0, 2.0, WHITE);
            }
        }
        Phase::Flight(_) => {}
    }
}
