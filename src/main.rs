use russsty::engine::{Engine, RasterizerType, RenderMode};
use russsty::window::{
    FpsCounter, FrameLimiter, Key, Window, WindowEvent, WINDOW_HEIGHT, WINDOW_WIDTH,
};
use russsty::ShadingMode;

fn format_window_title(fps: f64, engine: &Engine) -> String {
    format!(
        "Russsty | FPS: {:.1} | {} | Cull: {} | {:?}",
        fps,
        engine.rasterizer(),
        if engine.backface_culling { "ON" } else { "OFF" },
        engine.render_mode()
    )
}

fn main() -> Result<(), String> {
    let mut window = Window::new("Russsty", WINDOW_WIDTH, WINDOW_HEIGHT)?;
    let mut engine = Engine::new(window.width(), window.height());

    engine
        .load_mesh("assets/f22.obj")
        .map_err(|e| e.to_string())?;

    let mut frame_limiter = FrameLimiter::new(&window);
    let mut fps_counter = FpsCounter::new();

    loop {
        match window.poll_events() {
            WindowEvent::Quit => break,
            WindowEvent::Resize(w, h) => {
                window.resize(w, h)?;
                engine.resize(w, h);
            }
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
                        ShadingMode::None => ShadingMode::None,
                        ShadingMode::Flat => ShadingMode::Gouraud,
                        ShadingMode::Gouraud => ShadingMode::Flat,
                    };
                    engine.set_shading_mode(next);
                }
            },
            WindowEvent::None => {}
        }

        let _delta_time = frame_limiter.wait_and_get_delta(&window);

        let mesh = engine.mesh_mut();
        mesh.translation_mut().z = 5.0;

        mesh.rotation_mut().x += 0.01;
        mesh.rotation_mut().y += 0.01;
        mesh.rotation_mut().z += 0.01;

        engine.update();
        engine.render();
        window.present(engine.frame_buffer())?;

        if let Some(fps) = fps_counter.tick() {
            window.set_title(&format_window_title(fps, &engine));
        }
    }

    Ok(())
}
