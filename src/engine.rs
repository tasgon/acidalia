use iced_winit::winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub struct Engine {
    event_loop: EventLoop<()>,
    window: Window,
}

impl Engine {
    pub fn new() -> Self {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop).unwrap();
        Self {
            event_loop,
            window,
        }
    }

    pub fn run(&mut self) {

    }
}