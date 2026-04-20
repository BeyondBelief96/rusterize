//! §1.2 regression test — sub-pixel precision + top-left fill rule.
//!
//! Locks in the watertightness behavior that the planned §1.2 rewrite is
//! designed to produce: when two triangles share an edge, every boundary
//! pixel must be owned by exactly one of them — no double-writes.
//!
//! **TODAY THIS TEST FAILS**, and that is the point. The current
//! `EdgeFunctionRasterizer` evaluates edges in `f32` with a `>= 0` /
//! `<= 0` inside test, so pixels whose center lies exactly on a shared
//! edge pass the test for both triangles and get rasterized twice.
//! Appendix B in `docs/ROADMAP.md` tracks the fix (fixed-point vertex
//! coordinates with `i64` edge math + the top-left fill rule bias).
//! When that lands the test turns green.
//!
//! # Why an axis-aligned shared edge at `y = k + 0.5`
//!
//! Pixel centers are at `(x + 0.5, y + 0.5)` for integer `x`, `y`. An
//! edge running exactly through `y = k + 0.5` has `B.y - A.y == 0`, so
//! the edge function
//!
//!     E(A, B, P) = (B.x - A.x)*(P.y - A.y) - (B.y - A.y)*(P.x - A.x)
//!
//! collapses to `(B.x - A.x) * (P.y - A.y)`. For every pixel center on
//! that row, `P.y - A.y == 0` exactly (no floating-point slop — these
//! are representable values and subtraction is exact), so `E == 0.0` on
//! the nose. Both triangles claim the pixel. With a non-axis-aligned
//! shared edge, FP roundoff pushes most pixels slightly off the edge
//! to one side or the other, and the bug goes hidden. The top-left
//! rule has to handle the "exactly on edge" case, so that is exactly
//! what we test.

use russsty::bench::{EdgeFunctionRasterizer, FrameBuffer, Rasterizer, ScreenVertex, Triangle};
use russsty::engine::TextureMode;
use russsty::prelude::Vec2;
use russsty::ShadingMode;

// Horizontal shared edge at y = 30.5, running from x = 10.5 to x = 50.5.
// Pixel centers on framebuffer row y = 30 have centers at y = 30.5 —
// exactly on the edge. 41 such pixels (x = 10..=50) should be claimed
// by both triangles today.
//
//                 y = 5.5
//                    C2
//                   /  \
//                  / T2 \
//                 /      \
//               A ━━━━━━━━ B y = 30.5   (shared edge)
//                \       /
//                 \  T1 /
//                  \   /
//                   \ /
//                    C1
//                 y = 70.5
const A: Vec2 = Vec2 { x: 10.5, y: 30.5 };
const B: Vec2 = Vec2 { x: 50.5, y: 30.5 };
const C1: Vec2 = Vec2 { x: 30.5, y: 70.5 };
const C2: Vec2 = Vec2 { x: 30.5, y: 5.5 };

const W: u32 = 128;
const H: u32 = 128;

fn sv(v: Vec2) -> ScreenVertex {
    // w = 1 — no perspective. 1/w = 1 at every pixel, so every covered
    // pixel clears the depth test when written into a fresh buffer.
    ScreenVertex::new(v, 1.0)
}

fn tri(points: [ScreenVertex; 3], color: u32) -> Triangle {
    Triangle::new(
        points,
        color,
        [color; 3],
        [Vec2::ZERO; 3],
        ShadingMode::None,
        TextureMode::None,
    )
}

/// Rasterize a single triangle into its own fresh buffer. Any pixel the
/// rasterizer's inside test accepts becomes non-zero. Because each call
/// gets a fresh depth buffer (cleared to 0.0) and our vertices have
/// 1/w = 1, every inside-the-triangle pixel clears the depth test —
/// nothing is masked by z-fighting, so we see exactly what the
/// coverage test accepted.
fn rasterize_alone(triangle: &Triangle) -> Vec<u32> {
    let mut color = vec![0u32; (W * H) as usize];
    let mut depth = vec![0.0f32; (W * H) as usize];
    let mut fb = FrameBuffer::new(&mut color, &mut depth, W, H);
    let rasterizer = EdgeFunctionRasterizer::new();
    rasterizer.fill_triangle(triangle, &mut fb, triangle.color, None);
    color
}

#[test]
fn shared_edge_has_no_double_writes() {
    // Both triangles are wound consistently relative to the rasterizer's
    // edge-function sign convention. The shared edge is traversed in
    // opposite directions by each triangle:
    //   T1 contains edge A→B
    //   T2 contains edge B→A
    let t1 = tri([sv(A), sv(B), sv(C1)], 0xFFFFFFFF);
    let t2 = tri([sv(B), sv(A), sv(C2)], 0xFFFFFFFF);

    let buf1 = rasterize_alone(&t1);
    let buf2 = rasterize_alone(&t2);

    let mut t1_only = 0u32;
    let mut t2_only = 0u32;
    let mut both = 0u32;
    for i in 0..(W * H) as usize {
        match (buf1[i] != 0, buf2[i] != 0) {
            (true, false) => t1_only += 1,
            (false, true) => t2_only += 1,
            (true, true) => both += 1,
            (false, false) => {}
        }
    }

    println!(
        "coverage: t1_only={t1_only} t2_only={t2_only} both={both} total_covered={}",
        t1_only + t2_only + both,
    );

    assert_eq!(
        both, 0,
        "Shared-edge pixels are being written by BOTH triangles \
         ({both} overlap). §1.2 top-left fill rule should reduce this to 0."
    );
}
