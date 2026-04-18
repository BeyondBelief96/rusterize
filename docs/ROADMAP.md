# Rusterize Roadmap

A learning-oriented roadmap for evolving this software rasterizer into a 1990s-style flight simulator. Each item lists *what* it is, *why* it's worth learning, *where* in the code it lives, rough difficulty, and **curated resources** — papers, articles, and book chapters — for actually implementing it.

## Current State (baseline)

The engine already implements the core scratchapixel pipeline:

- Left-handed coordinate system, SDL2 for window/display only
- Model → World → View → Clip transforms (`engine.rs`)
- Perspective projection via `Mat4::perspective_lh` (`projection.rs`)
- Sutherland–Hodgman polygon clipping in both clip-space and view-space (`clipper/`)
- Two rasterizers: scanline (flat-top/flat-bottom) and edge-function (GPU-style) (`render/rasterizer/`)
- Per-pixel z-buffer storing `1/w` for correct linear interpolation in screen space
- Flat and Gouraud shading, directional light with ambient (`light.rs`)
- Texture modes: None / Replace / Modulate, with perspective-correct UV interpolation
- OBJ mesh loading (`tobj`), textures (`image`), multi-mesh model support
- Bresenham line drawing, backface culling, multiple render modes

## General-Purpose References

Keep these open while working on anything in this doc:

- **Scratchapixel** — <https://www.scratchapixel.com/> — the foundational site this project was built on. Lessons on Rasterization, Shading, Texture Mapping, and Projection Matrices are the primary reference for everything below.
- **Fabian "ryg" Giesen — *A Trip Through the Graphics Pipeline 2011*** — <https://fgiesen.wordpress.com/2011/07/09/a-trip-through-the-graphics-pipeline-2011-index/> — 13-part series on how modern GPUs implement the pipeline. The single best post-scratchapixel resource.
- **Fabian Giesen — *Optimizing Software Occlusion Culling*** — <https://fgiesen.wordpress.com/2013/02/17/optimizing-sw-occlusion-culling-index/> — a practical deep-dive that covers edge-function rasterization, fixed-point math, depth buffering, and SIMD tuning.
- **Real-Time Rendering, 4th ed.** (Akenine-Möller, Haines, Hoffman) — the standard reference textbook. Cited below by chapter where relevant.
- **Jim Blinn — *A Trip Down the Graphics Pipeline*** — older book but still the clearest exposition of the classical fixed-function pipeline.
- **Inigo Quilez — <https://iquilezles.org/articles/>** — particularly useful for procedural content, noise, and math tricks.

---

## Section 1 — Renderer Improvements

Ordered roughly by flight-sim criticality and learning ROI. Numbering is just for reference; see the end-of-doc schedule for suggested ordering.

### 1.1 Frustum culling at the object / chunk level

**What:** Before transforming any of a model's vertices, test its world-space bounding sphere (or AABB) against the 6 frustum planes. Skip the whole model if outside.

**Why it matters:** Hierarchical visibility is the most important optimization principle in real-time rendering. You currently transform every vertex of every model every frame, even those entirely behind the camera. For a 1000-chunk terrain, this optimization is the difference between 5 fps and 60 fps.

**Where:** Add `bounds: BoundingSphere` to `Mesh` in `src/mesh.rs`, computed at load. In `engine.rs::update()` (around the `for mesh in model.meshes()` loop), test transformed bounds against frustum planes from `Projection::view_frustum()` composed with the view matrix. The `Plane` infrastructure already exists in `clipper/view_space.rs`.

**Difficulty:** Low (~half a day).

**Resources:**
- Scratchapixel — *Rendering a Scene with a 3D Camera / Frustum Culling* lesson.
- Real-Time Rendering §19.4 "Culling Techniques" — bounding volume hierarchies and plane tests.
- Ulf Assarsson & Tomas Möller — *Optimized View Frustum Culling Algorithms for Bounding Boxes* — <http://www.cse.chalmers.se/~uffe/vfc_bbox.pdf> — the canonical paper on fast sphere/AABB-vs-frustum tests.

---

### 1.2 Sub-pixel precision + top-left fill rule

**What:** `render/rasterizer/edgefunction.rs` currently evaluates edge functions in `f32` at pixel centers with `>= 0` tests. That works for single triangles but double-shades (or gaps) pixels along shared edges. GPUs solve this with **fixed-point vertex coordinates** (4 or 8 sub-pixel bits) and the **top-left rule** for edge ownership.

**Why it matters:** Teaches fixed-point rasterization, edge ownership, and watertight meshes. Essential once you render terrain — shared ridge edges will shimmer without it.

**Where:** `render/rasterizer/edgefunction.rs::rasterize_with_shader`. Convert vertex `(x,y)` to `i32` with 4 fractional bits: `(v.x * 16.0).round() as i32`. Evaluate edges in `i64`. Bias non-top-left edges by `-1` before the `>= 0` test.

