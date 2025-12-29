//! First-person camera
//!
//! # Coordinate System
//!
//! Uses a **left-handed** coordinate system:
//! - X: positive right
//! - Y: positive up
//! - Z: positive forward (into screen)
//!
//! # Orientation
//!
//! Orientation is stored as yaw/pitch/roll angles and converted to a rotation
//! matrix when needed. This is simpler than caching direction vectors.
//!
//! - **Yaw**: Rotation around Y-axis (horizontal look, positive = look right)
//! - **Pitch**: Rotation around X-axis (vertical look, positive = look down)
//! - **Roll**: Rotation around Z-axis (tilt, positive = tilt right)

use crate::math::mat4::Mat4;
use crate::math::vec3::Vec3;

/// First-person camera with position and yaw/pitch/roll orientation.
///
/// Uses matrices internally for all transformations. Direction vectors
/// are computed on-demand from the rotation matrix.
#[derive(Debug, Clone)]
pub struct FpsCamera {
    position: Vec3,
    yaw: f32,   // Rotation around Y-axis (radians)
    pitch: f32, // Rotation around X-axis (radians)
    roll: f32,  // Rotation around Z-axis (radians)

    pitch_min: f32,
    pitch_max: f32,
}

impl Default for FpsCamera {
    fn default() -> Self {
        Self::new(Vec3::ZERO)
    }
}

