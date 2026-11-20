use macroquad::prelude::*;

use crate::park::Park;

const SECTIONS: usize = 32;
const ROWS: usize = 6;
const ROW_DEPTH: f32 = 1.05;
const ROW_RISE: f32 = 0.88;
const SEATS_PER_ROW: usize = 7;

const SHIRTS: [Color; 9] = [
    Color::from_rgba(225, 224, 199, 255),
    Color::from_rgba(96, 160, 214, 255),
    Color::from_rgba(199, 103, 77, 255),
    Color::from_rgba(231, 172, 78, 255),
    Color::from_rgba(83, 134, 114, 255),
    Color::from_rgba(113, 114, 172, 255),
    Color::from_rgba(157, 189, 188, 255),
    Color::from_rgba(175, 118, 147, 255),
    Color::from_rgba(59, 81, 109, 255),
];
const SKIN: [Color; 4] = [
    Color::from_rgba(235, 192, 147, 255),
    Color::from_rgba(202, 151, 103, 255),
    Color::from_rgba(161, 108, 75, 255),
    Color::from_rgba(117, 77, 61, 255),
];

pub struct Stands {
    central: Vec<Mesh>,
    sunset: Vec<Mesh>,
}

impl Stands {
    pub fn new() -> Self {
        Self {
            central: build_stands(Park::Central),
            sunset: build_stands(Park::Sunset),
        }
    }

    pub fn draw(&self, park: Park) {
        let sections = match park {
            Park::Central => &self.central,
            Park::Sunset => &self.sunset,
        };
        for section in sections {
            draw_mesh(section);
        }
    }
}

fn on_arc(radius: f32, angle: f32, y: f32) -> Vec3 {
    vec3(radius * angle.sin(), y, -radius * angle.cos())
}

fn shaded(color: Color, factor: f32) -> Color {
    Color::new(
        color.r * factor,
        color.g * factor,
        color.b * factor,
        color.a,
    )
}

fn build_stands(park: Park) -> Vec<Mesh> {
    let (radius, fence_height) = park.fence();
    let (concrete, seat) = match park {
        Park::Central => (
            Color::from_rgba(92, 120, 127, 255),
            Color::from_rgba(46, 83, 101, 255),
        ),
        Park::Sunset => (
            Color::from_rgba(117, 95, 112, 255),
            Color::from_rgba(69, 58, 92, 255),
        ),
    };
    let rng = rand::RandGenerator::new();
    // Give scenery a separate seed so its generation never affects the pitch sequence.
    rng.srand(match park {
        Park::Central => 17,
        Park::Sunset => 97,
    });
    let arc = std::f32::consts::FRAC_PI_2;
    let section_angle = arc / SECTIONS as f32;
    let mut meshes = Vec::with_capacity(SECTIONS);
    for section in 0..SECTIONS {
        let start = section as f32 * section_angle - arc * 0.5;
        let end = start + section_angle;
        let mut mesh = Mesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            texture: None,
        };
        for row in 0..ROWS {
            let front = radius + 4.2 + row as f32 * ROW_DEPTH;
            let back = front + ROW_DEPTH;
            let floor = fence_height - 0.55 + row as f32 * ROW_RISE;
            let bottom = if row == 0 { 0.0 } else { floor - ROW_RISE };
            let edge = |r, a, y| on_arc(r, a, y);
            quad(
                &mut mesh,
                [
                    edge(front, start, bottom),
                    edge(front, end, bottom),
                    edge(front, end, floor),
                    edge(front, start, floor),
                ],
                shaded(concrete, 0.78),
            );
            quad(
                &mut mesh,
                [
                    edge(front, start, floor),
                    edge(front, end, floor),
                    edge(back, end, floor),
                    edge(back, start, floor),
                ],
                concrete,
            );

            for column in 0..SEATS_PER_ROW {
                // Leave a stair aisle between groups of four sections.
                if section % 4 == 0 && column == 0 {
                    continue;
                }
                let fraction = (column as f32 + 0.5) / SEATS_PER_ROW as f32;
                let angle = start + fraction * section_angle;
                let foot = on_arc(front + 0.64, angle, floor);
                let across = vec3(angle.cos(), 0.0, angle.sin());
                silhouette(&mut mesh, foot + Vec3::Y * 0.26, across, 0.47, 0.52, seat);
                if rng.gen_range(0, 100) >= 78 {
                    continue;
                }
                let shirt = SHIRTS[rng.gen_range(0, SHIRTS.len())];
                let skin = SKIN[rng.gen_range(0, SKIN.len())];
                let size = rng.gen_range(0.87, 1.12);
                let inward = across.cross(Vec3::Y);
                let body = foot + inward * 0.12 + Vec3::Y * (0.32 + size * 0.30);
                block(
                    &mut mesh,
                    body,
                    across,
                    vec3(0.40, 0.56, 0.25) * size,
                    shirt,
                );
                let head = body + Vec3::Y * (0.39 * size);
                block(&mut mesh, head, across, vec3(0.24, 0.26, 0.22) * size, skin);
                if rng.gen_range(0, 5) == 0 {
                    silhouette(
                        &mut mesh,
                        head + inward * 0.13 + Vec3::Y * (0.10 * size),
                        across,
                        0.29 * size,
                        0.07 * size,
                        shaded(shirt, 0.72),
                    );
                }
            }
        }

        let back = radius + 4.2 + ROWS as f32 * ROW_DEPTH;
        let top = fence_height - 0.55 + (ROWS - 1) as f32 * ROW_RISE;
        quad(
            &mut mesh,
            [
                on_arc(back, start, 0.0),
                on_arc(back, end, 0.0),
                on_arc(back, end, top + 0.87),
                on_arc(back, start, top + 0.87),
            ],
            shaded(seat, 0.7),
        );
        meshes.push(mesh);
    }
    meshes
}

