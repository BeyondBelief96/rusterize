# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

### macOS

```bash
# Build the project (requires SDL2 library path on macOS)
LIBRARY_PATH="/opt/homebrew/opt/sdl2/lib:$LIBRARY_PATH" cargo build

# Run the project
LIBRARY_PATH="/opt/homebrew/opt/sdl2/lib:$LIBRARY_PATH" cargo run

# Run tests
cargo test

# Run a single test
cargo test test_name

# Check for compilation errors without building
cargo check

# Run benchmarks
LIBRARY_PATH="/opt/homebrew/opt/sdl2/lib:$LIBRARY_PATH" cargo bench
```

### Windows

1. Download SDL2 development libraries from https://github.com/libsdl-org/SDL/releases
   - Get the `SDL2-devel-X.XX.X-VC.zip` file (for MSVC) or `SDL2-devel-X.XX.X-mingw.zip` (for GNU toolchain)

2. For MSVC toolchain:
   - Extract the zip file
   - Copy `SDL2.lib` from `lib\x64\` to your Rust lib directory (e.g., `C:\Users\<username>\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\x86_64-pc-windows-msvc\lib\`)
   - Copy `SDL2.dll` from `lib\x64\` to the project root (or add it to your PATH)

3. For GNU toolchain (MinGW):
   - Extract the zip file
   - Copy all `.a` files from `lib\x64\` to your Rust lib directory
   - Copy `SDL2.dll` to the project root

4. Build and run:
   ```cmd
   cargo build
   cargo run
   ```

**Alternative (bundled):** Add `features = ["bundled"]` to the sdl2 dependency in `Cargo.toml`. This compiles SDL2 from source but requires CMake installed.

## Dependencies

- **SDL2**:
  - macOS: Install via `brew install sdl2`
  - Windows: Download from https://github.com/libsdl-org/SDL/releases (see build instructions above)
  - The `sdl2` Rust crate (v0.38.0) provides bindings.
- **tobj**: OBJ file loader for mesh import.
- **image**: Texture loading from image files (PNG, JPG, etc.).
- **approx**: Floating-point comparison utilities.

## Architecture

This is a CPU-based software-rendered 3D graphics engine using SDL2 only for window management and display.

### Coordinate System

**Left-handed coordinate system:**
- X-axis: positive right
- Y-axis: positive down (screen space)
- Z-axis: positive into the screen

#### Winding order

Winding is **not stored or enforced** anywhere — the OBJ loader trusts
whatever vertex order the file provides. Front-facing vs back-facing is
derived at runtime from the direction of the face normal:

1. `face_normal = (v1 - v0).cross(v2 - v0)` (`engine.rs:472-474`)
2. `if face_normal.dot(camera_ray) < 0.0 { cull }` (`engine.rs:477-482`)

Because `Vec3::cross` uses the standard formula and the coordinate system is
left-handed, **`(B-A) × (C-A)` points toward the camera when `A → B → C` is
traversed clockwise from the viewer's side**. So in this codebase:

- **CW-wound triangles are front-facing.**
- A CCW-only mesh would have every face culled — fix by reversing indices
  on import, or by flipping the cull test sign.

The edge-function rasterizer (`edgefunction.rs:176-183`) is winding-agnostic
on the fill side — it checks the sign of the signed area per-triangle — so
clipped fragments can come out either winding without breaking fills.

#### Where handedness and winding affect the math

| Location | What depends on LH / winding |
|----------|------------------------------|
| `math/mat4.rs` `perspective_lh` | `m[3][2] = +1` so `w_clip = +z_view`; z=near → NDC −1, z=far → NDC +1. RH would flip signs. |
| `math/mat4.rs` `look_at_lh` | Basis built as `right = up.cross(forward)`; RH would swap that order. |
| `math/vec3.rs` `Vec3::cross` | Formula is handedness-neutral, but *interpretation* of the result direction follows the left-hand rule. |
| `engine.rs:472-482` | Backface cull sign (`dot < 0 = back`) relies on the LH + CW-front convention. |
| `engine.rs:592-593` | Viewport Y flip (`1.0 - ndc_y`) — NDC has +Y up, framebuffer has +Y down. |
| `frustum.rs` | Gribb-Hartmann plane extraction assumes LH clip-z range `[-1, 1]`. Explicit comment at `frustum.rs:54,74`. |
| `clipper/clip_space.rs` | Canonical clip cube `-w ≤ z ≤ w` assumes the LH z-range `perspective_lh` produces. |

The chain: LH basis → LH projection → LH clip-space z-range → frustum plane
extraction assumes that range → cross product in LH means CW = front-facing
→ backface cull `dot < 0` drops back faces. Flip any one link and you must
flip the matching pieces.

### Rendering Pipeline

1. **Mesh Loading** (`mesh.rs`): Loads OBJ files via `tobj` or uses built-in cube mesh. Faces use 1-based vertex indices.

2. **Transform & Projection** (`engine.rs:update()`):
   - Model → World: Scale, then rotation (X, Y, Z axes), then translation
   - Lighting: Computed per-face (flat) or per-vertex (Gouraud) and stored in `vertex_colors`
   - Backface culling via cross product normal and dot product with camera ray
   - Perspective projection using left-handed perspective matrix
   - Clip-space W stored in vertex z component for depth testing

3. **Clipping** (`clipper/`): Sutherland-Hodgman polygon clipping:
   - **Clip-space** (`clip_space.rs`): Clips against canonical cube (-w ≤ x,y,z ≤ w) before perspective divide
   - **View-space** (`view_space.rs`): Alternative reference implementation
   - Handles triangles extending outside frustum; may produce 1-4 triangles per input

4. **Rasterization** (`rasterizer/`): Two algorithms available:
   - **Scanline** (`scanline.rs`): Flat-top/flat-bottom triangle decomposition
   - **Edge Function** (`edgefunction.rs`): Bounding box iteration with edge function tests (GPU-style)
   - Both use per-pixel depth testing via z-buffer

5. **Display** (`window.rs`): FrameBuffer bytes are uploaded to an SDL streaming texture (ARGB8888) and copied to canvas.

### Shading Modes

Controlled via `ShadingMode` enum:
- **None**: No lighting, base color only
- **Flat** (default): One color per face based on face normal
- **Gouraud**: Per-vertex lighting interpolated across face using barycentric coordinates

### Texture Modes

Controlled via `TextureMode` enum:
- **None** (default): Use shading color only
- **Replace**: Texture color replaces shading entirely (no lighting)
- **Modulate**: Texture color multiplied by lighting intensity (vertex_colors)

Texture mapping uses perspective-correct interpolation via `PerspectiveCorrectTextureShader` and `PerspectiveCorrectTextureModulateShader`.

### Lighting

Single directional light (`light.rs`):
- Direction-based diffuse lighting
- Ambient intensity for shadow areas
- Lighting is pre-computed in `engine.rs:update()` and stored in triangle's `vertex_colors`

### Depth Buffer (Z-Buffer)

Hidden surface removal uses a per-pixel depth buffer (`framebuffer.rs`, `renderer.rs`):
- Stores **1/w** values (reciprocal of clip-space W) for each pixel
- Using 1/w because it can be linearly interpolated in screen space
- Larger 1/w values are closer to the camera
- Depth buffer cleared to 0.0 (infinitely far) at frame start
- Replaces painter's algorithm - no triangle sorting needed

### Module Visibility

- **Public API** (`lib.rs`): `camera`, `colors`, `engine`, `light`, `math`, `projection`, `texture`, `window`
- **Internal** (`pub(crate)`): `clipper`, `mesh`, `render` (contains `framebuffer`, `rasterizer`, `renderer`)

### Key Types

- **Engine**: Main facade coordinating rendering. Holds Renderer, Rasterizer, Mesh, camera state.
- **Renderer**: Owns the color buffer (`Vec<u32>`), provides primitive drawing (pixels, lines, rectangles, grid).
- **FrameBuffer**: Borrowed view into Renderer's buffer for rasterization with bounds-checked pixel access.
- **Window**: SDL2 wrapper handling events, texture management, and frame presentation.
- **Triangle**: Stores projected vertices, colors, texture coords, shading/texture modes for rasterization.

### Render Modes (keys 1-5)

Controlled via `RenderMode` enum: Wireframe, WireframeVertices, FilledWireframe (default), FilledWireframeVertices, Filled.

### Line Drawing

Uses Bresenham's algorithm (`renderer.rs:draw_line_bresenham`). DDA algorithm also available but unused.
