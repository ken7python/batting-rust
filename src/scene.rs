use macroquad::camera::Camera;
use macroquad::prelude::*;

use crate::batting::*;
use crate::defense::{Defense, FIRST_BASE, FieldReturn, Fielder, SECOND_BASE, THIRD_BASE};
use crate::flight::GroundContact;
use crate::park::Park;
use crate::pitch::{Delivery, PitchForm};
use crate::runner::{Runner, SLIDE_SECONDS};

pub const MOUND: Vec3 = vec3(0.0, 0.0, -17.0);
const GRASS: Color = Color::from_rgba(49, 104, 74, 255);
const DIRT: Color = Color::from_rgba(174, 113, 78, 255);
const FENCE: Color = Color::from_rgba(23, 67, 59, 255);
const FOUL_LINE: Color = Color::from_rgba(234, 224, 189, 255);
const GOLD: Color = Color::from_rgba(255, 201, 95, 255);
const INK: Color = Color::from_rgba(9, 26, 34, 255);
pub const AIM_GREEN: Color = Color::from_rgba(126, 245, 71, 255);

pub fn batter_camera() -> Camera3D {
    batting_view(screen_width(), screen_height())
}

fn batting_view(width: f32, height: f32) -> Camera3D {
    let mut camera = field_camera();
    camera.fovy = batter_fov(width, height);
    // Lower the eye line on short screens to keep the release and zone close vertically.
    camera.position.y -= 0.50 * ((620.0 - height) / 210.0).clamp(0.0, 1.0);
    camera.target.y = batter_target_height(&camera);
    camera
}

fn batter_fov(width: f32, height: f32) -> f32 {
    let small_screen = ((640.0 - width.min(height)) / 220.0).clamp(0.0, 1.0);
    let focal_height = (width * 1.75).clamp(400.0, 740.0) * (1.0 + 0.40 * small_screen);
    let half_fov = 23.5_f32.to_radians();
    (2.0 * (half_fov.tan() * height / focal_height).atan())
        .clamp(15_f32.to_radians(), 68_f32.to_radians())
}

fn batter_target_height(camera: &Camera3D) -> f32 {
    // Anchor the zone near 58% of the screen height, even when the phone zooms in.
    let down_to_zone = ((camera.position.y - CENTER.y) / camera.position.z).atan();
    let down_to_target = down_to_zone - ((camera.fovy * 0.5).tan() * 0.16).atan();
    camera.position.y - down_to_target.tan() * (camera.position.z - camera.target.z)
}

pub fn marker_scale(width: f32, height: f32) -> f32 {
    (width.min(height * 1.7) / 760.0).clamp(1.0, 1.85)
}

fn field_camera() -> Camera3D {
    Camera3D {
        position: vec3(0.07, 1.68, 3.55),
        target: vec3(0.0, 0.43, -9.0),
        up: Vec3::Y,
        fovy: 47_f32.to_radians(),
        z_far: 250.0,
        ..Default::default()
    }
}

pub fn flight_camera(ball: Vec3, velocity: Vec3, elapsed: f32) -> Camera3D {
    let mut camera = batter_camera();
    let direction = vec3(velocity.x, 0.0, velocity.z).normalize_or_zero();
    let blend = (elapsed / 0.55).clamp(0.0, 1.0);
    let blend = blend * blend * (3.0 - 2.0 * blend);
    camera.fovy = camera.fovy + (field_camera().fovy - camera.fovy) * blend;
    camera.position = camera
        .position
        .lerp(ball - direction * 7.5 + vec3(3.2, 2.5, 0.0), blend);
    camera.target = camera
        .target
        .lerp(ball + direction * 5.0 - Vec3::Y * 0.75, blend);
    camera
}

pub fn return_camera(returning: &FieldReturn, runner: &Runner) -> Camera3D {
    let middle = (returning.pickup.xz() + returning.target) * 0.5;
    let span = returning.pickup.xz().distance(returning.target);
    let target = vec3(middle.x, 0.8, middle.y);
    let mut camera = Camera3D {
        position: target + vec3(span * 0.35 + 7.0, span * 0.40 + 5.0, span * 0.5 + 9.0),
        target,
        z_near: 0.65,
        ..field_camera()
    };
    if runner.sliding {
        let target = vec3(returning.target.x, 0.6, returning.target.y);
        let closeup = (1.0 - runner.remaining_time() / SLIDE_SECONDS) as f32 * 0.85 + 0.15;
        camera.position = camera.position.lerp(target + vec3(5.2, 4.4, 8.5), closeup);
        camera.target = camera.target.lerp(target, closeup);
    }
    camera
}

