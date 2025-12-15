use russsty::engine::{Engine, COLOR_BACKGROUND, COLOR_GRID, COLOR_MAGENTA};
use russsty::window::{WINDOW_HEIGHT, WINDOW_WIDTH, Window, WindowEvent, FrameLimiter};
use russsty::math::{vec2::Vec2, vec3::Vec3};
use russsty::triangle::Triangle;

const FOV_FACTOR: f32 = 640.0;

// Project a 3D point to a 2D point using perspective division
fn project(point: &Vec3) -> Option<Vec2> {
    // Clip points that are behind or too close to the camera
    if point.z < 0.1 {
        return None;
    }

    Some(Vec2::new(
        FOV_FACTOR * point.x / point.z,
        FOV_FACTOR * point.y / point.z,
    ))
}

fn update(camera_position: &Vec3, engine: &mut Engine){
    let faces = engine.mesh().faces().to_vec();
    let vertices = engine.mesh().vertices().to_vec();
    let rotation = engine.mesh().rotation();
    let buffer_width = engine.buffer_width();
    let buffer_height = engine.buffer_height();
    let triangles = engine.get_triangles_to_render_mut();

    for face in faces.iter() {
        let face_vertices = [
            vertices[face.a as usize - 1],
            vertices[face.b as usize - 1],
            vertices[face.c as usize - 1],
        ];

        let mut projected_points = Vec::new();
        for vertex in face_vertices.iter() {
            let mut transformed_vertex = *vertex;
            transformed_vertex = transformed_vertex.rotate_x(rotation.x);
            transformed_vertex = transformed_vertex.rotate_y(rotation.y);
            transformed_vertex = transformed_vertex.rotate_z(rotation.z);
            transformed_vertex.z -= camera_position.z;
            if let Some(mut projected) = project(&transformed_vertex) {
                // Adjust triangle points to be centered on screen
                projected.x += buffer_width as f32 / 2.0;
                projected.y += buffer_height as f32 / 2.0;
                projected_points.push(projected);
            }
        }

        // Only create triangle if all three vertices were successfully projected
        if projected_points.len() == 3 {
            triangles.push(Triangle {
                points: projected_points,
                color: COLOR_MAGENTA,
            });
        }
    }
}

fn render(engine: &mut Engine) {
    engine.clear_color_buffer(COLOR_BACKGROUND);
    engine.draw_grid(50, COLOR_GRID);

    // Draw all triangles stored in the engine
    // Collect triangles first to avoid borrow checker issues
    let triangles: Vec<Triangle> = engine.get_triangles_to_render().to_vec();
    for triangle in triangles.iter() {
        engine.draw_triangle(triangle);
    }
}

fn main() -> Result<(), String> {
    let mut window = Window::new("Russsty", WINDOW_WIDTH, WINDOW_HEIGHT)?;
    let mut engine = Engine::new(window.width(), window.height());
    engine.load_cube_mesh();

    let camera_position = Vec3::new(0.0, 0.0, -5.0);
    
    let mut frame_limiter = FrameLimiter::new(&window);
    loop  {
        match window.poll_events() {
            WindowEvent::Quit => break,
            WindowEvent::Resize(w, h) => {
                window.resize(w, h)?;
                engine.resize(w, h);
            }
            WindowEvent::None => {}
        }

        // Get delta time (in milliseconds) after frame limiting
        let delta_time = frame_limiter.wait_and_get_delta(&window);

        // Only run update/render after enough time has passed for this frame.
        engine.mesh_mut().rotation_mut().y += 0.01;
        engine.mesh_mut().rotation_mut().z += 0.01;
        engine.mesh_mut().rotation_mut().x += 0.01;

        engine.clear_triangles_to_render();
        update(&camera_position, &mut engine);
        render(&mut engine);
        window.present(engine.get_buffer_as_bytes())?;
    }

    Ok(())
}
