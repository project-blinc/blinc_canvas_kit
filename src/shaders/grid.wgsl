// Infinite ground-plane grid via per-fragment ray-plane intersection.
// Fullscreen triangle, no geometry. Unprojection happens per-fragment
// (not per-vertex) to avoid curved-line artifacts from interpolating
// world-space positions across the oversized triangle.

struct GridUniforms {
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    grid_size: f32,
    thin_color: vec4<f32>,
    thick_color: vec4<f32>,
    fade_near: f32,
    fade_far: f32,
    subdivisions: f32,
    _pad: f32,
}

@group(0) @binding(0) var<uniform> grid: GridUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

fn unproject(ndc: vec2<f32>, z: f32) -> vec3<f32> {
    let clip = vec4(ndc.x, ndc.y, z, 1.0);
    let world = grid.inv_view_proj * clip;
    return world.xyz / world.w;
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vi & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vi >> 1u)) * 4.0 - 1.0;
    out.position = vec4(x, y, 0.0, 1.0);
    out.uv = vec2((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Per-fragment NDC from UV — avoids the curved-line artifact
    // caused by interpolating world-space points from 3 vertices.
    let ndc = vec2(input.uv.x * 2.0 - 1.0, -(input.uv.y * 2.0 - 1.0));

    let near_point = unproject(ndc, 0.0);
    let far_point = unproject(ndc, 1.0);
    let ray_dir = far_point - near_point;

    // Intersect ray with Y=0 plane
    if abs(ray_dir.y) < 0.0001 {
        discard;
    }
    let t = -near_point.y / ray_dir.y;
    if t < 0.0 {
        discard;
    }

    let hit = near_point + ray_dir * t;

    // ── Grid lines ──────────────────────────────────────────────────
    let minor_size = grid.grid_size / grid.subdivisions;

    // Minor grid
    let minor_coord = hit.xz / minor_size;
    let minor_deriv = fwidth(minor_coord);
    let minor_grid = abs(fract(minor_coord - 0.5) - 0.5);
    let minor_line = min(minor_grid.x / minor_deriv.x, minor_grid.y / minor_deriv.y);
    let minor_alpha = 1.0 - min(minor_line, 1.0);

    // Major grid
    let major_coord = hit.xz / grid.grid_size;
    let major_deriv = fwidth(major_coord);
    let major_grid = abs(fract(major_coord - 0.5) - 0.5);
    let major_line = min(major_grid.x / major_deriv.x, major_grid.y / major_deriv.y);
    let major_alpha = 1.0 - min(major_line, 1.0);

    // X axis (red) and Z axis (blue) highlight — same width as grid lines
    let x_axis = smoothstep(minor_deriv.y, 0.0, abs(hit.z));
    let z_axis = smoothstep(minor_deriv.x, 0.0, abs(hit.x));

    // Compose
    var color = grid.thin_color.rgb * minor_alpha;
    color = mix(color, grid.thick_color.rgb, major_alpha);
    color = mix(color, vec3(0.8, 0.2, 0.2), x_axis);
    color = mix(color, vec3(0.2, 0.3, 0.8), z_axis);
    var alpha = max(minor_alpha * grid.thin_color.a, major_alpha * grid.thick_color.a);
    alpha = max(alpha, max(x_axis, z_axis) * 0.8);

    // Distance fade
    let dist = length(hit.xz - grid.camera_pos.xz);
    let fade = 1.0 - smoothstep(grid.fade_near, grid.fade_far, dist);
    alpha *= fade;

    if alpha < 0.01 {
        discard;
    }

    return vec4(color, alpha);
}
