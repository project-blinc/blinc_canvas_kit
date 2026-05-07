//! Primitive geometry generators for 3D scenes.
//!
//! Each generator produces a `(Vec<Vertex>, Vec<u32>)` pair ready to
//! feed into `MeshData`. All geometry is centered at the origin with
//! correct normals and UV mapping.

use blinc_core::draw::Vertex;
use std::f32::consts::PI;

/// (normal, tangent_unused, [4 vertex positions]) per face.
type BoxFace = ([f32; 3], [f32; 3], [[f32; 3]; 4]);

/// Primitive geometry generator. Call a static method, get back
/// vertices + indices ready for `MeshData`.
pub struct Geometry;

impl Geometry {
    /// Axis-aligned box centered at origin.
    pub fn cube(size: f32) -> (Vec<Vertex>, Vec<u32>) {
        Self::box_(size, size, size)
    }

    /// Axis-aligned box with independent dimensions, centered at origin.
    pub fn box_(width: f32, height: f32, depth: f32) -> (Vec<Vertex>, Vec<u32>) {
        let (hw, hh, hd) = (width * 0.5, height * 0.5, depth * 0.5);

        // 6 faces × 4 vertices = 24 vertices (unshared for correct normals)
        #[rustfmt::skip]
        let faces: &[BoxFace] = &[
            // (normal, tangent_sign_unused, [positions])
            ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [[-hw,-hh, hd],[ hw,-hh, hd],[ hw, hh, hd],[-hw, hh, hd]]),  // +Z
            ([0.0, 0.0,-1.0], [-1.0,0.0, 0.0], [[ hw,-hh,-hd],[-hw,-hh,-hd],[-hw, hh,-hd],[ hw, hh,-hd]]), // -Z
            ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [[-hw, hh, hd],[ hw, hh, hd],[ hw, hh,-hd],[-hw, hh,-hd]]),  // +Y
            ([0.0,-1.0, 0.0], [1.0, 0.0, 0.0], [[-hw,-hh,-hd],[ hw,-hh,-hd],[ hw,-hh, hd],[-hw,-hh, hd]]),  // -Y
            ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [[ hw,-hh, hd],[ hw,-hh,-hd],[ hw, hh,-hd],[ hw, hh, hd]]),  // +X
            ([-1.0,0.0, 0.0], [0.0, 0.0,-1.0], [[-hw,-hh,-hd],[-hw,-hh, hd],[-hw, hh, hd],[-hw, hh,-hd]]), // -X
        ];

        let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);

        for (normal, _, positions) in faces {
            let base = vertices.len() as u32;
            for (i, pos) in positions.iter().enumerate() {
                vertices.push(Vertex::new(*pos).with_normal(*normal).with_uv(uvs[i]));
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        (vertices, indices)
    }

    /// UV sphere centered at origin.
    ///
    /// `segments` controls both horizontal (longitude) and vertical
    /// (latitude) subdivisions. 16 is low-poly, 32 is smooth, 64 is
    /// high quality. Total triangles ≈ `2 × segments²`.
    pub fn sphere(radius: f32, segments: u32) -> (Vec<Vertex>, Vec<u32>) {
        let rings = segments;
        let sectors = segments;
        let mut vertices = Vec::with_capacity(((rings + 1) * (sectors + 1)) as usize);
        let mut indices = Vec::with_capacity((rings * sectors * 6) as usize);

        for r in 0..=rings {
            let v = r as f32 / rings as f32;
            let phi = v * PI;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            for s in 0..=sectors {
                let u = s as f32 / sectors as f32;
                let theta = u * 2.0 * PI;
                let sin_theta = theta.sin();
                let cos_theta = theta.cos();

                let nx = sin_phi * cos_theta;
                let ny = cos_phi;
                let nz = sin_phi * sin_theta;

                vertices.push(
                    Vertex::new([nx * radius, ny * radius, nz * radius])
                        .with_normal([nx, ny, nz])
                        .with_uv([u, v]),
                );
            }
        }

        for r in 0..rings {
            for s in 0..sectors {
                let a = r * (sectors + 1) + s;
                let b = a + sectors + 1;
                indices.extend_from_slice(&[a, b, a + 1, b, b + 1, a + 1]);
            }
        }

        (vertices, indices)
    }

    /// Flat plane on the XZ plane (Y = 0), centered at origin.
    pub fn plane(width: f32, depth: f32) -> (Vec<Vertex>, Vec<u32>) {
        let (hw, hd) = (width * 0.5, depth * 0.5);
        let vertices = vec![
            Vertex::new([-hw, 0.0, -hd])
                .with_normal([0.0, 1.0, 0.0])
                .with_uv([0.0, 0.0]),
            Vertex::new([hw, 0.0, -hd])
                .with_normal([0.0, 1.0, 0.0])
                .with_uv([1.0, 0.0]),
            Vertex::new([hw, 0.0, hd])
                .with_normal([0.0, 1.0, 0.0])
                .with_uv([1.0, 1.0]),
            Vertex::new([-hw, 0.0, hd])
                .with_normal([0.0, 1.0, 0.0])
                .with_uv([0.0, 1.0]),
        ];
        let indices = vec![0, 2, 1, 0, 3, 2];
        (vertices, indices)
    }

    /// Cylinder along the Y axis, centered at origin.
    pub fn cylinder(radius: f32, height: f32, segments: u32) -> (Vec<Vertex>, Vec<u32>) {
        let half_h = height * 0.5;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Side wall
        for i in 0..=segments {
            let u = i as f32 / segments as f32;
            let theta = u * 2.0 * PI;
            let (sin_t, cos_t) = (theta.sin(), theta.cos());
            let nx = cos_t;
            let nz = sin_t;

            // Bottom vertex
            vertices.push(
                Vertex::new([nx * radius, -half_h, nz * radius])
                    .with_normal([nx, 0.0, nz])
                    .with_uv([u, 1.0]),
            );
            // Top vertex
            vertices.push(
                Vertex::new([nx * radius, half_h, nz * radius])
                    .with_normal([nx, 0.0, nz])
                    .with_uv([u, 0.0]),
            );
        }

        for i in 0..segments {
            let base = i * 2;
            indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
        }

        // Top cap
        let top_center = vertices.len() as u32;
        vertices.push(
            Vertex::new([0.0, half_h, 0.0])
                .with_normal([0.0, 1.0, 0.0])
                .with_uv([0.5, 0.5]),
        );
        for i in 0..=segments {
            let theta = (i as f32 / segments as f32) * 2.0 * PI;
            let (sin_t, cos_t) = (theta.sin(), theta.cos());
            vertices.push(
                Vertex::new([cos_t * radius, half_h, sin_t * radius])
                    .with_normal([0.0, 1.0, 0.0])
                    .with_uv([cos_t * 0.5 + 0.5, sin_t * 0.5 + 0.5]),
            );
        }
        for i in 0..segments {
            indices.extend_from_slice(&[top_center, top_center + 1 + i, top_center + 2 + i]);
        }

        // Bottom cap
        let bot_center = vertices.len() as u32;
        vertices.push(
            Vertex::new([0.0, -half_h, 0.0])
                .with_normal([0.0, -1.0, 0.0])
                .with_uv([0.5, 0.5]),
        );
        for i in 0..=segments {
            let theta = (i as f32 / segments as f32) * 2.0 * PI;
            let (sin_t, cos_t) = (theta.sin(), theta.cos());
            vertices.push(
                Vertex::new([cos_t * radius, -half_h, sin_t * radius])
                    .with_normal([0.0, -1.0, 0.0])
                    .with_uv([cos_t * 0.5 + 0.5, sin_t * 0.5 + 0.5]),
            );
        }
        for i in 0..segments {
            indices.extend_from_slice(&[bot_center, bot_center + 2 + i, bot_center + 1 + i]);
        }

        (vertices, indices)
    }

    /// Torus (donut) centered at origin on the XZ plane.
    ///
    /// `major_radius` is the distance from the center to the tube center.
    /// `minor_radius` is the tube thickness.
    pub fn torus(
        major_radius: f32,
        minor_radius: f32,
        major_segments: u32,
        minor_segments: u32,
    ) -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for i in 0..=major_segments {
            let u = i as f32 / major_segments as f32;
            let theta = u * 2.0 * PI;
            let (sin_t, cos_t) = (theta.sin(), theta.cos());

            for j in 0..=minor_segments {
                let v = j as f32 / minor_segments as f32;
                let phi = v * 2.0 * PI;
                let (sin_p, cos_p) = (phi.sin(), phi.cos());

                let x = (major_radius + minor_radius * cos_p) * cos_t;
                let y = minor_radius * sin_p;
                let z = (major_radius + minor_radius * cos_p) * sin_t;

                let nx = cos_p * cos_t;
                let ny = sin_p;
                let nz = cos_p * sin_t;

                vertices.push(
                    Vertex::new([x, y, z])
                        .with_normal([nx, ny, nz])
                        .with_uv([u, v]),
                );
            }
        }

        for i in 0..major_segments {
            for j in 0..minor_segments {
                let a = i * (minor_segments + 1) + j;
                let b = a + minor_segments + 1;
                indices.extend_from_slice(&[a, b, a + 1, b, b + 1, a + 1]);
            }
        }

        (vertices, indices)
    }

    /// Flat grid on the XZ plane (Y = 0) centered at origin.
    ///
    /// Generates thin quad strips for each grid line — both major and
    /// minor subdivisions. Lines have vertex color baked in (gray for
    /// minor, brighter for major, red for X axis, blue for Z axis).
    ///
    /// `size` is the total extent (grid goes from -size/2 to +size/2).
    /// `divisions` is the number of cells along each axis.
    /// `subdivisions` is the number of minor lines per cell.
    /// `line_width` is the thickness of each line quad.
    pub fn grid(
        size: f32,
        divisions: u32,
        subdivisions: u32,
        line_width: f32,
    ) -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let half = size * 0.5;
        let total_lines = divisions * subdivisions;
        let step = size / total_lines as f32;
        let hw = line_width * 0.5;

        let minor_color = [0.35, 0.35, 0.35, 0.4];
        let major_color = [0.5, 0.5, 0.5, 0.6];
        let x_axis_color = [0.7, 0.2, 0.2, 0.7];
        let z_axis_color = [0.2, 0.3, 0.7, 0.7];

        for i in 0..=total_lines {
            let t = -half + i as f32 * step;
            let is_major = subdivisions > 0 && i % subdivisions == 0;

            // Lines parallel to Z axis (varying X)
            let color_x = if t.abs() < step * 0.5 {
                z_axis_color
            } else if is_major {
                major_color
            } else {
                minor_color
            };
            let base = vertices.len() as u32;
            vertices.push(
                Vertex::new([t - hw, 0.0, -half])
                    .with_normal([0.0, 1.0, 0.0])
                    .with_color(color_x),
            );
            vertices.push(
                Vertex::new([t + hw, 0.0, -half])
                    .with_normal([0.0, 1.0, 0.0])
                    .with_color(color_x),
            );
            vertices.push(
                Vertex::new([t + hw, 0.0, half])
                    .with_normal([0.0, 1.0, 0.0])
                    .with_color(color_x),
            );
            vertices.push(
                Vertex::new([t - hw, 0.0, half])
                    .with_normal([0.0, 1.0, 0.0])
                    .with_color(color_x),
            );
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);

            // Lines parallel to X axis (varying Z)
            let color_z = if t.abs() < step * 0.5 {
                x_axis_color
            } else if is_major {
                major_color
            } else {
                minor_color
            };
            let base = vertices.len() as u32;
            vertices.push(
                Vertex::new([-half, 0.0, t - hw])
                    .with_normal([0.0, 1.0, 0.0])
                    .with_color(color_z),
            );
            vertices.push(
                Vertex::new([half, 0.0, t - hw])
                    .with_normal([0.0, 1.0, 0.0])
                    .with_color(color_z),
            );
            vertices.push(
                Vertex::new([half, 0.0, t + hw])
                    .with_normal([0.0, 1.0, 0.0])
                    .with_color(color_z),
            );
            vertices.push(
                Vertex::new([-half, 0.0, t + hw])
                    .with_normal([0.0, 1.0, 0.0])
                    .with_color(color_z),
            );
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        (vertices, indices)
    }
}
