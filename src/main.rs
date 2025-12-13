use russsty::engine::{Engine, COLOR_BACKGROUND, COLOR_GRID, COLOR_MAGENTA};
use russsty::window::{Window, WindowEvent, WINDOW_WIDTH, WINDOW_HEIGHT};
use russsty::math::{vec2::Vec2, vec3::Vec3};

const FOV_FACTOR: f32 = 640.0;  

fn setup_cube_points(cube_points: &mut Vec<Vec3>) {
    let mut x = -1.0;
    while x <= 1.0 {
        let mut y = -1.0;
        while y <= 1.0 {
            let mut z = -1.0;
            while z <= 1.0 {
                cube_points.push(Vec3::new(x, y, z));
                z += 0.25;
            }
            y += 0.25;
        }
        x += 0.25;
    }
}

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

fn update(cube_points: & Vec<Vec3>, camera_position: &Vec3, cube_rotation: &Vec3) -> Vec<Vec2> {
    cube_points
        .iter()
        .filter_map(|point: &Vec3|  {
            let mut point = point.clone();
            point = point.rotate_x(cube_rotation.x);
            point = point.rotate_y(cube_rotation.y);
            point = point.rotate_z(cube_rotation.z);
            point.z -= camera_position.z;
            project(&point)
        })
        .collect()
}

fn render(engine: &mut Engine, window: &Window, projected_points: &Vec<Vec2>) {
    engine.clear_color_buffer(COLOR_BACKGROUND);
    engine.draw_grid(50, COLOR_GRID);

    for point in projected_points.iter() {
        engine.draw_rect( window.width() as i32 / 2 + point.x as i32, window.height() as i32 / 2 + point.y as i32, 4, 4, COLOR_MAGENTA);
    }
}

fn main() -> Result<(), String> {
    let mut window = Window::new("Russsty", WINDOW_WIDTH, WINDOW_HEIGHT)?;
    let mut engine = Engine::new(WINDOW_WIDTH, WINDOW_HEIGHT);
    let mut cube_points: Vec<Vec3> = vec![];
    let camera_position = Vec3::new(0.0, 0.0, -5.0);
    let mut cube_rotation = Vec3::new(0.0, 0.0, 0.0);
    setup_cube_points(&mut cube_points);
    
    loop {
        match window.poll_events() {
            WindowEvent::Quit => break,
            WindowEvent::Resize(w, h) => {
                window.resize(w, h)?;
                engine = Engine::new(w, h);
            }
            WindowEvent::None => {}
        }

        cube_rotation.y += 0.01;
        cube_rotation.z += 0.01;
        cube_rotation.x += 0.01;

        let projected_points = update(&cube_points, &camera_position, &cube_rotation);
        render(&mut engine, &window, &projected_points);
        window.present(engine.get_buffer_as_bytes())?;
    }

    Ok(())
}