impl FpsCamera {
    /// Creates a new FPS camera at the given position, looking along +Z axis.
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            pitch_min: -89.0_f32.to_radians(),
            pitch_max: 89.0_f32.to_radians(),
        }
    }

    /// Creates a camera at `position` looking toward `target`.
    pub fn looking_at(position: Vec3, target: Vec3) -> Self {
        let mut camera = Self::new(position);
        camera.look_at(target);
        camera
    }

    // =========================================================================
    // Core: Rotation Matrix
    // =========================================================================

    /// Builds the rotation matrix from yaw, pitch, and roll.
    ///
    /// Order: Yaw (Y) * Pitch (X) * Roll (Z)
    /// This means roll is applied first (local), then pitch, then yaw.
    ///
    /// Note: Yaw and roll are negated to match left-handed conventions where
    /// positive yaw = look right, positive roll = tilt right.
    fn rotation_matrix(&self) -> Mat4 {
        Mat4::rotation_y(-self.yaw) * Mat4::rotation_x(self.pitch) * Mat4::rotation_z(-self.roll)
    }

    // =========================================================================
    // Orientation - Rotation
    // =========================================================================

    /// Rotates the camera by yaw (horizontal) and pitch (vertical) deltas.
    pub fn rotate(&mut self, yaw_delta: f32, pitch_delta: f32) {
        self.rotate_yaw(yaw_delta);
        self.rotate_pitch(pitch_delta);
    }

    /// Rotates the camera horizontally (around Y-axis).
    /// Positive values rotate right, negative values rotate left.
    pub fn rotate_yaw(&mut self, delta: f32) {
        self.yaw += delta;
        self.yaw = self.yaw.rem_euclid(std::f32::consts::TAU);
    }

    /// Rotates the camera vertically (around X-axis).
    /// Positive values look down, negative values look up.
    /// Automatically clamped to pitch limits.
    pub fn rotate_pitch(&mut self, delta: f32) {
        self.pitch += delta;
        self.pitch = self.pitch.clamp(self.pitch_min, self.pitch_max);
    }

    /// Rolls the camera (around Z-axis / forward vector).
    /// Positive values tilt right, negative values tilt left.
    pub fn rotate_roll(&mut self, delta: f32) {
        self.roll += delta;
        self.roll = self.roll.rem_euclid(std::f32::consts::TAU);
        if self.roll > std::f32::consts::PI {
            self.roll -= std::f32::consts::TAU;
        }
    }

    /// Points the camera at a world position.
    pub fn look_at(&mut self, target: Vec3) {
        let direction = target - self.position;
        let horizontal_len = (direction.x * direction.x + direction.z * direction.z).sqrt();

        if horizontal_len > f32::EPSILON {
            self.yaw = direction.x.atan2(direction.z);
        }

        if direction.magnitude() > f32::EPSILON {
            self.pitch = direction.y.atan2(horizontal_len);
            self.pitch = self.pitch.clamp(self.pitch_min, self.pitch_max);
        }
    }

    /// Sets the pitch limits (in radians).
    pub fn set_pitch_limits(&mut self, min: f32, max: f32) {
        self.pitch_min = min;
        self.pitch_max = max;
        self.pitch = self.pitch.clamp(self.pitch_min, self.pitch_max);
    }

    // =========================================================================
    // Movement
    // =========================================================================

    /// Moves the camera along its forward direction.
    pub fn move_forward(&mut self, distance: f32) {
        self.position = self.position + self.forward() * distance;
    }

    /// Moves the camera along its right direction (strafe).
    pub fn move_right(&mut self, distance: f32) {
        self.position = self.position + self.right() * distance;
    }

    /// Moves the camera along the world up direction.
    /// Positive distance moves up (negative Y in left-handed coords).
    pub fn move_up(&mut self, distance: f32) {
        self.position.y -= distance;
    }

    /// Moves the camera along its local up direction.
    pub fn move_local_up(&mut self, distance: f32) {
        self.position = self.position + self.up() * distance;
    }

    /// Teleports the camera to a new position without changing orientation.
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    // =========================================================================
    // Queries - Direction Vectors (computed from rotation matrix)
    // =========================================================================

    /// Returns the camera's world position.
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Returns the camera's forward direction (normalized).
    /// This is the +Z axis transformed by the rotation matrix.
    pub fn forward(&self) -> Vec3 {
        let rot = self.rotation_matrix();
        // Transform +Z unit vector: just read the third column of rotation matrix
        Vec3::new(rot.get(0, 2), rot.get(1, 2), rot.get(2, 2)).normalize()
    }

    /// Returns the camera's right direction (normalized).
    /// This is the +X axis transformed by the rotation matrix.
    pub fn right(&self) -> Vec3 {
        let rot = self.rotation_matrix();
        // Transform +X unit vector: just read the first column of rotation matrix
        Vec3::new(rot.get(0, 0), rot.get(1, 0), rot.get(2, 0)).normalize()
    }

    /// Returns the camera's up direction (normalized).
    /// This is the -Y axis transformed by the rotation matrix (Y-down system).
    pub fn up(&self) -> Vec3 {
        let rot = self.rotation_matrix();
        // Transform -Y unit vector: negate the second column
        Vec3::new(-rot.get(0, 1), -rot.get(1, 1), -rot.get(2, 1)).normalize()
    }

    /// Returns the yaw angle in radians.
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Returns the pitch angle in radians.
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Returns the roll angle in radians.
    pub fn roll(&self) -> f32 {
        self.roll
    }

    // =========================================================================
    // Matrix Generation
    // =========================================================================

    /// Computes the view matrix for the rendering pipeline.
    ///
    /// View matrix = inverse of camera's world transform.
    /// For a camera with rotation R and position P:
    ///   World transform = T(P) * R
    ///   View = R^T * T(-P)
    pub fn view_matrix(&self) -> Mat4 {
        let rot = self.rotation_matrix();
        let rot_transposed = rot.transpose();

        // Apply inverse translation: rotate(-position)
        let neg_pos = self.position * -1.0;
        let translated = rot_transposed * neg_pos;

        // Build the view matrix: rotation transpose with translation in last column
        Mat4::new([
            [
                rot_transposed.get(0, 0),
                rot_transposed.get(0, 1),
                rot_transposed.get(0, 2),
                translated.x,
            ],
            [
                rot_transposed.get(1, 0),
                rot_transposed.get(1, 1),
                rot_transposed.get(1, 2),
                translated.y,
            ],
            [
                rot_transposed.get(2, 0),
                rot_transposed.get(2, 1),
                rot_transposed.get(2, 2),
                translated.z,
            ],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
}

// =============================================================================
// Camera Controller
// =============================================================================

/// Configuration and input handling for FPS camera movement.
#[derive(Debug, Clone)]
pub struct FpsCameraController {
    /// Movement speed in units per second.
    pub move_speed: f32,
    /// Mouse sensitivity in radians per pixel.
    pub look_sensitivity: f32,
    /// Roll speed in radians per second.
    pub roll_speed: f32,
}

impl Default for FpsCameraController {
    fn default() -> Self {
        Self {
            move_speed: 5.0,
            look_sensitivity: 0.002,
            roll_speed: 1.5,
        }
    }
}

impl FpsCameraController {
    /// Creates a new camera controller with the given speed and sensitivity.
    pub fn new(move_speed: f32, look_sensitivity: f32) -> Self {
        Self {
            move_speed,
            look_sensitivity,
            roll_speed: 1.5,
        }
    }

    /// Updates the camera based on input state.
    ///
    /// # Input Mapping
    /// - W/S: Move forward/backward
    /// - A/D: Strafe left/right
    /// - Q/E: Roll left/right
    /// - Space/Shift: Move up/down
    /// - Mouse: Look around (when captured)
    pub fn update(
        &self,
        camera: &mut FpsCamera,
        input: &crate::window::InputState,
        delta_time: f32,
    ) {
        let move_amount = self.move_speed * delta_time;

        if input.forward {
            camera.move_forward(move_amount);
        }
        if input.back {
            camera.move_forward(-move_amount);
        }
        if input.right {
            camera.move_right(move_amount);
        }
        if input.left {
            camera.move_right(-move_amount);
        }
        if input.up {
            camera.move_up(move_amount);
        }
        if input.down {
            camera.move_up(-move_amount);
        }

        let roll_amount = self.roll_speed * delta_time;
        if input.roll_left {
            camera.rotate_roll(-roll_amount);
        }
        if input.roll_right {
            camera.rotate_roll(roll_amount);
        }

        let (dx, dy) = input.mouse_delta;
        if dx != 0 || dy != 0 {
            camera.rotate(
                dx as f32 * self.look_sensitivity,
                dy as f32 * self.look_sensitivity,
            );
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn camera_starts_looking_forward() {
        let camera = FpsCamera::new(Vec3::ZERO);
        // In left-handed coords, forward is +Z
        assert_relative_eq!(camera.forward().z, 1.0, epsilon = 1e-5);
        assert_relative_eq!(camera.forward().x, 0.0, epsilon = 1e-5);
    }

    #[test]
    fn yaw_rotates_horizontally() {
        let mut camera = FpsCamera::new(Vec3::ZERO);
        camera.rotate_yaw(std::f32::consts::FRAC_PI_2); // 90 degrees right

        // After 90 degree yaw, forward should be +X
        assert_relative_eq!(camera.forward().x, 1.0, epsilon = 1e-5);
        assert_relative_eq!(camera.forward().z, 0.0, epsilon = 1e-5);
    }

    #[test]
    fn pitch_is_clamped() {
        let mut camera = FpsCamera::new(Vec3::ZERO);
        camera.rotate_pitch(std::f32::consts::PI); // 180 degrees - should clamp

        // Pitch should be clamped to ~89 degrees
        assert!(camera.pitch() < std::f32::consts::FRAC_PI_2);
        assert!(camera.pitch() > 0.0);
    }

    #[test]
    fn move_forward_changes_position() {
        let mut camera = FpsCamera::new(Vec3::ZERO);
        camera.move_forward(5.0);

        // Should move along +Z
        assert_relative_eq!(camera.position().z, 5.0, epsilon = 1e-5);
    }

    #[test]
    fn view_matrix_is_valid() {
        let camera = FpsCamera::looking_at(Vec3::new(0.0, 0.0, -5.0), Vec3::ZERO);
        let view = camera.view_matrix();

        // Transform origin to view space - should be 5 units in front
        let origin = view * Vec3::ZERO;
        assert_relative_eq!(origin.z, 5.0, epsilon = 1e-4);
    }

    #[test]
    fn roll_works_via_matrix() {
        let mut camera = FpsCamera::new(Vec3::ZERO);
        camera.rotate_roll(std::f32::consts::FRAC_PI_2); // 90 degrees

        // After 90 degree roll, the "up" direction should have rotated
        // Original up is -Y (0, -1, 0), after roll should be approximately +X
        let up = camera.up();
        assert_relative_eq!(up.x, 1.0, epsilon = 1e-5);
        assert_relative_eq!(up.y, 0.0, epsilon = 1e-5);
    }
}
