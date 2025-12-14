use russsty::engine::{Engine, COLOR_BACKGROUND, COLOR_GRID, COLOR_MAGENTA};
use russsty::window::{WINDOW_HEIGHT, WINDOW_WIDTH, Window, WindowEvent, FrameLimiter};
use russsty::math::{vec2::Vec2, vec3::Vec3};
use russsty::mesh::{MESH_VERTICES, MESH_FACES};
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



fn update(camera_position: &Vec3, cube_rotation: &Vec3) -> Vec<Triangle> {
    let mut triangles_to_render = Vec::new();

    for face in MESH_FACES.iter() {
        let face_vertices = [
            MESH_VERTICES[face.a as usize - 1],
            MESH_VERTICES[face.b as usize - 1],
            MESH_VERTICES[face.c as usize - 1],
        ];

        let mut projected_points = Vec::new();
        for vertex in face_vertices.iter() {
            let mut transformed_vertex = *vertex;
            transformed_vertex = transformed_vertex.rotate_x(cube_rotation.x);
            transformed_vertex = transformed_vertex.rotate_y(cube_rotation.y);
            transformed_vertex = transformed_vertex.rotate_z(cube_rotation.z);
            transformed_vertex.z -= camera_position.z;
            if let Some(projected) = project(&transformed_vertex) {
                projected_points.push(projected);
            }
        }

        // Only create triangle if all three vertices were successfully projected
        if projected_points.len() == 3 {
            triangles_to_render.push(Triangle {
                points: projected_points,
                color: COLOR_MAGENTA,
            });
        }
    }

    triangles_to_render
}

fn render(engine: &mut Engine, window: &Window) {
    engine.clear_color_buffer(COLOR_BACKGROUND);
    engine.draw_grid(50, COLOR_GRID);

    // Draw all triangles stored in the engine
    // Collect triangles first to avoid borrow checker issues
    let triangles: Vec<Triangle> = engine.get_triangles_to_render().to_vec();
    for triangle in triangles.iter() {
        // Adjust triangle points to be centered on screen
        let mut adjusted_triangle = triangle.clone();
        for point in adjusted_triangle.points.iter_mut() {
            point.x += window.width() as f32 / 2.0;
            point.y += window.height() as f32 / 2.0;
        }
        engine.draw_triangle(&adjusted_triangle);
    }
}

fn main() -> Result<(), String> {
    let mut window = Window::new("Russsty", WINDOW_WIDTH, WINDOW_HEIGHT)?;
    let mut engine = Engine::new(window.width(), window.height());

    let camera_position = Vec3::new(0.0, 0.0, -5.0);
    let mut cube_rotation = Vec3::new(0.0, 0.0, 0.0);
    
    let mut frame_limiter = FrameLimiter::new(&window);
    loop  {
        match window.poll_events() {
            WindowEvent::Quit => break,
            WindowEvent::Resize(w, h) => {
                window.resize(w, h)?;
                engine = Engine::new(w, h);
            }
            WindowEvent::None => {}
        }

        // Get delta time (in milliseconds) after frame limiting
        let delta_time = frame_limiter.wait_and_get_delta(&window);

        // Only run update/render after enough time has passed for this frame.
        cube_rotation.y += 0.01;
        cube_rotation.z += 0.01;
        cube_rotation.x += 0.01;

        let triangles_to_render = update(&camera_position, &cube_rotation);
        engine.set_triangles_to_render(triangles_to_render);
        render(&mut engine, &window);
        window.present(engine.get_buffer_as_bytes())?;
    }

    Ok(())
}
