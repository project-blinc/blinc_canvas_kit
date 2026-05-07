# blinc_canvas_kit — Backlog

Outstanding work, split by the two toolkits this crate ships
(`CanvasKit` for 2D infinite canvas, `SceneKit3D` for PBR scene
viewer) plus the cross-cutting surface they share. Each entry notes
**why** it matters and **how** to approach it so items are pickable
cold.

---

## SceneKit3D — lighting ergonomics

- [ ] **Expose the IBL intensity knob**
  - **Why:** The procedural cubemap is currently hard-coded at 0.75
    linear in `blinc_gpu::mesh_pipeline` to match a "studio
    preset" look. Assets authored for darker environments (dusk
    city, underwater, night scenes) render too bright against
    that default, and there's no API surface to dial it down.
  - **How:** Add `SceneKit3D::with_ambient_intensity(f32)` that
    plumbs a multiplier through `Camera` / scene draw state into
    the mesh shader's IBL branch. Default stays 1.0 (no change).
    Under the hood the multiplier scales `irradiance` and
    `prefiltered` in the fragment shader.

- [ ] **Bloom threshold + strength configuration**
  - **Why:** Threshold is 0.6, bloom blur passes are fixed — works
    for character demos, probably wrong for a geometric / hard-
    edged-surface app (factory, architectural). Expose
    `SceneKit3D::with_bloom(threshold, strength)` with `None` to
    disable bloom entirely.
  - **How:** Thread through `CustomRenderPass` or add a
    `BloomConfig` field on the kit. Shader already reads
    `threshold` from the `BloomUniforms`; just plumb the value.

- [ ] **Light-rig helpers that follow the camera**
  - **Why:** Demo authors repeatedly discover that changing
    `OrbitCamera::with_azimuth(π)` moves the camera without
    moving the lights, so the directional rig ends up illuminating
    the back of the subject. Needs a helper that ties lights to
    camera-relative directions.
  - **How:** `SceneKit3D::with_three_point_rig(key_intensity)` —
    creates three directional lights with canonical positions
    (key front-up-right, fill front-up-left, rim back-up) in
    *camera space*, re-evaluated each frame from the orbit
    camera's current yaw/elevation.

---

## SceneKit3D — asset pipeline integration

- [ ] **BC texture capability probe in the public API**
  - **Why:** `blinc_gpu::GpuRenderer::has_texture_compression_bc()`
    exists, but `SceneKit3D` consumers have no way to query it
    before deciding whether to enable
    `blinc_gltf::load_asset_with_options(..., bc = true)`. Today
    the `bc-encode` cargo feature is static — a user enabling it
    on a device without BC support gets silent white textures
    (the renderer skips the upload).
  - **How:** Surface a `SceneKit3D::supports_compressed_textures()`
    helper (forwards to the underlying renderer capability) plus
    a `LoadHints` struct on `blinc_gltf::LoadOptions` that
    callers fill from the kit's probe result.

- [ ] **Texture memory budget**
  - **Why:** `blinc_gpu::mesh_pipeline` FIFO-caches GPU textures
    at `MESH_CACHE_CAPACITY = 128`. Scenes with more unique
    textures evict silently and the renderer falls back to the
    1×1 white placeholder because `TextureData::drop_cpu_bytes`
    fires after first upload (see
    `crates/blinc_gpu/src/mesh_pipeline.rs` — the post-upload
    bytes drop saves ~500 MB on the strangler rig but breaks
    re-upload on cache miss). Move the cache to LRU, or add a
    high-water-mark warning when eviction starts degrading.

---

## CanvasKit — editor ergonomics

- [ ] **Undo / redo stack integration**
  - **Why:** Every non-trivial editor consumer rebuilds their own
    undo stack on top of `CanvasKit`. A shared helper that
    captures `on_transform_change` / `on_selection_change` /
    `on_element_delete` into a pluggable `HistoryRecorder` trait
    would collapse most of the per-app boilerplate.
  - **How:** `CanvasKit::with_history(Box<dyn HistoryRecorder>)`.
    Trait carries `push_action(Action)` + `undo()` + `redo()`.
    Default implementation records `Change { element_id, before,
    after }` pairs keyed on element id.

- [ ] **Spatial-hash tuning knobs**
  - **Why:** `SpatialHash::new()` uses a fixed cell size. Scenes
    with wildly different element sizes (a 5000 px background
    rect alongside 20 px UI handles) thrash the hash with
    false-positive bucket matches.
  - **How:** Expose `CanvasKit::with_spatial_cell_size(f32)` and
    document the trade-offs. Could auto-tune on first insert by
    sampling element sizes; left for a later pass.

- [ ] **Drag-drop interop with outside windows** (OS-level file
  drop into the canvas). Currently only internal drags work.

---

## Cross-cutting — input plumbing

- [x] **Automate `capture_input` + `frame_end` wiring** — shipped as
  `SceneKit3D::with_input(&InputState)` in 922cb316. Auto-calls
  `capture_input` on the outer viewport `Div` and `frame_end()`
  inside the inner canvas closure after the user's render closure
  returns. The original backlog framing was `ctx.input() -> &InputState`,
  but tracing actual demo usage showed readers already have
  `InputState` via closure capture — the pain was in the two
  error-prone wiring sites, not in accessing the state. API shape
  matches the real pain: caller keeps owning the state, just
  opts into `.with_input(&state)` to drop the boilerplate.

- [ ] **Input-driven camera helper**
  - `SceneKit3D` already owns an `OrbitCamera` and wires pointer
    + scroll. Expose the same for `OrbitCamera::new_fly()`
    (WASD + mouselook) so first-person scene inspection is one
    line at the demo layer.

---

## Performance

- [ ] **Background grid culling cost**
  - The infinite dot/line/crosshatch grids tessellate the entire
    visible rect every frame, even on pure pan-with-no-zoom. For
    high-DPI desktop frames that's a measurable drop on low-end
    GPUs. Cache the tessellated grid buffer per (zoom, grid
    spacing) and only rebuild on zoom change.

- [ ] **Frustum culling pre-pass**
  - Currently every scene object dispatches a draw call regardless
    of whether its AABB intersects the view frustum. Stacks
    reasonably up to ~100 objects; past that the overdraw + mesh
    setup work becomes the bottleneck. Build an AABB cache on
    `add` / `set_position` and gate `draw_mesh_data` calls on it.

---

## Non-goals

- **Node-graph editor widgets** (bezier connectors, port matching,
  auto-layout). That's the job of a `blinc_node_editor` crate if
  it ever gets built.
- **ECS-style scene layering** (tags, queries, systems). SceneKit3D
  is a viewer primitive, not an engine.
- **Physics / animation curves authoring.** Playback lives in
  `blinc_skeleton`; authoring is out of scope.
