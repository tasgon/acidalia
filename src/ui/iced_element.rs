use std::{marker::PhantomData, sync::Arc};

use futures::task::SpawnExt;
use iced_wgpu::{
    wgpu::{self, util::DeviceExt},
    Backend, Renderer, Settings, Viewport,
};
use iced_winit::{
    conversion, futures,
    image::Data,
    program,
    winit::{
        self,
        dpi::PhysicalPosition,
        event::{Event, ModifiersState, WindowEvent},
    },
    Clipboard, Debug, Program, Size,
};

use crate::{
    engine::{Element, Engine},
    graphics::ToExtent,
    shaders::{InternalShaders, RenderTags},
};

const INDICES: &[u16] = &[0, 2, 1, 1, 2, 3];
const NUM_INDICES: u32 = 6;

/// Renders and handles events for objects implementing [`Program`].
/// As all elements share a common data struct, a function must also be provided
/// which will send messages to the state and read data from the state into the common struct.
/// TODO: deal with the fact that iced doesnt wanna reuse a render pass. maybe have a separate ui pass?
#[allow(dead_code)]
pub struct IcedElement<
    D,
    T: Program<Renderer = Renderer, Clipboard = Clipboard> + 'static,
    F: FnMut(&mut program::State<T>, &mut D),
> {
    state: program::State<T>,
    func: F,
    _phantom: PhantomData<D>,
    viewport: Viewport,
    renderer: Renderer,
    clipboard: Clipboard,
    debug: Debug,
    cursor_position: PhysicalPosition<f64>,
    modifiers: ModifiersState,
    staging_belt: wgpu::util::StagingBelt,
    pool: futures::executor::LocalPool,
    // Because iced doesn't accept the previous render pass,
    // I have to have it draw to a texture, which I then add to the pass.
    dest_tex: wgpu::Texture,
    dest_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    index_buf: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    pipeline: Arc<wgpu::RenderPipeline>,
}

impl<
        D,
        T: Program<Renderer = Renderer, Clipboard = Clipboard>,
        F: FnMut(&mut program::State<T>, &mut D),
    > IcedElement<D, T, F>
{
    /// Construct an `IcedElement` given a program object and a processing function.
    pub fn new(engine: &mut Engine, iced_program: T, func: F) -> Self {
        let gs = &mut engine.graphics_state;
        let mut debug = Debug::new();
        let viewport = Viewport::with_physical_size(
            Size::new(gs.get_size().width, gs.get_size().height),
            engine.window.scale_factor(),
        );
        let mut renderer = Renderer::new(Backend::new(&mut gs.device, Settings::default()));
        let mut clipboard = Clipboard::connect(&engine.window);
        let cursor_position = PhysicalPosition::new(-1.0, -1.0);
        let modifiers = ModifiersState::default();
        let state = program::State::new(
            iced_program,
            viewport.logical_size(),
            conversion::cursor_position(cursor_position, viewport.scale_factor()),
            &mut renderer,
            &mut debug,
        );
        let dest_tex = gs.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("iced tex"),
            size: gs.get_size().to_extent(1),
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::RENDER_ATTACHMENT,
        });
        let dest_view = dest_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = gs.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let index_buf = gs
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("iced index buf"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsage::INDEX,
            });
        let bind_group_layout = gs
            .bind_group_layout("iced bgl")
            .add(
                None,
                wgpu::ShaderStage::FRAGMENT,
                wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                },
            )
            .add(
                None,
                wgpu::ShaderStage::FRAGMENT,
                wgpu::BindingType::Sampler {
                    comparison: false,
                    filtering: true,
                },
            )
            .build();
        let bind_group = gs
            .bind_group("iced bg", &bind_group_layout)
            .add(&dest_view)
            .add(&sampler)
            .build();
        let pipeline_layout =
            gs.pipeline_layout("iced pipeline layout", &[&bind_group_layout], &[]);
        let format = gs.swapchain_descriptor.format;
        let pipeline = engine
            .shader_state
            .render_pipeline_builder("iced pipeline", pipeline_layout, InternalShaders::IcedVert)
            .fragment(
                InternalShaders::IcedFrag,
                wgpu::ColorTargetState {
                    format,
                    alpha_blend: wgpu::BlendState::REPLACE,
                    color_blend: wgpu::BlendState::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                },
            )
            .primitive(wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                polygon_mode: wgpu::PolygonMode::Fill,
            })
            .depth_stencil(None)
            .multisample(1, !0, false)
            .build();

        Self {
            state,
            func,
            _phantom: PhantomData::default(),
            viewport,
            renderer,
            clipboard,
            debug,
            cursor_position,
            modifiers,
            staging_belt: wgpu::util::StagingBelt::new(5 * 1024),
            pool: futures::executor::LocalPool::new(),
            dest_tex,
            dest_view,
            sampler,
            index_buf,
            bind_group_layout,
            bind_group,
            pipeline,
        }
    }
}

