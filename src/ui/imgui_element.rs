pub use imgui::{self, *};
use iced_wgpu::wgpu;

use crate::engine::{Engine, Screen};
pub struct ImguiElement<F: Fn(&Ui)> {
    func: F,
    gui: imgui::Context,
    renderer: imgui_wgpu::Renderer,
    platform: imgui_winit_support::WinitPlatform,
    last_cursor: Option<Option<imgui::MouseCursor>>,
}

impl<F: Fn(&Ui)> ImguiElement<F> {
    pub fn new(func: F, engine: &Engine) -> Self {
        let gs = &engine.graphics_state;
        let mut gui = imgui::Context::create();
        let mut platform = imgui_winit_support::WinitPlatform::init(&mut gui);
        platform.attach_window(gui.io_mut(), &engine.window, imgui_winit_support::HiDpiMode::Default);

        gui.set_ini_filename(None);

        let hidpi_factor = 1.0f32;
        let font_size = (13.0 * hidpi_factor) as f32;
        gui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        gui.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        let renderer_conf = imgui_wgpu::RendererConfig {
            texture_format: gs.swapchain_descriptor.format,
            ..Default::default()
        };
        let renderer = imgui_wgpu::Renderer::new(&mut gui, &gs.device, &gs.queue, renderer_conf);

        Self {
            func,
            gui,
            renderer,
            platform,
            last_cursor: None,
        }
    }
}

impl<F: Fn(&Ui)> Screen for ImguiElement<F> {
    fn update(&mut self, engine: &Engine, event: iced_winit::winit::event::Event<()>) {
        self.platform.handle_event(self.gui.io_mut(), &engine.window, &event);
    }

    fn render(&mut self, engine: &Engine) {
        let gs = &engine.graphics_state;

        let frame = match gs.swapchain.get_current_frame() {
            Ok(frame) => frame,
            Err(e) => {
                eprintln!("dropped frame: {:?}", e);
                return;
            }
        };
        self.platform
            .prepare_frame(self.gui.io_mut(), &engine.window)
            .expect("Failed to prepare frame");

        let ui = self.gui.frame();
        (self.func)(&ui);

        let mut encoder = gs.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        if self.last_cursor != Some(ui.mouse_cursor()) {
            self.last_cursor = Some(ui.mouse_cursor());
            self.platform.prepare_render(&ui, &engine.window);
        }

        // This'll probably move
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.output.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        self.renderer.render(ui.render(), &gs.queue, &gs.device, &mut rpass).unwrap();

        std::mem::drop(rpass);

        gs.queue.submit(Some(encoder.finish()));
    }
}