mod batting;
mod scene;

use batting::*;
use macroquad::prelude::*;
use scene::*;

const FLIGHT: f64 = 0.48;
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
    let mut time = -WINDUP;
    let mut aim = Aim::default();
    let mut previous_mouse = Vec2::from(mouse_position());
    let mut result = None;

    loop {
        time += f64::from(get_frame_time().min(0.05));
        let camera = batter_camera();
        let mouse = Vec2::from(mouse_position());
        if result.is_none() && time < FLIGHT + CATCH_DELAY {
            if is_mouse_button_pressed(MouseButton::Left) {
                aim.press();
            } else {
                aim.drag(aim_movement(&camera, mouse - previous_mouse));
            }
            if is_mouse_button_released(MouseButton::Left) {
                aim.release(time);
            }
        }
        previous_mouse = mouse;
        if result.is_none() && time >= FLIGHT + CATCH_DELAY {
            aim.held = false;
            result = Some(match aim.swing.map(|s| grade(s, FLIGHT, CENTER)) {
                Some(g) => match g.contact {
                    Contact::Barrel => "BARREL",
                    Contact::Touch => "CONTACT",
                    Contact::Miss if g.timing < -0.065 => "EARLY",
                    Contact::Miss if g.timing > 0.065 => "LATE",
                    Contact::Miss
                        if g.offset.y / MEET_RADIUS.y > (g.offset.x / MEET_RADIUS.x).abs() =>
                    {
                        "AIM LOWER"
                    }
                    Contact::Miss
                        if -g.offset.y / MEET_RADIUS.y > (g.offset.x / MEET_RADIUS.x).abs() =>
                    {
                        "AIM HIGHER"
                    }
                    Contact::Miss if g.offset.x > 0.0 => "AIM LEFT",
                    Contact::Miss => "AIM RIGHT",
                },
                None if in_zone(CENTER) => "CALLED STRIKE",
                None => "BALL",
            });
        }
        if time > FLIGHT + CATCH_DELAY + 1.0 {
            result = None;
            aim.next_pitch();
            time = -WINDUP;
        }

        clear_background(SKY);
        set_camera(&camera);
        field();
        pitcher(time);
        bat(time, &aim);
        if (0.0..FLIGHT + CATCH_DELAY).contains(&time) {
            ball(RELEASE.lerp(on_plate(CENTER), (time / FLIGHT) as f32));
        }
        set_default_camera();
        reticle(&camera, &aim);
        draw_text(
            "BATTING CLUB  /  FOUR-SEAM DRILL",
            28.0,
            48.0,
            25.0,
            DARKBLUE,
        );
        draw_text(
            "Hold + drag to aim. Release to swing.",
            28.0,
            screen_height() - 26.0,
            22.0,
            WHITE,
        );
        if let Some(label) = result {
            let width = measure_text(label, None, 36, 1.0).width;
            draw_text(
                label,
                (screen_width() - width) * 0.5,
                screen_height() * 0.75,
                36.0,
                WHITE,
            );
        }
        next_frame().await;
    }
}