impl<
        D,
        T: Program<Renderer = Renderer, Clipboard = Clipboard>,
        F: FnMut(&mut program::State<T>, &mut D),
    > Element for IcedElement<D, T, F>
{
    type Data = D;

    fn update(
        &mut self,
        engine: &mut Engine,
        data: &mut Self::Data,
        event: &winit::event::Event<()>,
    ) {
        match event {
            Event::WindowEvent { event: wev, .. } => {
                match wev {
                    WindowEvent::CursorMoved { position, .. } => {
                        self.cursor_position = position.clone();
                    }
                    WindowEvent::ModifiersChanged(mods) => self.modifiers = mods.clone(),
                    WindowEvent::Resized(size) => {
                        self.viewport = Viewport::with_physical_size(
                            Size::new(size.width, size.height),
                            engine.window.scale_factor(),
                        );

                        // This is just copied from `new`.
                        // TODO: maybe look at making dest_* and bind_group MaybeUninit and
                        //       move this code to its own function?
                        let gs = &engine.graphics_state;
                        self.dest_tex = gs.device.create_texture(&wgpu::TextureDescriptor {
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
                            usage: wgpu::TextureUsage::SAMPLED
                                | wgpu::TextureUsage::RENDER_ATTACHMENT,
                        });
                        self.dest_view = self
                            .dest_tex
                            .create_view(&wgpu::TextureViewDescriptor::default());
                        self.bind_group = gs
                            .bind_group("iced bg", &self.bind_group_layout)
                            .add(&self.dest_view)
                            .add(&self.sampler)
                            .build();
                    }
                    _ => (),
                }

                if let Some(ev) =
                    conversion::window_event(wev, engine.window.scale_factor(), self.modifiers)
                {
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
                        &mut self.renderer,
                        &mut self.clipboard,
                        &mut self.debug,
                    );

                    engine.window.request_redraw();
                }
            }
            _ => (),
        }
        (self.func)(&mut self.state, data);
    }

    fn render<'a: 'rp, 'rp>(
        &'a mut self,
        engine: &mut Engine,
        data: &mut Self::Data,
        _frame: &wgpu::SwapChainFrame,
        render_pass: &mut wgpu::RenderPass<'rp>,
    ) {
        let gs = &mut engine.graphics_state;
        let mut encoder = gs.command_encoder("iced encoder");

        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &self.dest_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        self.renderer.backend_mut().draw(
            &gs.device,
            &mut self.staging_belt,
            &mut encoder,
            &self.dest_view,
            &self.viewport,
            self.state.primitive(),
            &self.debug.overlay(),
        );

        self.staging_belt.finish();
        gs.queue.submit(Some(encoder.finish()));
        self.pool
            .spawner()
            .spawn(self.staging_belt.recall())
            .unwrap();
        self.pool.run_until_stalled();

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..NUM_INDICES, 0, 0..1);
    }
}
