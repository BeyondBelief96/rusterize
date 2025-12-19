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
- **approx**: Floating-point comparison utilities.

## Architecture

This is a CPU-based software-rendered 3D graphics engine using SDL2 only for window management and display.

### Coordinate System

**Left-handed coordinate system:**
- X-axis: positive right
- Y-axis: positive down (screen space)
- Z-axis: positive into the screen

This affects:
- Cross product calculations (use left-hand rule)
- Winding order interpretation
- Projection matrices use `Mat4::perspective_lh`

### Rendering Pipeline

1. **Mesh Loading** (`mesh.rs`): Loads OBJ files via `tobj` or uses built-in cube mesh. Faces use 1-based vertex indices.

2. **Transform & Projection** (`engine.rs:update()`):
   - Model → World: Scale, then rotation (X, Y, Z axes), then translation
   - Lighting: Computed per-face (flat) or per-vertex (Gouraud) and stored in `vertex_colors`
   - Backface culling via cross product normal and dot product with camera ray
   - Perspective projection using left-handed perspective matrix
   - Clip-space W stored in vertex z component for depth testing

3. **Rasterization** (`rasterizer/`): Two algorithms available:
   - **Scanline** (`scanline.rs`): Flat-top/flat-bottom triangle decomposition
   - **Edge Function** (`edgefunction.rs`): Bounding box iteration with edge function tests (GPU-style)
   - Both use per-pixel depth testing via z-buffer

4. **Display** (`window.rs`): FrameBuffer bytes are uploaded to an SDL streaming texture (ARGB8888) and copied to canvas.

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

- **Public API** (`lib.rs`): `engine`, `math`, `window` modules
- **Internal** (`pub(crate)`): `framebuffer`, `mesh`, `rasterizer`, `renderer`

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
