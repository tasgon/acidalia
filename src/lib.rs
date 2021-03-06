pub mod ticker;
mod graphics;
mod shaders;
pub mod ui;
pub mod engine;

/*fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = game_render::RenderState {
        gfx_state: graphics::GraphicsState::new(&window),
        fps_counter: fps::FPSCounter::new(fps::FPSCounterConfig { seconds_per_print: std::time::Duration::from_millis(5000) }),
        background_color: (0.0, 0.0, 0.0, 1.0),
    };

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::KeyboardInput { input, .. } => match input {
                KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode: Some(VirtualKeyCode::Escape),
                    ..
                } => *control_flow = ControlFlow::Exit,
                _ => *control_flow = ControlFlow::Wait,
            },
            WindowEvent::Resized(size) => state.gfx_state.set_size(size.clone()),
            event => {
                *control_flow = ControlFlow::Wait;
                state.update(event);
                state.render();
            }
        },
        _ => *control_flow = ControlFlow::Wait,
    });
}*/