pub fn project(camera: &Camera3D, position: Vec3) -> Vec2 {
    let clip = camera.matrix() * position.extend(1.0);
    screen_point(clip.truncate() / clip.w)
}

pub fn visible_ball(camera: &Camera3D, position: Vec3) -> Option<Vec2> {
    let clip = camera.matrix() * position.extend(1.0);
    if clip.w <= 0.0 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    (ndc.abs().max_element() < 1.0).then(|| screen_point(ndc))
}

fn screen_point(ndc: Vec3) -> Vec2 {
    vec2(
        (ndc.x + 1.0) * screen_width() * 0.5,
        (1.0 - ndc.y) * screen_height() * 0.5,
    )
}

pub fn on_plate(point: Vec2) -> Vec3 {
    point.extend(0.0)
}

fn flat_quad(corners: [Vec3; 4], color: Color) {
    draw_mesh(&Mesh {
        vertices: corners
            .iter()
            .map(|p| Vertex::new(p.x, p.y, p.z, 0.0, 0.0, color))
            .collect(),
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: None,
    });
}

fn line_on_ground(a: Vec3, b: Vec3, width: f32, color: Color) {
    let normal = Vec3::Y.cross(b - a).normalize() * width * 0.5;
    flat_quad([a + normal, a - normal, b - normal, b + normal], color);
}

pub fn field(park: Park) {
    let (fence_radius, fence_height) = park.fence();
    let (sky, grass, stripes, wall, seats) = match park {
        Park::Central => (
            Color::from_rgba(147, 198, 218, 255),
            GRASS,
            Color::from_rgba(56, 116, 80, 255),
            FENCE,
            [
                Color::from_rgba(68, 100, 113, 255),
                Color::from_rgba(75, 109, 120, 255),
            ],
        ),
        Park::Sunset => (
            Color::from_rgba(228, 165, 128, 255),
            Color::from_rgba(55, 100, 73, 255),
            Color::from_rgba(65, 114, 79, 255),
            Color::from_rgba(44, 51, 82, 255),
            [
                Color::from_rgba(89, 74, 105, 255),
                Color::from_rgba(103, 85, 114, 255),
            ],
        ),
    };
    clear_background(sky);
    draw_plane(vec3(0.0, -0.04, -95.0), vec2(155.0, 152.0), None, grass);
    for i in 0..12 {
        draw_plane(
            vec3(0.0, -0.035, -7.0 - i as f32 * 8.0),
            vec2(100.0, 2.0),
            None,
            stripes,
        );
    }
    draw_cylinder(vec3(0.0, -0.03, 0.0), 2.3, 2.3, 0.04, None, DIRT);
    draw_cylinder(MOUND, 2.6, 2.4, 0.14, None, DIRT);
    draw_cube(
        MOUND + vec3(0.0, 0.11, -0.20),
        vec3(0.61, 0.06, 0.16),
        None,
        WHITE,
    );

    let plate = vec3(0.0, 0.032, 0.0);
    let first = vec3(FIRST_BASE.x, 0.032, FIRST_BASE.y);
    let third = vec3(THIRD_BASE.x, 0.032, THIRD_BASE.y);
    let second = vec3(SECOND_BASE.x, 0.032, SECOND_BASE.y);
    for (a, b) in [
        (plate, first),
        (plate, third),
        (first, second),
        (third, second),
    ] {
        line_on_ground(a, b, 1.2, DIRT);
    }
    for base in [first, second, third] {
        draw_cube(
            base + Vec3::Y * 0.04,
            vec3(0.45, 0.12, 0.45),
            None,
            FOUL_LINE,
        );
    }
    for side in [-1.0, 1.0] {
        let end = vec3(side, 0.0, -1.0).normalize() * fence_radius;
        line_on_ground(
            plate + Vec3::Y * 0.01,
            end + Vec3::Y * 0.042,
            0.05,
            FOUL_LINE,
        );
        draw_cylinder(end + Vec3::Y * fence_height, 0.08, 0.08, 11.0, None, GOLD);
    }
    flat_quad(
        [
            vec3(-0.215, 0.055, 0.0),
            vec3(0.215, 0.055, 0.0),
            vec3(0.215, 0.055, 0.22),
            vec3(-0.215, 0.055, 0.22),
        ],
        WHITE,
    );
    flat_quad(
        [
            vec3(-0.215, 0.055, 0.22),
            vec3(0.215, 0.055, 0.22),
            vec3(0.0, 0.055, 0.43),
            vec3(-0.215, 0.055, 0.22),
        ],
        WHITE,
    );

    for i in 0..32 {
        let edge = |j| {
            let angle = (j as f32 / 32.0 - 0.5) * std::f32::consts::FRAC_PI_2;
            vec3(angle.sin() * fence_radius, 0.0, -angle.cos() * fence_radius)
        };
        let (a, b) = (edge(i), edge(i + 1));
        let top = Vec3::Y * fence_height;
        let band = top - Vec3::Y * 0.17;
        flat_quad([a, b, b + band, a + band], wall);
        flat_quad([a + band, b + band, b + top, a + top], GOLD);
        let back = |p: Vec3| p * 1.07 - Vec3::Y;
        flat_quad(
            [
                back(a),
                back(b),
                back(b) + Vec3::Y * 8.0,
                back(a) + Vec3::Y * 8.0,
            ],
            seats[i % 2],
        );
    }
}

