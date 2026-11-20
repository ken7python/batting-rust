mod batting;
mod defense;
mod display;
mod flight;
mod park;
mod pitch;
mod play;
mod runner;
mod scene;
mod session;
mod sound;
mod ui;

use defense::Defense;
use display::DisplayProfile;
use flight::Outcome;
use macroquad::input::utils::{register_input_subscriber, repeat_all_miniquad_input};
use macroquad::miniquad::{EventHandler, TouchPhase};
use macroquad::prelude::*;
use park::Park;
use pitch::Delivery;
use play::{Phase, Play, ResultCard};
use scene::*;
use session::{Mode, Record, Session};
use sound::Sounds;
use ui::{Action, PreviewTiming, Settings, Ui};

const NATIVE_TOUCH: bool = cfg!(any(target_os = "ios", target_os = "android"));

enum Screen {
    Menu,
    Game(Box<Session>),
    Summary(Box<Record>),
}

#[derive(Default)]
struct FrameInput {
    lost: bool,
    away: bool,
    drag_from: Option<Vec2>,
    actions: Vec<BatInput>,
    finger: Option<u64>,
}

enum BatInput {
    Press(Vec2),
    Drag(Vec2),
    Release,
    Cancel,
}

impl FrameInput {
    fn native_touch(&mut self, phase: TouchPhase, id: u64, point: Vec2) {
        match phase {
            TouchPhase::Started if self.finger.is_none() && !self.away => {
                self.finger = Some(id);
                self.mouse_button_down_event(MouseButton::Left, point.x, point.y);
            }
            TouchPhase::Moved if self.finger == Some(id) => {
                self.mouse_motion_event(point.x, point.y);
            }
            TouchPhase::Ended if self.finger == Some(id) => {
                self.finger = None;
                self.mouse_button_up_event(MouseButton::Left, point.x, point.y);
            }
            TouchPhase::Cancelled if self.finger == Some(id) => {
                self.finger = None;
                self.drag_from = None;
                self.actions.push(BatInput::Cancel);
            }
            _ => {}
        }
    }
}

