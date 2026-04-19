//! Polygon clipping in homogeneous clip space.
//!
//! Clipping happens after the vertex shader transforms vertices by the
//! view-projection matrix but before the perspective divide. This is the
//! approach real GPUs use — it avoids issues with vertices behind the camera
//! and doesn't require rebuilding clipping planes when projection parameters
//! change (the clip cube `-w ≤ x,y,z ≤ w` is canonical).

pub mod clip_space;

pub use clip_space::{ClipSpaceClipper, ClipSpacePolygon, ClipSpaceVertex};
