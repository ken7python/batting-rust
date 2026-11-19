use macroquad::camera::Camera;
use macroquad::prelude::*;

use crate::batting::*;
use crate::pitch::ThrowingHand;

pub const MOUND: Vec3 = vec3(0.0, 0.0, -17.0);
pub const FENCE_RADIUS: f32 = 90.0;
pub const FENCE_HEIGHT: f32 = 3.0;
const GRASS: Color = Color::from_rgba(49, 104, 74, 255);
const DIRT: Color = Color::from_rgba(174, 113, 78, 255);
const FENCE: Color = Color::from_rgba(23, 67, 59, 255);
const FOUL_LINE: Color = Color::from_rgba(234, 224, 189, 255);
const GOLD: Color = Color::from_rgba(255, 201, 95, 255);
const INK: Color = Color::from_rgba(9, 26, 34, 255);

pub fn batter_camera() -> Camera3D {
    Camera3D {
        position: vec3(0.07, 1.68, 3.15),
        target: vec3(0.0, 1.35, -9.0),
        up: Vec3::Y,
        fovy: 47_f32.to_radians(),
        z_far: 250.0,
        ..Default::default()
    }
}

pub fn flight_camera(ball: Vec3, velocity: Vec3, elapsed: f32) -> Camera3D {
    let mut camera = batter_camera();
    let direction = vec3(velocity.x, 0.0, velocity.z).normalize_or_zero();
    let blend = (elapsed / 0.3).clamp(0.0, 1.0);
    camera.position = camera
        .position
        .lerp(ball - direction * 7.5 + vec3(3.2, 2.5, 0.0), blend);
    camera.target = camera
        .target
        .lerp(ball + direction * 5.0 - Vec3::Y * 0.75, blend);
    camera
}

pub fn project(camera: &Camera3D, position: Vec3) -> Vec2 {
    let clip = camera.matrix() * position.extend(1.0);
    let ndc = clip.truncate() / clip.w;
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

pub fn field() {
    draw_plane(vec3(0.0, -0.04, -35.0), vec2(120.0, 110.0), None, GRASS);
    for i in 0..12 {
        draw_plane(
            vec3(0.0, -0.035, -7.0 - i as f32 * 8.0),
            vec2(100.0, 2.0),
            None,
            Color::from_rgba(56, 116, 80, 255),
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
    let first = vec3(19.4, 0.032, -19.4);
    let third = vec3(-19.4, 0.032, -19.4);
    let second = vec3(0.0, 0.032, -38.8);
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
        let end = vec3(side, 0.0, -1.0).normalize() * FENCE_RADIUS;
        line_on_ground(
            plate + Vec3::Y * 0.01,
            end + Vec3::Y * 0.042,
            0.05,
            FOUL_LINE,
        );
        draw_cylinder(end + Vec3::Y * FENCE_HEIGHT, 0.08, 0.08, 11.0, None, GOLD);
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
            vec3(angle.sin() * FENCE_RADIUS, 0.0, -angle.cos() * FENCE_RADIUS)
        };
        let (a, b) = (edge(i), edge(i + 1));
        let top = Vec3::Y * FENCE_HEIGHT;
        flat_quad([a, b, b + top, a + top], FENCE);
        flat_quad(
            [
                a + top,
                b + top,
                b + top + Vec3::Y * 0.17,
                a + top + Vec3::Y * 0.17,
            ],
            GOLD,
        );
        let back = |p: Vec3| p * 1.07 - Vec3::Y;
        flat_quad(
            [
                back(a),
                back(b),
                back(b) + Vec3::Y * 8.0,
                back(a) + Vec3::Y * 8.0,
            ],
            if i % 2 == 0 {
                Color::from_rgba(68, 100, 113, 255)
            } else {
                Color::from_rgba(75, 109, 120, 255)
            },
        );
    }
}

pub fn pitcher(time: f64, throwing_hand: ThrowingHand) {
    let jersey = Color::from_rgba(245, 239, 220, 255);
    let red = Color::from_rgba(171, 57, 46, 255);
    let skin = Color::from_rgba(212, 151, 105, 255);
    let wind = ((time + WINDUP) / WINDUP).clamp(0.0, 1.0) as f32;
    let follow = (time / 0.31).clamp(0.0, 1.0) as f32;
    let mirror = throwing_hand.sign();
    let release = throwing_hand.release();
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
    let hand = if time <= 0.0 {
        (body
            + vec3(
                0.25 * mirror,
                0.20 + (wind * std::f32::consts::PI).sin() * 0.55,
                0.10,
            ))
        .lerp(release, wind.powi(4))
    } else {
        release.lerp(body + vec3(-0.25 * mirror, -0.19, 0.34), follow)
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

pub fn bat(time: f64, aim: &Aim) {
    let pivot = vec3(-0.77 + aim.point.x, aim.point.y - 0.15, 0.22);
    let phase = aim
        .swing
        .map_or(0.0, |s| ((time - s.time) / 0.22).clamp(0.0, 1.0) as f32);
    let theta = -1.38 + phase * 3.70;
    let direction = vec3(theta.cos(), 0.30 * (1.0 - phase * 1.5), -theta.sin());
    if phase > 0.03 && phase < 0.89 {
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
    draw_rectangle_lines(
        a.x,
        b.y,
        b.x - a.x,
        a.y - b.y,
        1.5,
        Color::new(0.98, 0.98, 0.83, 0.55),
    );
    let p = project(camera, on_plate(aim.point));
    let w = project(camera, on_plate(aim.point + vec2(MEET_RADIUS.x, 0.0))).x - p.x;
    let h = p.y - project(camera, on_plate(aim.point + vec2(0.0, MEET_RADIUS.y))).y;
    let tint = if aim.held {
        GOLD
    } else {
        Color::new(0.93, 0.99, 0.92, 0.77)
    };
    draw_ellipse_lines(p.x, p.y, w, h, 0.0, 1.8, tint);
    draw_circle(p.x, p.y, 2.2, tint);
}

pub fn aim_movement(camera: &Camera3D, pixels: Vec2) -> Vec2 {
    let a = project(camera, on_plate(ZONE_MIN));
    let b = project(camera, on_plate(ZONE_MAX));
    pixels * (ZONE_MAX - ZONE_MIN) / (b - a)
}