pub fn batter_runner(runner: &Runner, time: f32, active: bool) {
    draw_runner(runner, time, active, false);
}

pub fn confident_runner(runner: &Runner, time: f32) {
    draw_runner(runner, time, true, true);
}

fn draw_runner(runner: &Runner, time: f32, active: bool, walking: bool) {
    let point = runner.position();
    let feet = vec3(point.x, 0.0, point.y);
    let facing = runner.direction();
    let forward = vec3(facing.x, 0.0, facing.y);
    let side = Vec3::Y.cross(forward);
    let stride = if active && runner.moving() {
        (time * if walking { 7.0 } else { 24.0 }).sin() * if walking { 0.10 } else { 0.28 }
    } else {
        0.0
    };
    let jersey = Color::from_rgba(232, 116, 67, 255);
    if runner.sliding {
        let progress = (1.0 - runner.remaining_time() / SLIDE_SECONDS) as f32;
        let hips = feet - forward * 0.61 + Vec3::Y * (0.21 + 0.37 * (1.0 - progress));
        let shoulders = hips - forward * 0.51 + Vec3::Y * (0.14 + 0.32 * (1.0 - progress));
        draw_sphere(
            shoulders - forward * 0.16 + Vec3::Y * 0.13,
            0.16,
            None,
            jersey,
        );
        for segment in 0..=4 {
            let t = segment as f32 / 4.0;
            draw_cube(
                hips.lerp(shoulders, t),
                vec3(0.34, 0.23, 0.34),
                None,
                jersey,
            );
            for lateral in [-0.12, 0.12] {
                let toe = feet + Vec3::Y * 0.11 + side * lateral;
                draw_cube(
                    hips.lerp(toe, t) + side * (lateral * (1.0 - t)),
                    vec3(0.18, 0.17, 0.18),
                    None,
                    WHITE,
                );
            }
        }
        return;
    }
    draw_cube(feet + Vec3::Y * 1.09, vec3(0.47, 0.60, 0.38), None, jersey);
    draw_sphere(
        feet + Vec3::Y * 1.59,
        0.15,
        None,
        Color::from_rgba(216, 165, 118, 255),
    );
    draw_cube(feet + Vec3::Y * 1.71, vec3(0.35, 0.10, 0.36), None, jersey);
    for step_side in [-1.0, 1.0] {
        let step = stride * step_side;
        draw_cube(
            feet + side * (0.14 * step_side) + forward * step + Vec3::Y * (0.46 + step.max(0.0)),
            vec3(0.18, 0.88, 0.20),
            None,
            WHITE,
        );
    }
}

