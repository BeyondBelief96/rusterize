//! 4x4 transformation matrix using column-major convention.
//!
//! # Convention
//! - Vectors are **column vectors** on the right: `Mat4 * Vec`
//! - Translation is stored in the **last column**
//! - Transforms chain **right-to-left**: `A * B * v` applies B first, then A
//!
//! # Example
//! ```ignore
//! let transform = rotation * scale;  // scale applied first, then rotation
//! let result = transform * vertex;   // transform the vertex
//! ```

use std::ops::Mul;

use super::vec3::Vec3;
use super::vec4::Vec4;

/// 4x4 matrix stored as `data[row][col]` with column-major convention.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    data: [[f32; 4]; 4],
}

impl Mat4 {
    pub fn new(data: [[f32; 4]; 4]) -> Self {
        Mat4 { data }
    }

    pub fn identity() -> Self {
        Mat4::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Creates a translation matrix.
    ///
    /// Translation is stored in the last column (column-major convention).
    pub fn translation(x: f32, y: f32, z: f32) -> Self {
        Mat4::new([
            [1.0, 0.0, 0.0, x],
            [0.0, 1.0, 0.0, y],
            [0.0, 0.0, 1.0, z],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Creates a scale matrix.
    pub fn scaling(x: f32, y: f32, z: f32) -> Self {
        Mat4::new([
            [x, 0.0, 0.0, 0.0],
            [0.0, y, 0.0, 0.0],
            [0.0, 0.0, z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Creates a rotation matrix around the X axis.
    pub fn rotation_x(angle: f32) -> Self {
        let c = angle.cos();
        let s = angle.sin();
        Mat4::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, c, s, 0.0],
            [0.0, -s, c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Creates a rotation matrix around the Y axis.
    pub fn rotation_y(angle: f32) -> Self {
        let c = angle.cos();
        let s = angle.sin();
        Mat4::new([
            [c, 0.0, -s, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [s, 0.0, c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Creates a rotation matrix around the Z axis.
    pub fn rotation_z(angle: f32) -> Self {
        let c = angle.cos();
        let s = angle.sin();
        Mat4::new([
            [c, s, 0.0, 0.0],
            [-s, c, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Creates a perspective matrix with left-handed coordinate system.
    pub fn perspective_lh(fov: f32, aspect_ratio: f32, near: f32, far: f32) -> Self {
        let t = near * (fov / 2.0).tan();
        let r = t * aspect_ratio;
        let a = (far + near) / (near - far);
        let b = -2.0 * far * near / (far - near);
        Mat4::new([
            [near / r, 0.0, 0.0, 0.0],
            [0.0, near / t, 0.0, 0.0],
            [0.0, 0.0, a, b],
            [0.0, 0.0, 1.0, 0.0],
        ])
    }

    /// Creates a view matrix with left-handed coordinate system.
    ///
    /// # Arguments
    ///
    /// * `eye` - The position of the camera.
    /// * `target` - The point the camera is looking at.
    /// * `up` - The up direction of the camera.
    ///
    /// # Returns
    ///
    /// A view matrix.
    pub fn look_at_lh(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let forward = (target - eye).normalize();
        let right = up.cross(forward).normalize();
        let up = forward.cross(right).normalize();

        // Rotation matrix (transpose of basis vectors as rows)
        // Combined with translation to eye position
        Self::new([
            [right.x, right.y, right.z, -right.dot(eye)],
            [up.x, up.y, up.z, -up.dot(eye)],
            [forward.x, forward.y, forward.z, -forward.dot(eye)],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Returns a new matrix with translation applied: `self * Mat4::translation(x, y, z)`.
    pub fn translate(&self, x: f32, y: f32, z: f32) -> Self {
        *self * Mat4::translation(x, y, z)
    }

    /// Returns a new matrix with transpose applied: `self.transpose()`.
    pub fn transpose(&self) -> Self {
        Mat4 {
            data: [
                [
                    self.data[0][0],
                    self.data[1][0],
                    self.data[2][0],
                    self.data[3][0],
                ],
                [
                    self.data[0][1],
                    self.data[1][1],
                    self.data[2][1],
                    self.data[3][1],
                ],
                [
                    self.data[0][2],
                    self.data[1][2],
                    self.data[2][2],
                    self.data[3][2],
                ],
                [
                    self.data[0][3],
                    self.data[1][3],
                    self.data[2][3],
                    self.data[3][3],
                ],
            ],
        }
    }

    /// Computes the inverse of the matrix, if it exists.
    /// Returns `None` if the matrix is singular (determinant is zero).
    pub fn inverse(&self) -> Option<Mat4> {
        let m = &self.data;

        // Calculate cofactors for the first row (needed for determinant)
        let c00 = m[1][1] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
            - m[1][2] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
            + m[1][3] * (m[2][1] * m[3][2] - m[2][2] * m[3][1]);

        let c01 = -(m[1][0] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
            - m[1][2] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
            + m[1][3] * (m[2][0] * m[3][2] - m[2][2] * m[3][0]));

        let c02 = m[1][0] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
            - m[1][1] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
            + m[1][3] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]);

        let c03 = -(m[1][0] * (m[2][1] * m[3][2] - m[2][2] * m[3][1])
            - m[1][1] * (m[2][0] * m[3][2] - m[2][2] * m[3][0])
            + m[1][2] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]));

        // Determinant using first row expansion
        let det = m[0][0] * c00 + m[0][1] * c01 + m[0][2] * c02 + m[0][3] * c03;

        if det.abs() < f32::EPSILON {
            return None;
        }

        let inv_det = 1.0 / det;

        // Calculate remaining cofactors
        let c10 = -(m[0][1] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
            - m[0][2] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
            + m[0][3] * (m[2][1] * m[3][2] - m[2][2] * m[3][1]));

        let c11 = m[0][0] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
            - m[0][2] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
            + m[0][3] * (m[2][0] * m[3][2] - m[2][2] * m[3][0]);

        let c12 = -(m[0][0] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
            - m[0][1] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
            + m[0][3] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]));

        let c13 = m[0][0] * (m[2][1] * m[3][2] - m[2][2] * m[3][1])
            - m[0][1] * (m[2][0] * m[3][2] - m[2][2] * m[3][0])
            + m[0][2] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]);

        let c20 = m[0][1] * (m[1][2] * m[3][3] - m[1][3] * m[3][2])
            - m[0][2] * (m[1][1] * m[3][3] - m[1][3] * m[3][1])
            + m[0][3] * (m[1][1] * m[3][2] - m[1][2] * m[3][1]);

        let c21 = -(m[0][0] * (m[1][2] * m[3][3] - m[1][3] * m[3][2])
            - m[0][2] * (m[1][0] * m[3][3] - m[1][3] * m[3][0])
            + m[0][3] * (m[1][0] * m[3][2] - m[1][2] * m[3][0]));

        let c22 = m[0][0] * (m[1][1] * m[3][3] - m[1][3] * m[3][1])
            - m[0][1] * (m[1][0] * m[3][3] - m[1][3] * m[3][0])
            + m[0][3] * (m[1][0] * m[3][1] - m[1][1] * m[3][0]);

        let c23 = -(m[0][0] * (m[1][1] * m[3][2] - m[1][2] * m[3][1])
            - m[0][1] * (m[1][0] * m[3][2] - m[1][2] * m[3][0])
            + m[0][2] * (m[1][0] * m[3][1] - m[1][1] * m[3][0]));

        let c30 = -(m[0][1] * (m[1][2] * m[2][3] - m[1][3] * m[2][2])
            - m[0][2] * (m[1][1] * m[2][3] - m[1][3] * m[2][1])
            + m[0][3] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]));

        let c31 = m[0][0] * (m[1][2] * m[2][3] - m[1][3] * m[2][2])
            - m[0][2] * (m[1][0] * m[2][3] - m[1][3] * m[2][0])
            + m[0][3] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]);

        let c32 = -(m[0][0] * (m[1][1] * m[2][3] - m[1][3] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][3] - m[1][3] * m[2][0])
            + m[0][3] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]));

        let c33 = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

        // The inverse is the transpose of the cofactor matrix divided by determinant
        Some(Mat4::new([
            [c00 * inv_det, c10 * inv_det, c20 * inv_det, c30 * inv_det],
            [c01 * inv_det, c11 * inv_det, c21 * inv_det, c31 * inv_det],
            [c02 * inv_det, c12 * inv_det, c22 * inv_det, c32 * inv_det],
            [c03 * inv_det, c13 * inv_det, c23 * inv_det, c33 * inv_det],
        ]))
    }

    /// Access element at [row][col].
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> f32 {
        self.data[row][col]
    }

    /// Set element at [row][col].
    #[inline]
    pub fn set(&mut self, row: usize, col: usize, value: f32) {
        self.data[row][col] = value;
    }
}

/// Matrix multiplication: Mat4 * Mat4.
///
/// For column-major convention, `A * B * v` applies B first, then A.
impl Mul<Mat4> for Mat4 {
    type Output = Mat4;

    fn mul(self, rhs: Mat4) -> Self::Output {
        let mut result = [[0.0f32; 4]; 4];

        for row in 0..4 {
            for col in 0..4 {
                result[row][col] = self.data[row][0] * rhs.data[0][col]
                    + self.data[row][1] * rhs.data[1][col]
                    + self.data[row][2] * rhs.data[2][col]
                    + self.data[row][3] * rhs.data[3][col];
            }
        }

        Mat4::new(result)
    }
}

/// Transform a Vec4 by a matrix: Mat4 * Vec4 (column vector).
impl Mul<Vec4> for Mat4 {
    type Output = Vec4;

    fn mul(self, v: Vec4) -> Self::Output {
        Vec4::new(
            self.data[0][0] * v.x
                + self.data[0][1] * v.y
                + self.data[0][2] * v.z
                + self.data[0][3] * v.w,
            self.data[1][0] * v.x
                + self.data[1][1] * v.y
                + self.data[1][2] * v.z
                + self.data[1][3] * v.w,
            self.data[2][0] * v.x
                + self.data[2][1] * v.y
                + self.data[2][2] * v.z
                + self.data[2][3] * v.w,
            self.data[3][0] * v.x
                + self.data[3][1] * v.y
                + self.data[3][2] * v.z
                + self.data[3][3] * v.w,
        )
    }
}

/// Transform a point: Mat4 * Vec3 (treats Vec3 as column vector with w=1).
///
/// Applies perspective division if w != 1.
impl Mul<Vec3> for Mat4 {
    type Output = Vec3;

    fn mul(self, v: Vec3) -> Self::Output {
        let x =
            self.data[0][0] * v.x + self.data[0][1] * v.y + self.data[0][2] * v.z + self.data[0][3];
        let y =
            self.data[1][0] * v.x + self.data[1][1] * v.y + self.data[1][2] * v.z + self.data[1][3];
        let z =
            self.data[2][0] * v.x + self.data[2][1] * v.y + self.data[2][2] * v.z + self.data[2][3];
        let w =
            self.data[3][0] * v.x + self.data[3][1] * v.y + self.data[3][2] * v.z + self.data[3][3];

        if w != 0.0 && w != 1.0 {
            Vec3::new(x / w, y / w, z / w)
        } else {
            Vec3::new(x, y, z)
        }
    }
}
