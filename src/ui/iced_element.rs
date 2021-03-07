use bytemuck::cast_slice;
use egui_wgpu_backend::wgpu::util::DeviceExt;
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{Debug, Program, Size, conversion, futures, program, winit::{self, dpi::PhysicalPosition, event::{Event, ModifiersState, WindowEvent}}};

use crate::engine::{Engine, Element};

const INDICES: &[u16] = &[0, 1, 2, 1, 2, 3];
const NUM_INDICES: u8 = 6;

pub struct IcedElement<T: Program<Renderer = Renderer> + 'static> {
    state: program::State<T>,
    viewport: Viewport,
    renderer: Renderer,
    debug: Debug,
    cursor_position: PhysicalPosition<f64>,
    modifiers: ModifiersState,
    staging_belt: wgpu::util::StagingBelt,
    // Because iced doesn't accept the previous render pass,
    // I have to have it draw to a texture, which I then add to the pass.
    dest_tex: wgpu::Texture,
    index_buf: wgpu::Buffer,
}

impl<T: Program<Renderer = Renderer>> IcedElement<T> {
    pub fn new(engine: &mut Engine, iced_program: T) -> Self {
        let gs = &mut engine.graphics_state;
        let mut debug = Debug::new();
        let mut viewport = Viewport::with_physical_size(
            Size::new(gs.get_size().width, gs.get_size().height),
            engine.window.scale_factor(),
        );
        let mut renderer = Renderer::new(Backend::new(&mut gs.device, Settings::default()));
        let mut cursor_position = PhysicalPosition::new(-1.0, -1.0);
        let mut modifiers = ModifiersState::default();
        let mut state = program::State::new(
            iced_program,
            viewport.logical_size(),
            conversion::cursor_position(cursor_position, viewport.scale_factor()),
            &mut renderer,
            &mut debug,
        );
        let dest_tex = gs.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("iced tex"),
            size: wgpu::Extent3d {
                width: gs.get_size().width,
                height: gs.get_size().height,
                depth: 1,
            },
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::RENDER_ATTACHMENT,
        });
        let index_buf = gs.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("iced index buf"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsage::INDEX
        });

        Self {
            state,
            viewport,
            renderer,
            debug,
            cursor_position,
            modifiers,
            staging_belt: wgpu::util::StagingBelt::new(5 * 1024),
            dest_tex,
            index_buf,
        }
    }
}

impl<T: Program<Renderer = Renderer>> Element for IcedElement<T> {
    fn update(&mut self, engine: &mut Engine, event: &winit::event::Event<()>) {
        match event {
            Event::WindowEvent { event: wev, .. } => {
                match wev {
                    WindowEvent::CursorMoved { position, ..} => self.cursor_position = position.clone(),
                    WindowEvent::ModifiersChanged(mods) => self.modifiers = mods.clone(),
                    WindowEvent::Resized(size) => {
                        self.viewport = Viewport::with_physical_size(
                            Size::new(size.width, size.height),
                            engine.window.scale_factor(),
                        );
                    }
                    _ => (),
                }

                if let Some(ev) = conversion::window_event(&wev, engine.window.scale_factor(), self.modifiers) {
                    self.state.queue_event(ev);
                }
            }
            Event::MainEventsCleared => {
                if !self.state.is_queue_empty() {
                    self.state.update(
                        self.viewport.logical_size(),
                        conversion::cursor_position(
                            self.cursor_position,
                            self.viewport.scale_factor(),
                        ),
                        None,
                        &mut self.renderer,
                        &mut self.debug,
                    );
                }
            }
            _ => (),
        }
    }

    fn render<'a: 'rp, 'rp>(&'a mut self, engine: &mut Engine, frame: &wgpu::SwapChainFrame, _render_pass: &mut wgpu::RenderPass<'rp>) {
        let gs = &mut engine.graphics_state;
        let mut encoder = gs.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let frame = self.dest_tex.create_view(&wgpu::TextureViewDescriptor::default());
        
        self.renderer.backend_mut().draw(
            &gs.device,
            &mut self.staging_belt,
            &mut encoder,
            &frame,
            &self.viewport,
            self.state.primitive(),
            &self.debug.overlay(),
        );

        self.staging_belt.finish();
        gs.queue.submit(Some(encoder.finish()));
    }
}
