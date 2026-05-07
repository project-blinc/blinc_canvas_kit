# blinc_canvas_kit

Interactive canvas toolkit for 2D editors, 3D scene viewers, and node graphs — pan, zoom, spatial indexing, multi-select, PBR mesh rendering, orbit camera, and more.

## Overview

`blinc_canvas_kit` provides two main toolkits:

### CanvasKit — 2D infinite canvas

Everything needed to build interactive infinite canvas applications:

- **Viewport Management**: Pan (with momentum), zoom (scroll + pinch), coordinate conversion between screen-space and content-space
- **Spatial Indexing**: Uniform-grid spatial hash for O(1) hit testing and fast range queries
- **Multi-Select**: Shift+click to add, Cmd/Ctrl+click to toggle, with selection change callbacks
- **Marquee Selection**: Box-select via tool mode or Shift+drag
- **Snap-to-Grid**: Round content-space positions to configurable grid spacing
- **Background Patterns**: Infinite dot grid, line grid, and crosshatch with zoom-adaptive density
- **Viewport Culling**: Skip rendering off-screen elements
- **Hit Regions**: Register interactive bounding boxes with click/hover/drag callbacks

### SceneKit3D — 3D scene viewer (mini Three.js)

A Three.js-inspired API for 3D mesh rendering within Blinc's canvas element:

- **Orbit Camera**: Drag to orbit, scroll to zoom, with momentum deceleration
- **PBR Mesh Rendering**: Cook-Torrance BRDF with per-texel metallic/roughness/emissive/AO textures
- **IBL Environment**: Procedural studio lighting cubemap with HDR area lights for realistic reflections
- **HDR + Tonemapping**: Rgba16Float intermediate with ACES filmic tonemap
- **Bloom**: Threshold extraction + Kawase blur post-process
- **Infinity Grid**: Anti-aliased ground-plane grid via `CustomRenderPass` at the `Scene3D` stage
- **Geometry Primitives**: `Geometry::cube`, `sphere`, `plane`, `cylinder`, `torus` with normals + UVs
- **Material Builder**: `MaterialBuilder::standard().color(c).metallic(m).roughness(r)`
- **Scene Management**: `kit.add(geometry, material) → MeshHandle`, transform updates, auto-rendering

## Quick Start — 2D Canvas

```rust
use blinc_canvas_kit::prelude::*;

let mut kit = CanvasKit::new("editor")
    .with_background(CanvasBackground::dots().with_spacing(25.0))
    .with_snap(25.0);

kit.on_element_click(|evt| {
    if let Some(id) = &evt.region_id {
        println!("Clicked {id}");
    }
});

kit.element(|ctx, _bounds| {
    let rect = Rect::new(100.0, 100.0, 200.0, 150.0);
    ctx.fill_rect(rect, 8.0.into(), Brush::Solid(Color::BLUE));
    kit.hit_rect("my_node", rect);
})
```

## Quick Start — 3D Scene

```rust
use blinc_canvas_kit::prelude::*;
use blinc_core::{Color, Light, Mat4, Vec3};

let kit = SceneKit3D::new("viewer")
    .with_camera(OrbitCamera::default().with_distance(5.0))
    .with_light(Light::Directional {
        direction: Vec3::new(-0.4, -1.0, -0.3).normalize(),
        color: Color::WHITE,
        intensity: 2.5,
        cast_shadows: false,
    })
    .with_grid();

// Add primitives with materials
let cube = kit.add(
    Geometry::cube(1.0),
    MaterialBuilder::standard()
        .color(Color::from_hex(0x4488FF))
        .metallic(0.5)
        .roughness(0.3),
);

kit.set_position(cube, Vec3::new(0.0, 0.5, 0.0));

// Self-rendering — no draw closure needed
kit.element_auto()
```

## Features

### Tool Modes (2D)

```rust
kit.set_tool(CanvasTool::Pan);    // Background drag pans
kit.set_tool(CanvasTool::Select); // Background drag draws marquee
```

### Selection (2D)

```rust
let sel = kit.selection();
if kit.is_selected("node_1") { /* draw highlight */ }
kit.set_selection(HashSet::from(["a".into(), "b".into()]));
```

### Scene Objects (3D)

```rust
// Add from geometry + material
let sphere = kit.add(
    Geometry::sphere(0.5, 32),
    MaterialBuilder::standard().color(Color::RED).metallic(1.0),
);

// Add from loaded mesh data (e.g. glTF)
let helmet = kit.add_mesh(Arc::new(mesh_data));

// Transform
kit.set_position(sphere, Vec3::new(2.0, 0.5, 0.0));
kit.set_rotation(sphere, Vec3::new(0.0, 1.57, 0.0));
kit.set_scale(sphere, Vec3::new(1.5, 1.5, 1.5));
kit.set_visible(sphere, false);
```

### Orbit Camera (3D)

```rust
let kit = SceneKit3D::new("scene")
    .with_camera(
        OrbitCamera::default()
            .with_distance(4.0)
            .with_elevation(0.3)
            .with_azimuth(0.5)
            .with_target(Vec3::new(0.0, 1.0, 0.0))
    )
    .with_drag_sensitivity(0.003)
    .with_zoom_sensitivity(0.002)
    .with_momentum_decay(0.95);
```

## Architecture

| Module | Purpose |
|--------|---------|
| `viewport` | Pan/zoom state, coordinate conversion, visibility testing |
| `pan` | Momentum panning with EMA velocity tracking |
| `zoom` | Scroll and pinch zoom handlers |
| `spatial` | Uniform-grid spatial hash for hit testing and range queries |
| `selection` | Multi-select state, marquee drag, tool modes |
| `snap` | Grid snapping for content-space coordinates |
| `background` | Infinite viewport-aware pattern rendering |
| `hit` | Hit region types and event structs |
| `scene3d` | `SceneKit3D` + `OrbitCamera` + environment cubemap generation |
| `geometry` | Primitive geometry generators (cube, sphere, plane, cylinder, torus, grid) |
| `material` | `MaterialBuilder` for ergonomic PBR material creation |
| `grid_pass` | Infinity grid as `CustomRenderPass` at `Scene3D` stage |

All state persists across UI rebuilds via `BlincContextState` keyed storage.
