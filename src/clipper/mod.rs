//! Polygon clipping implementations.
//!
//! This module provides clipping against convex volumes using the
//! Sutherland-Hodgman algorithm. Two implementations are available:
//!
//! - [`clip_space`]: Clipping in homogeneous clip space (after projection).
//!   This is the preferred method as it uses fixed planes and doesn't need
//!   to be rebuilt when projection parameters change.
//!
//! - [`view_space`]: Clipping in view/camera space (before projection).
//!   Kept for reference but not actively used.

pub mod clip_space;
pub mod view_space;

// Re-export commonly used types from clip_space (the active implementation)
pub use clip_space::{ClipSpaceClipper, ClipSpacePolygon, ClipSpaceVertex};