impl EventHandler for FrameInput {
    fn update(&mut self) {}
    fn draw(&mut self) {}
    fn window_minimized_event(&mut self) {
        self.lost = true;
        self.away = true;
        self.drag_from = None;
        self.finger = None;
        self.actions.clear();
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

    fn touch_event(&mut self, phase: TouchPhase, id: u64, x: f32, y: f32) {
        if NATIVE_TOUCH {
            self.native_touch(phase, id, vec2(x, y));
        } else if phase == TouchPhase::Cancelled {
            self.drag_from = None;
            self.actions.push(BatInput::Cancel);
        }
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Batting Club".into(),
        window_width: 1120,
        window_height: 720,
        high_dpi: true,
        sample_count: if NATIVE_TOUCH { 1 } else { 4 },
        #[cfg(target_os = "ios")]
        platform: macroquad::miniquad::conf::Platform {
            apple_gfx_api: macroquad::miniquad::conf::AppleGfxApi::Metal,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn run() {
    macroquad::Window::from_config(window_conf(), game());
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn quad_main() {
    run();
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn batting_ios_main() {
    run();
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn batting_ios_view() -> *mut std::ffi::c_void {
    macroquad::miniquad::window::apple_view().cast()
}

#[cfg(target_os = "ios")]
unsafe extern "C" {
    fn batting_ios_metrics(insets: *mut f32) -> std::ffi::c_int;
}

fn safe_viewport(_input: &mut FrameInput) -> Rect {
    let width = screen_width();
    let height = screen_height();
    #[cfg(target_os = "ios")]
    {
        let mut insets = [0.0; 4];
        // The launcher writes four framebuffer insets and consumes the pause notification.
        let flags = unsafe { batting_ios_metrics(insets.as_mut_ptr()) };
        if flags & 2 != 0 || (flags & 1 == 0 && !_input.away) {
            _input.window_minimized_event();
        }
        if flags & 1 != 0 && _input.away {
            _input.window_restored_event();
        }
        let [left, top, right, bottom] = insets.map(|inset| inset / screen_dpi_scale());
        Rect::new(
            left,
            top,
            (width - left - right).max(1.0),
            (height - top - bottom).max(1.0),
        )
    }
    #[cfg(not(target_os = "ios"))]
    Rect::new(0.0, 0.0, width, height)
}

async fn game() {
    let font = load_ttf_font_from_bytes(include_bytes!("../assets/BattingSans.ttf"))
        .expect("bundled UI font must be a valid TTF");
    let mut sounds = Sounds::load().await;
    let mut audio_started = false;
    let mut screen = Screen::Menu;
    let mut settings = Settings::default();
    let subscriber = register_input_subscriber();
    let mut input = FrameInput::default();
    let mut previous_size = Vec2::ZERO;
    let display = DisplayProfile::current();
    if NATIVE_TOUCH {
        simulate_mouse_with_touch(false);
    }

    loop {
        repeat_all_miniquad_input(&mut input, subscriber);
        let viewport = safe_viewport(&mut input);
        let dpi = screen_dpi_scale();
        let size = vec2(screen_width(), screen_height());
        if NATIVE_TOUCH && previous_size != Vec2::ZERO && previous_size != size {
            input.drag_from = None;
            input.finger = None;
            input.actions.clear();
            input.actions.push(BatInput::Cancel);
        }
        let pointer = if NATIVE_TOUCH {
            input.actions.iter().find_map(|action| match *action {
                BatInput::Press(point) => Some(point / dpi),
                _ => None,
            })
        } else {
            is_mouse_button_pressed(MouseButton::Left).then(|| Vec2::from(mouse_position()))
        };
        audio_started |= pointer.is_some();
        let dt = f64::from(get_frame_time().min(0.1));
        let camera = batter_camera(display);
        let mut ui = Ui::new(&font, viewport, pointer, display);

        if let Screen::Game(session) = &mut screen {
            if input.lost {
                session.pause();
            } else if is_key_pressed(KeyCode::Escape) {
                if session.history_open {
                    session.history_open = false;
                } else if session.paused {
                    session.paused = false;
                } else {
                    session.pause();
                }
            } else if !session.paused && session.play.can_swing() {
                let aim = &mut session.play.aim;
                let at = session.play.time + dt;
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
                        BatInput::Cancel => aim.held = false,
                        _ => {}
                    }
                }
                if !(if NATIVE_TOUCH {
                    input.finger.is_some()
                } else {
                    is_mouse_button_down(MouseButton::Left)
                }) {
                    aim.held = false;
                }
            } else if !session.paused
                && let Phase::Flight(flight) = &mut session.play.phase
                && flight.can_boost_runner()
                && input
                    .actions
                    .iter()
                    .any(|action| matches!(*action, BatInput::Press(p) if !ui.pause_rect().contains(p / dpi)))
            {
                flight.runner.tap();
            }
            let was_pitched = matches!(session.play.phase, Phase::Pitch);
            let was_winding_up = was_pitched && session.play.time < 0.0;
            let was_resolved = matches!(session.play.phase, Phase::Result(_));
            let fence_was_cleared =
                matches!(&session.play.phase, Phase::Flight(flight) if flight.home_run);
            let was_fielded =
                matches!(&session.play.phase, Phase::Flight(flight) if flight.returning.is_some());
            let was_thrown = matches!(&session.play.phase, Phase::Flight(flight)
                if flight.returning.is_some_and(|returning| returning.airborne()));
            let summary = session.tick(dt);
            if settings.effects {
                if was_winding_up && session.play.time >= 0.0 {
                    sounds.pitch();
                }
                if let Phase::Flight(flight) = &session.play.phase {
                    if was_pitched {
                        sounds.contact(flight.strong);
                    }
                    if flight.home_run && !fence_was_cleared {
                        sounds.result(Outcome::Homer);
                    }
                    if let Some(returning) = flight.returning {
                        if !was_fielded {
                            sounds.glove();
                        }
                        if !was_thrown && returning.airborne() {
                            sounds.pitch();
                        }
                    }
                }
                if !was_resolved
                    && let Phase::Result(card) = &session.play.phase
                    && !(fence_was_cleared && card.outcome == Outcome::Homer)
                {
                    if card.fielding.as_ref().is_some_and(|view| {
                        view.returning.is_some() || matches!(card.outcome, Outcome::FlyOut(_))
                    }) {
                        sounds.glove();
                    }
                    sounds.result(card.outcome);
                }
            }
            if let Some(record) = summary {
                screen = Screen::Summary(Box::new(record));
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
                draw_play(&run.play, &camera, settings.preview, display);
                ui.game(run)
            }
            Screen::Summary(record) => {
                draw_idle(&camera, settings.delivery, settings.park);
                ui.summary(record)
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
                | Action::Preview(_)
                | Action::Sensitivity
                | Action::Hand
                | Action::Form
                | Action::Park
                | Action::Repertoire
                | Action::Music
                | Action::Effects
                | Action::BreakSize => settings.change(action),
                Action::Pause | Action::Resume | Action::History => {
                    if let Screen::Game(run) = &mut screen {
                        match action {
                            Action::Pause => run.pause(),
                            Action::Resume => {
                                run.paused = false;
                                run.history_open = false;
                            }
                            Action::History => run.history_open = !run.history_open,
                            _ => unreachable!(),
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
    fielders(&Defense::new(park), 0.0, None, None, 0.0);
    pitcher(-batting::WINDUP, delivery);
    set_default_camera();
}

fn draw_play(play: &Play, camera: &Camera3D, preview: PreviewTiming, display: DisplayProfile) {
    let mut ball_marker = None;
    match &play.phase {
        Phase::Flight(flight) => {
            let follow = flight.returning.as_ref().map_or_else(
                || {
                    if flight.confident {
                        home_run_camera(flight.position, flight.velocity, flight.elapsed, display)
                    } else {
                        flight_camera(flight.position, flight.velocity, flight.elapsed, display)
                    }
                },
                |returning| return_camera(returning, &flight.runner),
            );
            set_camera(&follow);
            field(play.park);
            let catch = flight
                .defense
                .chasing
                .filter(|&fielder| {
                    !flight.home_run
                        && flight.returning.is_none()
                        && flight.ground_contact.is_none()
                        && flight.velocity.y < 0.0
                        && (0.45..=2.65).contains(&flight.position.y)
                        && flight
                            .defense
                            .position(fielder)
                            .distance(flight.position.xz())
                            < 1.25
                })
                .map(|fielder| (fielder, flight.position));
            fielders(
                &flight.defense,
                flight.elapsed,
                flight.returning.as_ref(),
                catch,
                0.0,
            );
            if flight.confident {
                confident_runner(&flight.runner, flight.elapsed);
            } else {
                batter_runner(&flight.runner, flight.elapsed, true);
            }
            if let Some(contact) = flight.ground_contact {
                ground_impact(contact, flight.elapsed);
            }
            if let Some(returning) = flight.returning {
                if returning.airborne() {
                    ball_marker = visible_ball(&follow, flight.position).map(|p| (p, SKYBLUE));
                }
            } else {
                batted_ball_shadow(flight.position);
                if flight.confident {
                    ball_marker = visible_ball(&follow, flight.position).map(|p| (p, GOLD));
                }
                let trail_step = flight.elapsed.min(0.13) / 7.0;
                let trail_length = if flight.ground_contact.is_some() {
                    0
                } else {
                    7
                };
                for i in 0..trail_length {
                    let start = flight.position - flight.velocity * (i as f32 * trail_step);
                    let end = start - flight.velocity * trail_step;
                    let alpha = 0.9 - i as f32 * 0.11;
                    let color = if flight.confident {
                        Color::new(1.0, 0.73, 0.25, alpha)
                    } else {
                        Color::new(1.0, 0.88, 0.58, alpha)
                    };
                    draw_line_3d(start, end, color);
                }
            }
            ball(flight.position);
        }
        Phase::Result(ResultCard {
            fielding: Some(view),
            outcome,
            ..
        }) => {
            let follow = view.returning.as_ref().map_or_else(
                || {
                    if *outcome == Outcome::Homer {
                        home_run_camera(view.ball, view.velocity, view.elapsed, display)
                    } else {
                        flight_camera(view.ball, view.velocity, view.elapsed, display)
                    }
                },
                |returning| return_camera(returning, &view.runner),
            );
            set_camera(&follow);
            field(play.park);
            let catch = match *outcome {
                Outcome::FlyOut(fielder) => Some((fielder, view.ball)),
                _ => None,
            };
            fielders(
                &view.defense,
                view.elapsed,
                view.returning.as_ref(),
                catch,
                play.result_time as f32,
            );
            if !matches!(outcome, Outcome::Foul | Outcome::Homer) {
                batter_runner(&view.runner, view.elapsed, false);
            }
            if let Some(contact) = view.ground_contact {
                ground_impact(contact, view.elapsed + play.result_time as f32);
                if view.returning.is_none() {
                    batted_ball_shadow(view.ball);
                }
            }
            ball(view.ball);
        }
        _ => {
            set_camera(camera);
            field(play.park);
            fielders(&Defense::new(play.park), 0.0, None, None, 0.0);
            pitcher(play.time, play.pitch.delivery);
            bat(play.time, &play.aim);
            if matches!(play.phase, Phase::Pitch) && play.time >= 0.0 {
                ball(play.pitch.position(play.time));
            }
        }
    }
    set_default_camera();
    if let Some((point, color)) = ball_marker {
        let scale = marker_scale(screen_width(), screen_height(), display);
        draw_circle(
            point.x,
            point.y,
            5.0 * scale,
            Color::new(0.08, 0.17, 0.19, 0.8),
        );
        draw_circle(point.x, point.y, 3.0 * scale, WHITE);
        draw_circle_lines(point.x, point.y, 7.0 * scale, 1.6 * scale, color);
    }
    match &play.phase {
        Phase::Pitch => {
            reticle(camera, &play.aim);
            if preview.visible(play.time) {
                let predicted = play.pitch.preview_point(play.time);
                let p = project(camera, predicted);
                let radius = marker_scale(screen_width(), screen_height(), display) * 6.0;
                let color = if predicted.z < 0.0 { GOLD } else { SKYBLUE };
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

#[cfg(test)]
mod input_tests {
    use super::*;

    #[test]
    fn a_secondary_finger_cannot_release_or_move_the_bat() {
        let mut input = FrameInput::default();
        input.native_touch(TouchPhase::Started, 1, Vec2::ZERO);
        input.native_touch(TouchPhase::Started, 2, Vec2::Y);
        input.native_touch(TouchPhase::Moved, 2, Vec2::X);
        input.native_touch(TouchPhase::Ended, 2, Vec2::X);
        assert!(matches!(input.actions.as_slice(), [BatInput::Press(_)]));
        input.native_touch(TouchPhase::Ended, 1, Vec2::X);
        assert!(matches!(input.actions.last(), Some(BatInput::Release)));
    }

    #[test]
    fn a_cancelled_or_backgrounded_touch_cannot_swing_on_lift() {
        let mut input = FrameInput::default();
        input.native_touch(TouchPhase::Started, 1, Vec2::ZERO);
        input.native_touch(TouchPhase::Cancelled, 1, Vec2::ZERO);
        input.native_touch(TouchPhase::Ended, 1, Vec2::ZERO);
        assert!(matches!(input.actions.last(), Some(BatInput::Cancel)));
        input.native_touch(TouchPhase::Started, 2, Vec2::ZERO);
        input.window_minimized_event();
        input.window_restored_event();
        input.native_touch(TouchPhase::Ended, 2, Vec2::ZERO);
        assert!(input.actions.is_empty());
    }
}