fn silhouette(mesh: &mut Mesh, center: Vec3, across: Vec3, width: f32, height: f32, color: Color) {
    let right = across * (width * 0.5);
    let up = Vec3::Y * (height * 0.5);
    quad(
        mesh,
        [
            center - right - up,
            center + right - up,
            center + right + up,
            center - right + up,
        ],
        color,
    );
}

fn block(mesh: &mut Mesh, center: Vec3, across: Vec3, size: Vec3, color: Color) {
    let right = across * (size.x * 0.5);
    let up = Vec3::Y * (size.y * 0.5);
    let depth = Vec3::Y.cross(across) * (size.z * 0.5);
    let front = center - depth;
    let back = center + depth;
    let fl = front - right + up;
    let fr = front + right + up;
    let bl = back - right + up;
    let br = back + right + up;
    let down = up * 2.0;
    quad(mesh, [fl - down, fr - down, fr, fl], color);
    quad(mesh, [fr - down, br - down, br, fr], shaded(color, 0.78));
    quad(mesh, [bl - down, fl - down, fl, bl], shaded(color, 0.88));
    quad(mesh, [fl, fr, br, bl], color);
}

fn quad(mesh: &mut Mesh, points: [Vec3; 4], color: Color) {
    let first = mesh.vertices.len() as u16;
    mesh.vertices
        .extend(points.map(|point| Vertex::new2(point, Vec2::ZERO, color)));
    mesh.indices
        .extend([first, first + 1, first + 2, first, first + 2, first + 3]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn occupied_seats_fit_the_renderer_budget_and_stay_behind_the_fence() {
        for park in [Park::Central, Park::Sunset] {
            let (radius, _) = park.fence();
            let stands = build_stands(park);
            assert_eq!(stands.len(), SECTIONS);
            for section in stands {
                assert!(section.vertices.len() < 10_000 && section.indices.len() < 5_000);
                assert!(
                    section
                        .indices
                        .iter()
                        .all(|&i| usize::from(i) < section.vertices.len())
                );
                assert!(
                    section
                        .vertices
                        .iter()
                        .all(|v| v.position.xz().length() > radius)
                );
                assert!(section.vertices.iter().any(|v| {
                    SHIRTS
                        .iter()
                        .any(|&shirt| v.color == <[u8; 4]>::from(shirt))
                }));
            }
        }
    }
}
