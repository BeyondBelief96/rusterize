use russsty::engine::{Engine, RasterizerType, RenderMode, TextureMode};
use russsty::texture::Texture;
use russsty::window::{
    FpsCounter, FrameLimiter, Key, Window, WindowEvent, WINDOW_HEIGHT, WINDOW_WIDTH,
};
use russsty::ShadingMode;

fn format_window_title(fps: f64, engine: &Engine) -> String {
    format!(
        "Russsty | FPS: {:.1} | {} | Cull: {} | render mode: {:?} | shading mode: {:?} | texture mode: {:?}",
        fps,
        engine.rasterizer(),
        if engine.backface_culling { "ON" } else { "OFF" },
        engine.render_mode(),
        engine.shading_mode(),
        engine.texture_mode()
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
            },
            WindowEvent::None => {}
        }

        let _delta_time = frame_limiter.wait_and_get_delta(&window);

        let mesh = engine.mesh_mut();
        mesh.translation_mut().z = 10.0;

        //mesh.rotation_mut().x += 0.01;
        mesh.rotation_mut().y += 0.01;
        // mesh.rotation_mut().z += 0.01;

        engine.update();
        engine.render();
        window.present(engine.frame_buffer())?;

        if let Some(fps) = fps_counter.tick() {
            window.set_title(&format_window_title(fps, &engine));
        }
    }

    Ok(())
}