pub fn fielders(
    defense: &Defense,
    time: f32,
    returning: Option<&FieldReturn>,
    catch: Option<(Fielder, Vec3)>,
    result_age: f32,
) {
    let jersey = Color::from_rgba(52, 113, 201, 255);
    let skin = Color::from_rgba(216, 165, 118, 255);
    for (role, point) in Fielder::ALL.into_iter().zip(defense.positions) {
        let catching = catch
            .filter(|(fielder, _)| *fielder == role)
            .map(|(_, ball)| ball);
        if let Some(ball) = catching.filter(|_| defense.diving == Some(role)) {
            diving_fielder(point, ball, result_age, jersey, skin);
            continue;
        }
        let lift = catching.map_or(0.0, |ball| (ball.y - 1.79).clamp(0.0, 0.56));
        let feet = vec3(point.x, lift, point.y);
        let chasing = defense.chasing == Some(role);
        let carrier = returning.is_some_and(|play| play.fielder == role);
        let receiving = returning.is_some_and(|play| play.receiver == role && !play.carrying());
        let running = returning.map_or(chasing, |play| role == play.receiver);
        let step = if running && catching.is_none() {
            (time * 23.0).sin() * 0.18
        } else {
            0.0
        };
        draw_cube(
            feet + vec3(0.0, 1.12, 0.0),
            vec3(0.48, 0.59, 0.27),
            None,
            jersey,
        );
        draw_sphere(feet + vec3(0.0, 1.62, 0.02), 0.15, None, skin);
        draw_cube(
            feet + vec3(0.0, 1.73, 0.04),
            vec3(0.34, 0.10, 0.32),
            None,
            jersey,
        );
        for side in [-1.0, 1.0] {
            draw_cube(
                feet + vec3(side * 0.14, 0.48 + (step * side).max(0.0), step * side),
                vec3(0.19, 0.9, 0.21),
                None,
                WHITE,
            );
        }
        let glove_height = if carrier || receiving {
            1.55
        } else if chasing {
            1.28
        } else {
            0.83
        };
        let mut glove = feet + vec3(0.35, glove_height, 0.08);
        if let Some(play) = returning.filter(|play| carrier && !play.carrying()) {
            let toward = (play.target - point).normalize_or_zero();
            let progress = play.transfer_progress();
            let reach = -0.28 + progress * 0.58;
            glove = feet + vec3(toward.x * reach, 0.65 + progress, toward.y * reach);
        }
        if let Some(ball) = catching {
            let shoulder = feet + vec3(0.20, 1.35, 0.0);
            glove = ball;
            for joint in 1..=5 {
                draw_cube(
                    shoulder.lerp(glove, joint as f32 / 6.0),
                    vec3(0.16, 0.16, 0.16),
                    None,
                    if joint <= 2 { jersey } else { skin },
                );
            }
        }
        draw_cube(
            glove,
            vec3(0.25, 0.27, 0.21),
            None,
            if catching.is_some() {
                BROWN
            } else if chasing || receiving {
                GOLD
            } else {
                BROWN
            },
        );
    }
}

fn diving_fielder(point: Vec2, ball: Vec3, age: f32, jersey: Color, skin: Color) {
    let forward = (ball.xz() - point).normalize_or_zero();
    let forward = vec3(forward.x, 0.0, forward.y);
    let side = Vec3::Y.cross(forward);
    let settle = (age / 0.32).clamp(0.0, 1.0);
    let hips =
        vec3(point.x, 0.24 + 0.15 * (1.0 - settle), point.y) + forward * (0.18 + 0.36 * settle);
    let shoulders = hips + forward * 0.56 + Vec3::Y * 0.09;
    draw_sphere(
        shoulders + forward * 0.19 + Vec3::Y * 0.07,
        0.15,
        None,
        skin,
    );
    draw_cube(
        shoulders + forward * 0.21 + Vec3::Y * 0.16,
        vec3(0.28, 0.09, 0.28),
        None,
        jersey,
    );
    for segment in 0..=6 {
        let t = segment as f32 / 6.0;
        draw_cube(
            hips.lerp(shoulders, t),
            vec3(0.43, 0.24, 0.43),
            None,
            jersey,
        );
        for lateral in [-0.14, 0.14] {
            let foot = hips - forward * 0.78 + side * lateral - Vec3::Y * 0.12;
            draw_cube(
                (hips + side * lateral).lerp(foot, t),
                vec3(0.17, 0.18, 0.17),
                None,
                WHITE,
            );
        }
        let arm = (shoulders + side * 0.14).lerp(ball, t);
        draw_cube(
            arm,
            Vec3::splat(0.16),
            None,
            if segment < 3 { jersey } else { skin },
        );
    }
    draw_cube(ball, vec3(0.28, 0.22, 0.26), None, BROWN);
}

