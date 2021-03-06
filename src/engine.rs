use iced_winit::winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use iced_wgpu::wgpu;

use crate::graphics::GraphicsState;

pub struct Engine {
    event_loop: Option<EventLoop<()>>,
    pub window: Window,
    pub graphics_state: GraphicsState,
    //screens: Vec<Box<dyn Screen>>,
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
            //screens: vec![],
        }
    }

    pub fn run(mut self, screen: impl Screen + 'static) {
        let evloop = self.event_loop.take().unwrap();
        let mut screens: Vec<Box<dyn Screen>> = vec![Box::new(screen)];
        evloop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            let last_sc = screens.last_mut();
            if let Some(mut screen) = last_sc {
                match event {
                    Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
                    Event::WindowEvent { event: WindowEvent::Resized(_), ..} => {
                        let gs = &mut self.graphics_state;
                        let size = self.window.inner_size();

                        gs.swapchain_descriptor = wgpu::SwapChainDescriptor {
                            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
                            format: wgpu::TextureFormat::Bgra8UnormSrgb,
                            width: size.width as u32,
                            height: size.height as u32,
                            present_mode: wgpu::PresentMode::Mailbox,
                        };
        
                        gs.swapchain = gs.device.create_swap_chain(&gs.surface, &gs.swapchain_descriptor);
                    }
                    Event::MainEventsCleared => self.window.request_redraw(),
                    Event::RedrawEventsCleared => screen.render(&self),
                    ev => screen.update(&self, ev), 
                }
            }
            else {
                *control_flow = ControlFlow::Exit;
            }
        });
    }
}

pub trait Screen {
    fn update(&mut self, engine: &Engine, event: Event<()>);
    fn render(&mut self, engine: &Engine);
}