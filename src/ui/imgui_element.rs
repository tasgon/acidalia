use iced_wgpu::wgpu;
pub use imgui::{self, *};

use crate::engine::{Element, Engine};
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
        platform.attach_window(
            gui.io_mut(),
            &engine.window,
            imgui_winit_support::HiDpiMode::Default,
        );

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

impl<F: Fn(&Ui)> Element for ImguiElement<F> {
    fn update(&mut self, engine: &mut Engine, event: &iced_winit::winit::event::Event<()>) {
        self.platform
            .handle_event(self.gui.io_mut(), &engine.window, event);
    }

    fn render<'a: 'rp, 'rp>(
        &'a mut self,
        engine: &mut Engine,
        frame: &wgpu::SwapChainFrame,
        rpass: &mut wgpu::RenderPass<'rp>,
    ) {
        let gs = &engine.graphics_state;
        self.platform
            .prepare_frame(self.gui.io_mut(), &engine.window)
            .expect("Failed to prepare frame");

        let ui = self.gui.frame();
        (self.func)(&ui);

        if self.last_cursor != Some(ui.mouse_cursor()) {
            self.last_cursor = Some(ui.mouse_cursor());
            self.platform.prepare_render(&ui, &engine.window);
        }

        self.renderer
            .render(ui.render(), &gs.queue, &gs.device, rpass)
            .unwrap();

        std::mem::drop(rpass);
    }
}
