use std::time::Instant;

use iced_wgpu::wgpu;
use iced_winit::winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{graphics::GraphicsState, shaders::{InternalShaderState, InternalShaders, ShaderState}};

pub struct Engine {
    event_loop: Option<EventLoop<()>>,
    pub window: Window,
    pub graphics_state: GraphicsState,
    pub shader_state: InternalShaderState,
    start: Instant,
    //screens: Vec<Box<dyn Screen>>,
}

impl Engine {
    pub fn new() -> Self {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop).unwrap();
        let mut graphics_state = GraphicsState::new(&window);
        let mut shader_state = InternalShaderState::new();
        shader_state.init_shaders(&mut graphics_state);
        Self {
            event_loop: Some(event_loop),
            window,
            graphics_state,
            shader_state,
            start: Instant::now(),
            //screens: vec![],
        }
    }

    pub fn run(mut self, screen: Screen) {
        let evloop = self.event_loop.take().unwrap();
        let mut screens: Vec<Screen> = vec![screen];
        self.start = Instant::now();
        evloop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            let last_sc = screens.last_mut();
            if let Some(screen) = last_sc {
                match event {
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    Event::WindowEvent {
                        event: WindowEvent::Resized(_),
                        ..
                    } => {
                        let size = self.window.inner_size();
                        self.graphics_state.set_size(size);
                    }
                    Event::MainEventsCleared => self.window.request_redraw(),
                    Event::RedrawEventsCleared => {
                        let frame = match self.graphics_state.swapchain.get_current_frame() {
                            Ok(frame) => frame,
                            Err(e) => {
                                eprintln!("dropped frame: {:?}", e);
                                return;
                            }
                        };

                        let mut encoder = self.graphics_state.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor { label: None },
                        );

                        let mut render_pass =
                            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: None,
                                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                                    attachment: &frame.output.view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color {
                                            r: 0.1,
                                            g: 0.2,
                                            b: 0.3,
                                            a: 0.0,
                                        }),
                                        store: true,
                                    },
                                }],
                                depth_stencil_attachment: None,
                            });

                        for element in screen.iter_mut() {
                            element.render(&mut self, &frame, &mut render_pass);
                        }
                        std::mem::drop(render_pass);

                        self.graphics_state.queue.submit(Some(encoder.finish()));
                    }
                    _ => ()
                }
                for element in screen.iter_mut().rev() {
                    // TODO: allow event cancelling
                    element.update(&mut self, &event)
                }
            } else {
                *control_flow = ControlFlow::Exit;
            }
        });
    }

    pub fn start(&self) -> Instant { self.start }
}

pub trait Element {
    fn update(&mut self, engine: &mut Engine, event: &Event<()>);
    fn render<'a: 'rp, 'rp>(
        &'a mut self,
        engine: &mut Engine,
        frame: &wgpu::SwapChainFrame,
        render_pass: &mut wgpu::RenderPass<'rp>,
    ) {
    }
}

type Screen = Vec<Box<dyn Element>>;

#[macro_export]
macro_rules! screen {
    ($($el:expr),*) => {
        vec![$(Box::new($el), )*]
    };
}