**Difficulty:** Medium. Debug with a test that draws two triangles sharing an edge and asserts no pixel is written twice.

**Resources:**
- Fabian Giesen — *Triangle rasterization in practice* — <https://fgiesen.wordpress.com/2013/02/08/triangle-rasterization-in-practice/> — the definitive write-up, covers fixed-point, top-left rule, and edge traversal.
- Fabian Giesen — *Optimizing the basic rasterizer* — <https://fgiesen.wordpress.com/2013/02/10/optimizing-the-basic-rasterizer/> — incremental edge evaluation.
- Juan Pineda — *A Parallel Algorithm for Polygon Rasterization* (SIGGRAPH 1988) — the original edge-function paper. Search for the PDF.
- Chris Hecker — *Perspective Texture Mapping* series (Game Developer Magazine, 1995–96) — <http://chrishecker.com/Miscellaneous_Technical_Articles> — classic CPU-era rasterization including sub-pixel correction.

---

### 1.3 Sky rendering (dome or skybox)

**What:** A pre-pass that fills every pixel before triangle rasterization. Two options:
- **Sky dome / gradient** — interpolate horizon→zenith color based on view direction Y. Matches the F-19 / Flight Simulator 4 aesthetic exactly.
- **Skybox** — 6 cube faces sampled with the view direction.

Render the sky at the camera's position (translation stripped) so it never appears to move.

**Why it matters:** Teaches render-pass ordering, depth-buffer interaction (sky writes color but not depth, or writes depth at the far plane), and camera-relative rendering — the foundation for environment mapping.

**Where:** New `src/sky.rs`. Called from `Engine::render()` after `clear` and before the triangle loop.

**Difficulty:** Low (procedural dome, ~3 hours). Medium for textured cubemap.

**Resources:**
- Scratchapixel — *Simulating the Colors of the Sky* — physically-based sky gradient if you want to be fancy.
- Real-Time Rendering §13.3 "Skyboxes" — standard cubemap technique.
- Preetham, Shirley, Smits — *A Practical Analytic Model for Daylight* (SIGGRAPH 1999) — if you want physically-based sky colors.

---

### 1.4 Distance fog

**What:** Blend each fragment toward a fog color based on view-space depth: `final = lerp(color, fog_color, 1 - exp(-density * depth))`. Your z-buffer stores `1/w`, so recover depth as `1.0 / inv_w`.

**Why it matters:** Hides the far-clip plane — every flight sim of that era did this. Teaches depth recovery from 1/w and shader composition.

**Where:** Wrap existing shaders in a `FogShader<S: PixelShader>` decorator in `src/render/rasterizer/shader.rs`. Composable, so texture+fog and gouraud+fog come for free.

**Difficulty:** Low (~2 hours).

