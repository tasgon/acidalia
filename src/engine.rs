use crate::winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
use crate::{shaders::ShaderState, wgpu};

use crate::{graphics::GraphicsState, EngineBuilder};

/// The core engine that constructs the window and graphics states, and passes events
/// to user-defined screens.
pub struct Engine {
    event_loop: Option<EventLoop<()>>,
    pub window: Window,
    pub graphics_state: GraphicsState,
    pub shader_state: ShaderState,
    pub background_color: wgpu::Color,
}

impl Engine {
    /// Constructs a new `Engine`. Currently, this does not let you set parameters, but that
    /// will be available in the future, likely through an `EngineBuilder`.
    pub fn new(eb: EngineBuilder) -> Self {
        let event_loop = EventLoop::new();
        let window = eb.wb.build(&event_loop).unwrap();
        let graphics_state = GraphicsState::new(&window);
        let mut shader_state = ShaderState::new(&graphics_state);
        shader_state.init_shaders();
        Self {
            event_loop: Some(event_loop),
            window,
            graphics_state,
            shader_state,
            background_color: eb.bg_color,
        }
    }

    /// Runs the event loop with an initial `Screen`.
    pub fn run<T: 'static>(mut self, screen: Screen<T>, mut data: T) {
        let evloop = self.event_loop.take().unwrap();
        let mut screens: Vec<Screen<T>> = vec![screen];
        evloop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            let last_sc = screens.last_mut();
            if let Some(screen) = last_sc {
                match event {
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        ..
                    } => {
                        *control_flow = ControlFlow::Exit
                    }
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
                                        load: wgpu::LoadOp::Clear(self.background_color),
                                        store: true,
                                    },
                                }],
                                depth_stencil_attachment: None,
                            });

                        for element in screen.iter_mut() {
                            element.render(&mut self, &mut data, &frame, &mut render_pass);
                        }
                        std::mem::drop(render_pass);

                        self.graphics_state.queue.submit(Some(encoder.finish()));

                        self.shader_state.cull();
                    }
                    _ => (),
                }
                for element in screen.iter_mut().rev() {
                    // TODO: allow event cancelling
                    element.update(&mut self, &mut data, &event)
                }
            } else {
                *control_flow = ControlFlow::Exit;
            }
        });
    }
}

/// Represents items that have update events and draw to the screen.
pub trait Element {
    type Data;

    /// Process `winit` events.
    fn update(&mut self, engine: &mut Engine, data: &mut Self::Data, event: &Event<()>);

    /// Draw to the screen. Note: it is expected that trait implementers will use
    /// the supplied render pass, however, to explain the lifetime annotations,
    /// the render pass is provided to all elements in the screen, so they all
    /// must live as long as the render pass.
    fn render<'a: 'rp, 'rp>(
        &'a mut self,
        engine: &mut Engine,
        data: &mut Self::Data,
        frame: &wgpu::SwapChainFrame,
        render_pass: &mut wgpu::RenderPass<'rp>,
    );
}

/// A list of `Elements` that will all update and draw on the screen.
/// The draw order is the element order.
pub type Screen<T> = Vec<Box<dyn Element<Data = T>>>;

/// Convenience macro to construct a `Screen` from a list of objects
/// that implement the `Element` trait.
#[macro_export]
macro_rules! screen {
    ($($el:expr),*) => {
        vec![$(Box::new($el), )*]
    };
}
