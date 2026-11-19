mod batting;
mod flight;
mod pitch;
mod play;
mod scene;

use macroquad::prelude::*;
use pitch::Pitch;
use play::{Phase, Play};
use scene::*;

const SKY: Color = Color::from_rgba(147, 198, 218, 255);

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
    let mut play = Play::new(Pitch::random(None, true, 1.0));
    let mut previous_mouse = Vec2::from(mouse_position());
    let mut previous_size = Vec2::ZERO;

    loop {
        let dt = f64::from(get_frame_time().min(0.05));
        let camera = batter_camera();
        let mouse = Vec2::from(mouse_position());
        let size = vec2(screen_width(), screen_height());
        if play.can_swing() {
            if is_mouse_button_pressed(MouseButton::Left) {
                play.aim.press();
            } else if size == previous_size {
                play.aim.drag(aim_movement(&camera, mouse - previous_mouse));
            }
            if is_mouse_button_released(MouseButton::Left) && play.time + dt < play.pitch.end_time()
            {
                play.aim.release(play.time + dt);
            } else if !is_mouse_button_down(MouseButton::Left) {
                play.aim.held = false;
            }
        }
        previous_mouse = mouse;
        previous_size = size;
        play.advance(dt);
        if play.result_time > 1.25 {
            play.next_pitch(Pitch::random(None, false, 1.0));
        }

        draw_play(&play, &camera);
        draw_text("BATTING CLUB  /  PITCH DRILL", 28.0, 48.0, 25.0, DARKBLUE);
        draw_text(
            "Hold + drag to aim. Release to swing.",
            28.0,
            screen_height() - 26.0,
            22.0,
            WHITE,
        );
        if let Phase::Result(result) = &play.phase {
            let label = format!("{}  /  {}", play.pitch.kind.label(), result.outcome.label());
            let width = measure_text(&label, None, 28, 1.0).width;
            draw_text(
                &label,
                (screen_width() - width) * 0.5,
                screen_height() * 0.75,
                28.0,
                WHITE,
            );
        }
        next_frame().await;
    }
}

fn draw_play(play: &Play, camera: &Camera3D) {
    clear_background(SKY);
    match &play.phase {
        Phase::Flight(flight) => {
            let follow = flight_camera(flight.position, flight.velocity, flight.elapsed);
            set_camera(&follow);
            field();
            ball(flight.position);
        }
        _ => {
            set_camera(camera);
            field();
            pitcher(play.time);
            bat(play.time, &play.aim);
            if matches!(play.phase, Phase::Pitch) && play.time >= 0.0 {
                ball(play.pitch.position(play.time));
            }
        }
    }
    set_default_camera();
    match &play.phase {
        Phase::Pitch => reticle(camera, &play.aim),
        Phase::Result(card) => {
            if play.pitch.bounce_time.is_none() {
                let p = project(camera, on_plate(play.pitch.crossing));
                draw_circle(p.x, p.y, 4.5, GOLD);
            }
            if let Some(swing) = play.aim.swing {
                let p = project(camera, on_plate(swing.aim));
                draw_line(p.x - 7.0, p.y, p.x + 7.0, p.y, 2.0, WHITE);
                draw_line(p.x, p.y - 7.0, p.x, p.y + 7.0, 2.0, WHITE);
            }
            if let (Some(grade), Some(swing)) = (card.grade, play.aim.swing) {
                let target = project(camera, on_plate(swing.aim - grade.offset));
                let text = format!("{:+.0} ms", grade.timing * 1000.0);
                draw_text(&text, target.x + 15.0, target.y - 12.0, 20.0, WHITE);
            }
            if let Some(point) = card.landing {
                let text = format!("{:.0} m", point.xz().length());
                draw_text(
                    &text,
                    screen_width() * 0.5 - 15.0,
                    screen_height() * 0.82,
                    22.0,
                    WHITE,
                );
            }
        }
        Phase::Flight(_) => {}
    }
}
