use iced_winit::winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::graphics::GraphicsState;

pub struct Engine {
    event_loop: Option<EventLoop<()>>,
    window: Window,
    pub graphics_state: GraphicsState,
    screens: Vec<Box<dyn Screen>>,
}

impl Engine {
    pub fn new() -> Self {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop).unwrap();
        let graphics_state = GraphicsState::new(&window);
        Self {
            event_loop: Some(event_loop),
            window,
            graphics_state,
            screens: vec![],
        }
    }

    pub fn add_screen(&mut self, screen: impl Into<Box<dyn Screen>>) {
        self.screens.push(screen.into());
    }

    pub fn run(mut self) {
        let evloop = self.event_loop.take().unwrap();
        //let id = self.window.id();
        evloop.run(move |event, _, control_flow| {
            if let Some(screen) = { self.screens.last_mut() } {
                screen.update(event);
                screen.render(&self.graphics_state);
            } else {
                *control_flow = ControlFlow::Exit;
            }
        });
    }
}

pub trait Screen {
    fn update(&mut self, event: Event<()>);
    fn render(&self, engine: &GraphicsState);
}