pub fn pitcher(time: f64, delivery: Delivery) {
    let jersey = Color::from_rgba(245, 239, 220, 255);
    let red = Color::from_rgba(171, 57, 46, 255);
    let skin = Color::from_rgba(212, 151, 105, 255);
    let wind = ((time + WINDUP) / WINDUP).clamp(0.0, 1.0) as f32;
    let follow = (time / 0.31).clamp(0.0, 1.0) as f32;
    let mirror = delivery.hand.sign();
    let release = delivery.release();
    let body = MOUND + vec3(0.0, 1.14, 0.12 + 0.2 * wind);
    draw_cube(body, vec3(0.44, 0.55, 0.24), None, jersey);
    draw_sphere(body + vec3(0.0, 0.43, 0.01), 0.15, None, skin);
    draw_cube(
        body + vec3(0.0, 0.54, 0.05),
        vec3(0.32, 0.10, 0.33),
        None,
        red,
    );
    for side in [-1.0, 1.0] {
        let lift = if side * mirror < 0.0 {
            (wind * std::f32::consts::PI).sin() * 0.30
        } else {
            0.0
        };
        draw_cube(
            body + vec3(side * 0.15, -0.62 + lift, 0.04),
            vec3(0.16, 0.75, 0.18),
            None,
            jersey,
        );
        draw_cube(
            body + vec3(side * 0.15, -0.98 + lift, 0.11),
            vec3(0.21, 0.09, 0.29),
            None,
            red,
        );
    }
    let (reach, lift, drop) = match delivery.form {
        PitchForm::Overhand => (0.25, 0.55, -0.19),
        PitchForm::Sidearm => (0.66, 0.13, 0.11),
    };
    let hand = if time <= 0.0 {
        (body
            + vec3(
                reach * mirror,
                0.20 + (wind * std::f32::consts::PI).sin() * lift,
                0.10,
            ))
        .lerp(release, wind.powi(4))
    } else {
        release.lerp(body + vec3(-0.25 * mirror, drop, 0.34), follow)
    };
    for i in 0..6 {
        draw_sphere(
            (body + vec3(0.21 * mirror, 0.17, 0.04)).lerp(hand, i as f32 / 6.0),
            0.081,
            None,
            if i < 3 { red } else { skin },
        );
    }
    if time < 0.0 {
        ball(hand);
    }
}

pub fn ball(p: Vec3) {
    draw_sphere(p, BALL_RADIUS * 1.25, None, WHITE);
    draw_sphere(p + vec3(0.008, 0.0, 0.044), 0.008, None, MAROON);
}

fn ground_ring(point: Vec3, inner: f32, outer: f32, color: Color) {
    let segments = 16;
    let mut vertices = Vec::with_capacity(2 * (segments + 1));
    let mut indices = Vec::with_capacity(6 * segments);
    for i in 0..=segments {
        let angle = i as f32 * std::f32::consts::TAU / segments as f32;
        let direction = vec3(angle.sin(), 0.0, angle.cos());
        for radius in [inner, outer] {
            let p = point + direction * radius;
            vertices.push(Vertex::new(p.x, p.y, p.z, 0.0, 0.0, color));
        }
        if i > 0 {
            let next = (2 * i) as u16;
            indices.extend([next - 2, next - 1, next, next - 1, next + 1, next]);
        }
    }
    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: None,
    });
}

pub fn batted_ball_shadow(p: Vec3) {
    let height = p.y.clamp(0.0, 12.0);
    ground_ring(
        vec3(p.x, 0.017, p.z),
        0.0,
        0.18 + 0.025 * height,
        Color::new(0.07, 0.10, 0.07, 0.62 - 0.025 * height),
    );
}

pub fn ground_impact(contact: GroundContact, elapsed: f32) {
    let age = (elapsed - contact.at).max(0.0);
    let life = age / 0.65;
    if life >= 1.0 {
        return;
    }
    let point = vec3(contact.position.x, 0.060, contact.position.z);
    let radius = 0.24 + life * 1.25;
    ground_ring(
        point,
        radius - 0.09,
        radius,
        Color::new(1.0, 0.86, 0.57, 0.94 * (1.0 - life)),
    );
    for i in 0..8 {
        let angle = i as f32 * std::f32::consts::FRAC_PI_4;
        let direction = vec3(angle.sin(), 0.0, angle.cos());
        let height = life * (1.0 - life) * (1.1 + 0.3 * (i % 3) as f32);
        draw_sphere(
            point + direction * radius * 0.78 + Vec3::Y * height,
            0.07 * (1.0 - life),
            None,
            Color::new(0.96, 0.82, 0.58, 1.0 - life),
        );
    }
}