**Resources:**
- Real-Time Rendering §14.4.5 "Fog" — exponential and linear fog equations.
- *GPU Gems* (free online at NVIDIA's site) — chapter on atmospheric scattering if you want to go beyond exponential fog.

---

### 1.5 Heightmap terrain renderer

**What:** A specialized renderer that consumes a 2D `Vec<f32>` heightmap and emits triangles directly. Build in phases:
1. **Naive full grid** — one quad per heightmap cell. Get something flying over terrain.
2. **Chunked grid + frustum culling** — split into 64×64 chunks with bounding boxes.
3. **Geomipmap / chunked LOD** — distant chunks render at lower resolution. Classic flight-sim technique (FS4, F-19 used this; Comanche used voxel ray-casting instead).
4. **Crack-fixing** — stitch LOD boundaries with skirts or T-junction removal.

**Why it matters:** LOD is one of the most important real-time rendering concepts. Teaches spatial partitioning, frustum culling, and procedural generation (diamond-square, fBm noise).

**Where:** New `src/terrain.rs`. Terrain is its own primitive, not a `Model` — it feeds triangles into `engine.update()`'s triangle list.

**Difficulty:** Low for naive grid (~1 day), high for full LOD (~1 week).

**Resources:**
- **Willem H. de Boer — *Fast Terrain Rendering Using Continuous Level of Detail*** (2000) — the original geomipmap paper. Search for the PDF; it's widely mirrored.
- **Thatcher Ulrich — *Rendering Massive Terrains using Chunked Level of Detail Control*** (SIGGRAPH 2002) — <http://tulrich.com/geekstuff/sig-notes.pdf> — the chunked-LOD paper. This is what you eventually want.
- Hoppe — *Smooth View-Dependent Level-of-Detail Control* (SIGGRAPH 1997) — progressive mesh approach; more advanced.
- Inigo Quilez — *fBm* — <https://iquilezles.org/articles/fbm/> — for generating the heightmap procedurally.
- Scratchapixel — *Value Noise and Procedural Patterns*.

---

### 1.6 Mipmaps + bilinear texture filtering

**What:** `Texture::sample` in `src/texture.rs` is nearest-neighbor with one mip level. Distant ground textures alias horribly (moiré on checker patterns). Fix in order:
1. **Bilinear filtering** — sample 4 texels, weighted average. Trivial.
2. **Mipmap generation** — pre-compute downsampled versions (2×2 box filter).
3. **Mip selection** — use screen-space UV derivatives (`du/dx`, `dv/dy`) to pick a level. Analytic form: `lod = 0.5 * log2(max(du/dx² + dv/dx², du/dy² + dv/dy²))`.
4. **Trilinear filtering** — blend between two adjacent mip levels.

**Why it matters:** Signal processing applied to graphics (Nyquist sampling theorem). LOD as a general principle. Mandatory for ground textures in a flight sim.

**Where:** `src/texture.rs` — add `mips: Vec<Vec<u32>>`. Selection math happens in the perspective-correct shaders in `render/rasterizer/shader.rs` (you can compute derivatives between neighboring pixels in the edge-function rasterizer).

**Difficulty:** Bilinear is low (~2 hours). Proper mip selection is medium-high.

**Resources:**
- **Lance Williams — *Pyramidal Parametrics*** (SIGGRAPH 1983) — the original mipmap paper. Short, readable, foundational.
- Scratchapixel — *Texture Mapping / Mip Mapping* lesson.
- Real-Time Rendering §6.2 "Image Texturing" — covers filtering comprehensively.
- Paul Heckbert — *Fundamentals of Texture Mapping and Image Warping* (Master's thesis, 1989) — the comprehensive reference.

---

### 1.7 Phong (per-pixel) shading

**What:** Currently `engine.rs::update()` computes lighting once per face (Flat) or per vertex (Gouraud). True Phong interpolates the *normal* perspective-correctly across the triangle and shades per pixel. Add `ShadingMode::Phong`.

**Why it matters:** Specular highlights in the middle of triangles look wrong with Gouraud. Per-pixel normals are the foundation for normal mapping later. Teaches the programmable-fragment mental model — exactly *why* GPUs went programmable.

**Where:** Extend `Triangle` in `render/rasterizer/mod.rs` to carry `vertex_normals: [Vec3; 3]`. Add `PhongShader` in `shader.rs`. Pass world-space normals from `engine.rs::update()`.

**Difficulty:** Medium.

**Flight-sim caveat:** Old sims used Gouraud or flat on aircraft. Do Phong for the learning, not the FS4 look.

**Resources:**
- **Bui Tuong Phong — *Illumination for Computer Generated Pictures*** (CACM 1975) — the original paper. Extremely readable for a '75 paper.
- Scratchapixel — *Introduction to Shading / The Phong Model*.
- James F. Blinn — *Models of Light Reflection for Computer Synthesized Pictures* (SIGGRAPH 1977) — the Blinn-Phong variant (cheaper, usually preferred).

---

### Intentionally skipped for now

- **Transparency / back-to-front sorting** — old flight sims didn't blend. Your z-buffer handles opaque correctness. Revisit if you add particles (explosions, smoke trails).
- **MSAA / supersampling** — the aesthetic you're going for is pixelated. Skip.
- **Shadow maps** — requires a second render pass from the light's POV. Nice to have, but not flight-sim-critical. Defer.

---

## Section 2 — Flight Simulator Additions

Beyond the renderer, here's the engine-level work.

### 2.1 Quaternion math

**What:** Add `Quat` to `src/math/`. Your `FpsCamera` uses Euler angles, which gimbal-lock during aerobatic maneuvers (pitch straight up and roll breaks). Quaternions are the standard fix.

**Why it matters:** Mandatory for aircraft orientation. Great math topic in its own right — you'll use them again for animation, interpolation, and anywhere else rotations need to be smoothly combined.

**Where:** New `src/math/quat.rs`. Convert to `Mat4` when building the view matrix.

**Difficulty:** Medium.

**Resources:**
- **Ken Shoemake — *Animating Rotation with Quaternion Curves*** (SIGGRAPH 1985) — the paper that introduced quaternions to graphics. Covers `slerp`.
- **3Blue1Brown — *Visualizing quaternions, an explorable video series*** — <https://eater.net/quaternions> — the best intuition-building resource, period.
- Real-Time Rendering §4.3 "Quaternions".
- Scratchapixel does NOT have a great quaternion lesson; the two above are better.

---

### 2.2 Flight model

**What:** Physics driving the aircraft. At minimum:
- **State:** position (Vec3), velocity (Vec3), orientation (Quat), angular velocity (Vec3).
- **Forces:** thrust (from throttle), drag (∝ v²), lift (`lift_coeff * angle_of_attack * speed²`), gravity.
- **Integrator:** semi-implicit Euler. Do NOT use plain explicit Euler — it diverges for orbital-style motion.

This can be "arcade" simple (Comanche-style) and still feel great. Full 6-DoF rigid-body dynamics with moments of inertia is a separate learning project if you want it.

**Where:** New `src/aircraft.rs`. Drives the camera each frame. Replace `FpsCameraController` with `AircraftController`.

**Difficulty:** Medium for arcade model, high for realistic.

**Resources:**
- **David Baraff — *Physically Based Modeling: Rigid Body Simulation*** (SIGGRAPH course notes) — <https://www.cs.cmu.edu/~baraff/sigcourse/> — the standard reference for rigid-body integration.
- **Glenn Fiedler — *Game Physics* series** — <https://gafferongames.com/categories/game-physics/> — especially the "Integration Basics" and "Physics in 3D" posts. Excellent for semi-implicit Euler and RK4.
- Stevens & Lewis — *Aircraft Control and Simulation* — the textbook on real flight dynamics. Overkill, but fascinating.
- For arcade-style, the original *Flight Simulator* papers by Bruce Artwick are hard to find but periodically surface on forums — mostly historical interest.

---

### 2.3 HUD / 2D overlay system

**What:** Render text, boxes, lines, and simple widgets on top of the 3D scene with no depth test. You already have `set_pixel` and `draw_line_bresenham`; wire them into a post-3D pass.

**Components:**
- **Bitmap font** — an 8×8 monochrome font embedded as `const [u8; 1024]` (Atari 8-bit font is public domain and perfect for the aesthetic).
- **Airspeed / altitude / heading readouts** — just text.
- **Attitude indicator** — horizon line rotated by roll, translated by pitch.
- **Heading tape, compass rose** — procedural lines.
- **Crosshair / gunsight** — a few lines.

**Where:** New `src/hud.rs`. Called from `Engine::render()` after the triangle pass.

**Difficulty:** Low.

**Resources:**
- **"FONT8x8"** — a classic public-domain 8×8 bitmap font, widely mirrored on GitHub. Search "font8x8 basic".
- Hermann Seib's flight sim pages (old but good for HUD layout inspiration).
- F-19 Stealth Fighter / Falcon 3.0 manuals (scans online) — for period-authentic HUD layouts.

---

### 2.4 Joystick / gamepad input

**What:** Extend `InputState` in `src/window.rs` with `pitch_axis: f32`, `roll_axis: f32`, `yaw_axis: f32`, `throttle: f32`. SDL2's joystick API maps cleanly via the `sdl2` crate.

**Why:** WASD flying feels nothing like a sim. This is the single biggest "feel" improvement.

**Where:** `src/window.rs`. The Rust SDL2 crate exposes `JoystickSubsystem` and `GameControllerSubsystem`.

**Difficulty:** Low.

**Resources:**
- `sdl2` crate docs — <https://docs.rs/sdl2/latest/sdl2/joystick/index.html>.
- SDL2 documentation — <https://wiki.libsdl.org/SDL2/CategoryJoystick>.

---

### 2.5 Terrain texture splatting (nice-to-have)

**What:** Blend grass / sand / rock / snow textures across terrain based on slope and altitude. Sample multiple textures per fragment and combine with weight maps.

**Where:** Extends the terrain shader (`shader.rs`).

**Difficulty:** Medium.

**Resources:**
- *GPU Gems 3*, Chapter 1 — *Generating Complex Procedural Terrains Using the GPU* — free online at NVIDIA's developer site.
- Real-Time Rendering §6.7 "Texture Compression" and §6.5 "Multipass Texturing".

---

### 2.6 Later polish (skip for v1)

- **World streaming** — chunks load/unload as you move across a 50×50 km world. Defer until you actually run out of memory.
- **Audio** — SDL2 has audio. Engine drone, gun, radio. Skip until everything else works.
- **AI wingmen / enemies** — whole separate project.

---

## Section 3 — Developer Experience

Graphics bugs are visual, non-local, and weird. Good tooling will save weeks.

### 3.1 Debug visualization render modes — **highest DX ROI**

**What:** Extend `RenderMode` with:
- **Depth**: map `1/w` to grayscale; write to color buffer.
- **Overdraw heatmap**: maintain a `Vec<u8>` overdraw counter; color gradient blue→green→yellow→red (1→2→4→8+).
- **Triangle ID**: hash each triangle to a deterministic random color.
- **Clipper output**: highlight triangles produced by the clipper in a different color.

**Where:** Extend `RenderMode` enum in `src/engine.rs`. Bind to keys 6–8.

**Difficulty:** Low (~half a day for all).

**Resources:**
- RenderDoc — <https://renderdoc.org/> — not a library, but the gold-standard tool whose feature set you're effectively reimplementing. Worth studying its UI for ideas on what's useful.

---

### 3.2 Named frame-time profile scopes

**What:** A `profile_scope!("name")` macro that times a lexical scope. Print per-second breakdown: `transform: 1.2ms | clip: 0.4ms | raster: 8.1ms | present: 0.5ms`. Pure Rust, no deps — `Instant::now()` and a thread-local `Vec<(name, duration)>`.

**Where:** New `src/profiler.rs`. Sprinkle `profile_scope!` in `engine.rs::update()` and `render()`.

**Difficulty:** Low (~3 hours).

**Resources:**
- `puffin` crate — <https://github.com/EmbarkStudios/puffin> — production-quality Rust profiler if you outgrow a homemade one.
- `tracing` crate — <https://docs.rs/tracing/> — general-purpose structured instrumentation.

---

### 3.3 Golden-image tests

**What:** Render a known scene to a PNG, diff against a checked-in reference, fail if different.

```rust
#[test]
fn cube_renders_correctly() {
    let mut engine = Engine::new(256, 256);
    engine.add_model("cube", "tests/fixtures/cube.obj")?;
    engine.update();
    engine.render();
    assert_image_matches("tests/golden/cube.png", engine.frame_buffer());
}
```

**Why:** Catches silent regressions (broken perspective-correct UVs, wrong winding, off-by-one pixel placement). The standard renderer QA technique.

**Where:** New `tests/golden.rs`. The `image` crate (already a dep) handles PNGs. Allow a 1-LSB tolerance for floating-point drift.

**Difficulty:** Medium. Infrastructure is ~half a day; reference images iterate.

**Resources:**
- `image` crate docs — <https://docs.rs/image/>.
- `insta` crate — <https://insta.rs/> — snapshot testing framework; there's an `insta-image` adapter, or roll your own.

---

### 3.4 Hot-reloaded config / tweak overlay

**What:** Either a TOML file watched by the `notify` crate (light/fog/FOV reload live), or eventually a real HUD-based slider overlay. Right now you recompile to change FOV — the time cost compounds every feature.

**Where:** Build on the HUD system (§2.3) or a standalone `config.toml`.

**Difficulty:** Low (TOML + notify), medium (in-engine UI).

**Resources:**
- `notify` crate — <https://docs.rs/notify/>.
- `serde` + `toml` crates — standard Rust config loading.
- Casey Muratori — *Handmade Hero* (early episodes on live code reloading) — <https://handmadehero.org/> — conceptually relevant even though it's C.

---

### 3.5 Headless rendering + pause / single-step

**What:** Verify `Engine` can be constructed without `Window` (it looks like it already can — the decoupling is mostly there). That enables:
- `criterion` benchmarks without a window popping up (benches already set up in `Cargo.toml`).
- Golden-image tests above.
- Deterministic CI.

Also add `Space = pause`, `. = single-step forward` to `main.rs`. Invaluable for frame-137 bugs.

**Difficulty:** Low (~2 hours).

**Resources:**
- `criterion` docs — <https://bheisler.github.io/criterion.rs/book/>.

---

### 3.6 Input record / replay (optional)

**What:** Record `InputState` per frame to disk, replay deterministically. Mandatory once you multithread the renderer.

**Difficulty:** Medium. Defer until needed.

---

## Tech Debt Worth Fixing Now

Things that look fine today but will bite as you add features.

### TD-1: `shader.rs` has affine AND perspective-correct texture variants

Scanline uses `TextureShader` (affine UVs). Edge-function uses `PerspectiveCorrectTextureShader`. **The scanline rasterizer is silently producing wrong textures on close-up triangles.** Either unify both on perspective-correct, or document scanline as the deliberately-naive educational version.

### TD-2: `clipper/view_space.rs` is entirely `#[allow(dead_code)]`

Clip-space clipping is the right choice — it's what GPUs do. The whole view-space clipping module has been dead since that switch. Resolve alongside §1.1 (see Appendix A, Step 7): promote the `Plane` + `signed_distance` type to `src/math/plane.rs` where the culler can depend on it, then delete the rest of `view_space.rs` (the polygon clipping, `ViewFrustum`, and its usage in `Projection::view_frustum`). Clip-space handles per-triangle vertex generation; the new `Plane` module serves whole-object visibility. Different jobs, now with no vestigial middle.

### TD-3: `Triangle.points: [Vec3; 3]` overloads `.z` as clip-W

Footgun. Reading the code, `.z` should mean z. Introduce:

```rust
pub struct ScreenVertex {
    pub position: Vec2,  // screen-space
    pub w: f32,          // clip-space W (for 1/w interpolation)
}
```

Do this *before* adding per-vertex normals (Phong) — otherwise you'll end up stuffing `[Vec3; 3]` everywhere.

### TD-4: `engine.rs::update()` reallocates `Vec<Vec<Triangle>>` every frame

Make `triangles_per_model` a field on `Engine`, call `.clear()` on each inner Vec instead of replacing. Capacity is preserved. Matters once terrain emits 10k+ triangles per frame.

Longer-term: collapse `update` and `render` into one pass. Per triangle: transform, cull, clip, rasterize — no intermediate buffer. Closer to a real GPU; better cache locality; makes multi-pass effects harder (but that's a bridge to cross later).

### TD-5: `renderer.rs::draw_grid` is O(width × height)

It iterates every pixel even though it only draws on grid lines. Two `step_by(spacing)` loops fix it. Trivial.

---

## Suggested Two-Week Schedule

Each block is rough effort, not calendar time. Reorder freely.

| # | Task | Section | Effort |
|---|------|---------|--------|
| 1 | Frustum culling (bounding spheres) | §1.1 | 0.5d |
| 2 | Tech-debt sweep: scanline UV fix, delete view-space clipper, `ScreenVertex` type | TD-1, TD-2, TD-3 | 1d |
| 3 | Profiler macros + depth/overdraw visualization | §3.2, §3.1 | 1d |
| 4 | Sky dome + distance fog | §1.3, §1.4 | 1d |
| 5 | Quaternion math + flight model + aircraft camera | §2.1, §2.2 | 2–3d |
| 6 | Naive heightmap terrain | §1.5 phase 1 | 2d |
| 7 | Bilinear texture filtering | §1.6 phase 1 | 0.5d |
| 8 | HUD: font + airspeed/altitude/heading/attitude | §2.3 | 1–2d |

**End state:** flying over textured terrain with a HUD inside two weeks.

After that, iterate on polish: chunked terrain LOD, mipmap generation + LOD selection, Phong shading, top-left fill rule, golden-image tests, joystick support, sky textures, terrain texture splatting.

## Files you'll touch most

- `src/engine.rs` — pipeline orchestration
- `src/render/rasterizer/shader.rs` — fog, mipmaps, Phong, terrain shaders
- `src/render/rasterizer/edgefunction.rs` — sub-pixel, top-left rule
- `src/texture.rs` — bilinear, mipmaps
- `src/camera.rs` — quaternion-based aircraft controller (or split into new file)

## New files you'll create

- `src/terrain.rs`
- `src/sky.rs`
- `src/hud.rs`
- `src/aircraft.rs`
- `src/math/quat.rs`
- `src/profiler.rs`
- `tests/golden.rs`

---

## Further Reading (big-picture)

Beyond the per-technique references, these are worth keeping on hand:

- **Peter Shirley — *Ray Tracing in One Weekend*** — <https://raytracing.github.io/> — not directly applicable, but understanding ray tracing sharpens your rasterization intuition and is a great weekend detour.
- **Pharr, Jakob, Humphreys — *Physically Based Rendering*** — <https://pbr-book.org/> — free online. Reference material for when you eventually care about shading correctness.
- ***Handmade Hero* by Casey Muratori** — <https://handmadehero.org/> — hundreds of hours building a game engine in C from scratch, including a software renderer. Conceptually parallel to what you're doing.
- ***Journal of Computer Graphics Techniques*** — <http://jcgt.org/> — free, peer-reviewed, practical. Great source for implementable papers.
- ***GPU Gems 1–3*** — <https://developer.nvidia.com/gpugems/> — free online. Despite the name, most chapters translate to CPU rendering.
- **Michael Abrash — *Graphics Programming Black Book*** — <http://www.jagregory.com/abrash-black-book/> — free online. The software-rendering era bible; covers Quake's renderer in detail.

---

## Appendix A: Implementing Frustum Culling (§1.1)

A build-order guide for §1.1 — from naive to optimized. The optimization you hypothesized (remembering the plane that rejected last frame) is real and covered below as **Optimization A**; the Assarsson–Möller paper also contributes the tighter **n/p-vertex** test covered as **Optimization B**.

### What this touches

- `src/mesh.rs` — add a `BoundingSphere` field computed at load
- `src/clipper/view_space.rs` — the whole module is `#[allow(dead_code)]` today. We'll salvage the `Plane` + `signed_distance` type for culling, then delete the rest of the module in Step 7.
- `src/projection.rs` — existing `view_frustum()` (also `#[allow(dead_code)]`) returns 6 view-space planes; used temporarily through Steps 2–6, retired in Step 7.
- `src/engine.rs::update()` — insert the cull test just above the `for face in faces` loop at line 333

Important framing: nothing in the engine currently calls `ViewFrustum` or the view-space polygon clipper. Active clipping is `ClipSpaceClipper` (`engine.rs:95,444`), and that stays untouched. The only reason this appendix mentions `view_space.rs` is that `Plane` happens to live there — we're cannibalizing it for culling and cleaning up the module on the way out.

### Step 1 — Compute bounds at load time

```rust
// src/mesh.rs
#[derive(Clone, Copy, Debug)]
pub(crate) struct BoundingSphere {
    pub center: Vec3,   // model space
    pub radius: f32,
}

impl BoundingSphere {
    pub fn from_vertices(vertices: &[Vertex]) -> Self {
        // Centroid + max-distance is a loose but correct sphere.
        // Swap in Ritter's algorithm later if too loose.
        let n = vertices.len() as f32;
        let center = vertices.iter().map(|v| v.position).sum::<Vec3>() / n;
        let radius = vertices
            .iter()
            .map(|v| (v.position - center).length())
            .fold(0.0_f32, f32::max);
        Self { center, radius }
    }
}
```

Store on `Mesh`; compute in `Mesh::new` from the vertex slice.

### Step 2 — Get world-space frustum planes each frame

`Projection::view_frustum()` gives 6 planes in view space. You have two options:
1. **Transform planes to world space** using the inverse view matrix (what's shown below).
2. **Transform the bounds to view space** (one Vec3 per mesh vs six planes once per frame). Cheaper when you have >6 meshes — you almost always do.

Option 2 in practice:

```rust
// Engine::update(), before the model loop
let frustum = self.projection.view_frustum();          // 6 view-space planes
let view_matrix = self.camera.view_matrix();            // you already have this
```

Then per mesh:

```rust
let world_center = world_matrix * mesh.bounds.center;
let view_center  = view_matrix  * world_center;
// radius must be scaled by the largest scale component across model + mesh transforms:
let s_model = model.transform().scale();
let s_mesh  = mesh.transform().scale();
let scale_max = (s_model.x * s_mesh.x).abs()
    .max((s_model.y * s_mesh.y).abs())
    .max((s_model.z * s_mesh.z).abs());
let view_radius = mesh.bounds.radius * scale_max;
```

(Exposing `ViewFrustum::planes` via a getter, or making the field `pub(crate)`, is enough — you also need to drop the `#[allow(dead_code)]` on `Projection::view_frustum` in `src/projection.rs:92`.)

### Step 3 — Naive sphere-vs-6-planes test

```rust
fn sphere_in_frustum(center: Vec3, radius: f32, planes: &[Plane; 6]) -> bool {
    for plane in planes {
        // signed_distance > 0 means "inside" (normals point inward in this codebase)
        if plane.signed_distance(center) < -radius {
            return false; // fully outside this plane → fully outside the frustum
        }
    }
    true
}
```

Slot it into `engine.rs::update()` above `for face in faces.iter()`:

```rust
if !sphere_in_frustum(view_center, view_radius, &frustum.planes) {
    continue; // skip the whole face loop for this mesh
}
```

At this point you have working culling. **Measure before optimizing** — add a per-frame counter (`meshes_tested`, `meshes_culled`) and a `criterion` bench with a scene of scattered cubes. The vertex transforms you're skipping will dwarf the cost of the test.

### Step 4 — Optimization A: Plane coherency (your hypothesis)

Over consecutive frames, objects that are outside the frustum are usually rejected by the *same plane*. Test that plane first.

```rust
// src/mesh.rs — add a mutable cache cell; Cell<> lets update() stay &
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CullCache {
    last_rejecting_plane: Option<u8>, // 0..6
}

// on Mesh:
cull_cache: Cell<CullCache>,
```

```rust
fn sphere_in_frustum_cached(
    center: Vec3, radius: f32, planes: &[Plane; 6], cache: &Cell<CullCache>,
) -> bool {
    let mut c = cache.get();

    // Hot path: last rejecting plane still rejects → bail in 1 test
    if let Some(idx) = c.last_rejecting_plane {
        if planes[idx as usize].signed_distance(center) < -radius {
            return false;
        }
    }

    for (i, plane) in planes.iter().enumerate() {
        if Some(i as u8) == c.last_rejecting_plane { continue; }
        if plane.signed_distance(center) < -radius {
            c.last_rejecting_plane = Some(i as u8);
            cache.set(c);
            return false;
        }
    }

    c.last_rejecting_plane = None;
    cache.set(c);
    true
}
```

Expected gain: off-screen objects drop from ~6 tests to ~1.1 on average. On-screen objects still need all 6 — no help there. This is the single highest-value optimization in the Assarsson–Möller paper.

A related cheaper trick: **temporal coherency at the object level** — if neither the camera nor the mesh moved this frame, reuse last frame's in/out result entirely. Only worth it if you have thousands of static meshes.

### Step 5 — Optimization B: AABB with the n/p-vertex test

Spheres are loose on elongated geometry (a runway, a mountain ridge). Replace `BoundingSphere` with `BoundingAabb { min: Vec3, max: Vec3 }`. The n/p-vertex trick tests only **one corner** per plane:

```rust
fn aabb_outside_plane(min: Vec3, max: Vec3, plane: &Plane) -> bool {
    // "positive vertex" = the corner farthest along the plane normal
    let p = Vec3::new(
        if plane.normal.x >= 0.0 { max.x } else { min.x },
        if plane.normal.y >= 0.0 { max.y } else { min.y },
        if plane.normal.z >= 0.0 { max.z } else { min.z },
    );
    plane.signed_distance(p) < 0.0  // if p-vertex is outside, whole box is outside
}
```

Transform the 8 AABB corners to view space (or reconstruct the view-space AABB from the 8 transformed corners — enclosing AABB of an OBB). Spheres are cheaper and simpler — only upgrade if you're seeing false-positive culling misses on long objects.

### Step 6 — Optimization C: Hierarchical model → mesh

Your engine already has `Model → [Mesh]`. Add a `BoundingSphere` to `Model` that encloses all of its meshes' bounds. Then:

```rust
enum FrustumTest { Outside, FullyInside, Intersecting }

for model in &self.models {
    match classify_model_bounds(model, &planes) {
        FrustumTest::Outside => continue,                     // skip model + all meshes
        FrustumTest::FullyInside => render_without_culling(), // skip per-mesh tests
        FrustumTest::Intersecting => {
            for mesh in model.meshes() { /* per-mesh cull + render */ }
        }
    }
}
```

`classify_` returns three states instead of a bool: if `signed_distance >= +radius` for **every** plane, the sphere is fully inside and every descendant is fully inside too. Big win when meshes per model is high (>8).

Further refinement from the paper — **masking**: track *which* planes the parent was fully inside. Children only need to test the remaining planes. Bit-mask of `u8` (6 bits). This compounds well with plane coherency.

### Step 7 — Cleanup: Gribb-Hartmann plane extraction

Instead of rebuilding view-space planes from FOV + near/far and transforming them, extract 6 world-space planes **directly from the view-projection matrix**. Rows of VP combine into plane equations:

- Left   = row4 + row1
- Right  = row4 − row1
- Bottom = row4 + row2
- Top    = row4 − row2
- Near   = row4 + row3 (for DirectX-style `[0,1]` z: just row3)
- Far    = row4 − row3

Normalize each `(a, b, c, d)` by `length(a, b, c)` so `d` is a signed distance you can compare directly.

Move this logic to something like `Camera::world_frustum_planes(&Projection) -> [Plane; 6]`, returning world-space planes directly. Clip-space clipping (`ClipSpaceClipper`) stays untouched — it's solving a different problem.

**This is where the TD-2 cleanup happens.** After Gribb-Hartmann replaces `ViewFrustum::new`, the view-space clipping module is entirely vestigial except for the `Plane` type, which the culler now depends on. Do the module surgery in three moves:

1. **Promote `Plane`** out of `clipper/view_space.rs` into its own home — either `src/math/plane.rs` (alongside `vec3`, `mat4`) or a new `src/geometry.rs`. The culler's `[Plane; 6]` frustum now lives in the math layer, which is where other culling/collision primitives will eventually join it.
2. **Delete the polygon-clipping code** in `view_space.rs` — `ClipVertex`, `ClipPolygon::clip_against_plane`, `ClipPolygon::triangulate`, and `ViewFrustum::{new, clip_polygon}`. All of it has been dead since the clip-space clipper replaced it.
3. **Delete `src/clipper/view_space.rs`** and remove the `mod view_space;` line from `src/clipper/mod.rs`. The `clipper/` module now contains only `clip_space.rs`.

Also drop `#[allow(dead_code)]` from `Projection::view_frustum` (`src/projection.rs:92`) — or just delete that method, since after Gribb-Hartmann nothing calls it either.

Net effect: one new module (`math/plane.rs`), one deleted module (`clipper/view_space.rs`), one simplified `Projection` impl, and a culler that extracts world-space planes directly from the VP matrix. Three commits' worth of cleanup that leaves the codebase noticeably smaller and more honest about what actually runs.

### Verification

- **Visual**: temporarily render culled meshes in a different color (or draw only their bounding spheres as wireframe). Any that appear on-screen mean your plane math is wrong.
- **Counter HUD**: print `{tested}/{culled}/{drawn}` per frame. In a scattered-cube scene with the camera looking at one cube, expect culled/tested > 0.9.
- **Criterion bench**: a scene of 1000 randomly-placed cubes, benched with culling on vs off. Should be a multiple-of-10 speedup for the typical case where most cubes are off-screen.

### Order of implementation

1. Step 1 — `BoundingSphere` on `Mesh` (~30 min).
2. Step 3 — naive sphere-vs-planes, using transform-bounds-to-view-space. Commit. Measure. (~1–2 hrs)
3. Step 4 — rejecting-plane cache. Commit. Measure. (~1 hr)
4. Step 7 — Gribb-Hartmann cleanup; retires `ViewFrustum::new`. Commit. (~2 hrs)
5. Step 6 — hierarchical `Model` bounds if your scenes have many meshes per model. (~1–2 hrs)
6. Step 5 — AABB + n/p-vertex only if sphere false-positives become visible. (~2–3 hrs)

Total: half a day for a useful culler, a full day for the mature version.

### References for this appendix

- **Assarsson & Möller — *Optimized View Frustum Culling Algorithms for Bounding Boxes*** — <http://www.cse.chalmers.se/~uffe/vfc_bbox.pdf> — plane coherency, masking, n/p-vertex test.
- **Gribb & Hartmann — *Fast Extraction of Viewing Frustum Planes from the World-View-Projection Matrix*** — the canonical reference for Step 7. Search by title.
- **Christer Ericson — *Real-Time Collision Detection*, Chapter 4** — the definitive textbook treatment of bounding volumes and plane tests.
- **Jack Ritter — *An Efficient Bounding Sphere*** (Graphics Gems I, 1990) — better bounding-sphere construction than the centroid approach.
