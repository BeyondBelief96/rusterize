use russsty::camera::FpsCameraController;
use russsty::engine::{Engine, RasterizerType, RenderMode, TextureMode};
use russsty::math::vec3::Vec3;
use russsty::texture::Texture;
use russsty::window::{
    FpsCounter, FrameLimiter, Key, Window, WindowEvent, WINDOW_HEIGHT, WINDOW_WIDTH,
};
use russsty::ShadingMode;

fn format_window_title(fps: f64, engine: &Engine, mouse_captured: bool) -> String {
    format!(
        "Russsty | FPS: {:.1} | {} | Cull: {} | render: {:?} | shade: {:?} | tex: {:?} | {}",
        fps,
        engine.rasterizer(),
        if engine.backface_culling { "ON" } else { "OFF" },
        engine.render_mode(),
        engine.shading_mode(),
        engine.texture_mode(),
        if mouse_captured {
            "WASD to move, mouse to look, M/RMB to release"
        } else {
            "M/RMB to capture mouse"
        }
    )
}

fn main() -> Result<(), String> {
    let mut window = Window::new("Russsty", WINDOW_WIDTH, WINDOW_HEIGHT)?;
    let mut engine = Engine::new(window.width(), window.height());

    engine
        .load_mesh("assets/crab.obj")
        .map_err(|e| e.to_string())?;

    let texture = Texture::from_file("assets/crab.png").map_err(|e| e.to_string())?;
    engine.set_texture(texture);

    // Start with texture mode enabled so we can see it
    engine.set_texture_mode(TextureMode::Replace);

    // Position camera to see the mesh
    engine.camera_mut().set_position(Vec3::new(0.0, 0.0, -10.0));

    // Camera controller for FPS-style movement
    let camera_controller = FpsCameraController::default();

    let mut frame_limiter = FrameLimiter::new(&window);
    let mut fps_counter = FpsCounter::new();

    loop {
        match window.poll_events() {
            WindowEvent::Quit => break,
            WindowEvent::KeyPress(Key::Escape) => break, // Escape quits
            WindowEvent::Resize(w, h) => {
                window.resize(w, h)?;
                engine.resize(w, h);
            }
            WindowEvent::RightMouseDown => window.toggle_mouse_capture(),
            WindowEvent::KeyPress(key) => match key {
                Key::Num1 => engine.set_render_mode(RenderMode::Wireframe),
                Key::Num2 => engine.set_render_mode(RenderMode::WireframeVertices),
                Key::Num3 => engine.set_render_mode(RenderMode::FilledWireframe),
                Key::Num4 => engine.set_render_mode(RenderMode::FilledWireframeVertices),
                Key::Num5 => engine.set_render_mode(RenderMode::Filled),
                Key::C => engine.backface_culling = !engine.backface_culling,
                Key::G => engine.draw_grid = !engine.draw_grid,
                Key::R => {
                    let next = match engine.rasterizer() {
                        RasterizerType::Scanline => RasterizerType::EdgeFunction,
                        RasterizerType::EdgeFunction => RasterizerType::Scanline,
                    };
                    engine.set_rasterizer(next);
                }
                Key::F => {
                    let next = match engine.shading_mode() {
                        ShadingMode::None => ShadingMode::Flat,
                        ShadingMode::Flat => ShadingMode::Gouraud,
                        ShadingMode::Gouraud => ShadingMode::None,
                    };
                    engine.set_shading_mode(next);
                }
                Key::T => {
                    let next = match engine.texture_mode() {
                        TextureMode::None => TextureMode::Replace,
                        TextureMode::Replace => TextureMode::Modulate,
                        TextureMode::Modulate => TextureMode::None,
                    };
                    engine.set_texture_mode(next);
                }
                Key::M => window.toggle_mouse_capture(),
                _ => {}
            },
            WindowEvent::None => {}
        }

        let delta_ms = frame_limiter.wait_and_get_delta(&window);
        let delta_time_sec = delta_ms as f32 / 1000.0;

        // Update camera when mouse is captured
        if window.is_mouse_captured() {
            camera_controller.update(engine.camera_mut(), window.input_state(), delta_time_sec);
        }

        engine.update();
        engine.render();
        window.present(engine.frame_buffer())?;

        if let Some(fps) = fps_counter.tick() {
            window.set_title(&format_window_title(
                fps,
                &engine,
                window.is_mouse_captured(),
            ));
        }
    }

    Ok(())
}