pub fn bat(time: f64, aim: &Aim) {
    let pivot = vec3(-0.77 + aim.point.x, aim.point.y - 0.15, 0.22);
    let elapsed = aim.last_swing.map_or(0.0, |s| (time - s.time).max(0.0));
    let phase = if elapsed > 0.22 && aim.live_swing().is_none() {
        (1.0 - (elapsed - 0.22) / 0.13).clamp(0.0, 1.0) as f32
    } else {
        (elapsed / 0.22).clamp(0.0, 1.0) as f32
    };
    let theta = -1.38 + phase * 3.70;
    let direction = vec3(theta.cos(), 0.30 * (1.0 - phase * 1.5), -theta.sin());
    if elapsed < 0.22 && phase > 0.03 && phase < 0.89 {
        for i in 0..5 {
            let angle = theta - i as f32 * 0.10;
            let end = pivot + vec3(angle.cos(), direction.y, -angle.sin()) * 0.88;
            draw_line_3d(
                end,
                end.lerp(pivot, 0.32),
                Color::new(1.0, 0.83, 0.47, 0.24 - i as f32 * 0.04),
            );
        }
    }
    for i in 0..17 {
        let u = i as f32 / 16.0;
        draw_sphere(
            pivot + direction * u * 0.90,
            0.031 + 0.028 * u,
            None,
            if i < 4 {
                INK
            } else {
                Color::from_rgba(218, 161, 87, 255)
            },
        );
    }
}

pub fn reticle(camera: &Camera3D, aim: &Aim) {
    let a = project(camera, on_plate(ZONE_MIN));
    let b = project(camera, on_plate(ZONE_MAX));
    // Solid, pixel-aligned strips stay visible on high-DPI Metal displays.
    let left = a.x.min(b.x).floor();
    let top = a.y.min(b.y).floor();
    let projected_width = (b.x - a.x).abs();
    let width = projected_width.ceil();
    let height = (a.y - b.y).abs().ceil();
    let thickness = projected_width * 0.028;
    let color = Color::new(0.98, 0.98, 0.83, 0.85);
    draw_rectangle(left, top, width, thickness, color);
    draw_rectangle(left, top + height - thickness, width, thickness, color);
    draw_rectangle(left, top, thickness, height, color);
    draw_rectangle(left + width - thickness, top, thickness, height, color);
    let p = project(camera, on_plate(aim.point));
    let radius = projected_width * 0.15;
    let tint = if aim.held {
        Color::from_rgba(181, 255, 102, 255)
    } else {
        AIM_GREEN
    };
    if aim.held {
        draw_circle(p.x, p.y, radius * 1.2, Color::new(0.55, 1.0, 0.28, 0.20));
    }
    draw_circle_lines(p.x, p.y, radius, radius * 0.40, INK);
    draw_circle_lines(p.x, p.y, radius, radius * 0.27, tint);
    draw_circle(p.x, p.y, radius * 0.14, tint);
}

pub fn aim_movement(camera: &Camera3D, pixels: Vec2) -> Vec2 {
    let a = project(camera, on_plate(ZONE_MIN));
    let b = project(camera, on_plate(ZONE_MAX));
    pixels * (ZONE_MAX - ZONE_MIN) / (b - a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batting_framing_shows_both_zone_corners_and_release_on_small_screens() {
        for (width, height) in [
            (390.0, 844.0),
            (844.0, 390.0),
            (568.0, 320.0),
            (320.0, 568.0),
            (1120.0, 720.0),
        ] {
            let camera = batting_view(width, height);
            let view = Mat4::perspective_rh_gl(camera.fovy, width / height, 0.01, camera.z_far)
                * Mat4::look_at_rh(camera.position, camera.target, camera.up);
            for point in [
                on_plate(ZONE_MIN),
                on_plate(ZONE_MAX),
                Delivery::default().release(),
            ] {
                let clip = view * point.extend(1.0);
                assert!(clip.w > 0.0);
                assert!((clip.x / clip.w).abs() < 0.9);
                assert!((clip.y / clip.w).abs() < 0.9, "{width}x{height} {point:?}");
            }
        }
    }

    #[test]
    fn smaller_screens_get_a_longer_batting_camera_focal_length() {
        let magnification = |width, height| height / (batter_fov(width, height) * 0.5).tan();
        let desktop = magnification(1120.0, 720.0);
        assert!(magnification(390.0, 844.0) > desktop * 1.25);
        assert!(magnification(844.0, 390.0) > desktop * 1.35);
        assert!(magnification(320.0, 568.0) > desktop);
    }
}
