use acidalia::wgpu;
use core::marker::PhantomData;
pub use imgui::{self, *};

use acidalia::{Element, Engine};

/// Builds and renders an [`imgui::Ui`] constructed from a user-defined function.
pub struct ImguiElement<Data, F: Fn(&Ui, &Engine, &mut Data)> {
    func: F,
    // Note to self (since I forgot): this `PhantomData` is here because otherwise the type T
    // has nothing to attach to, so I have to create a 0-sized element here.
    _phantom: PhantomData<Data>,
    gui: imgui::Context,
    renderer: imgui_wgpu::Renderer,
    platform: imgui_winit_support::WinitPlatform,
    last_cursor: Option<Option<imgui::MouseCursor>>,
}

impl<Data, F: Fn(&Ui, &Engine, &mut Data)> ImguiElement<Data, F> {
    /// Construct a new `ImguiElement` from a function, which will take in a `Ui` struct and modify it
    /// as needed before drawing.
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
            _phantom: PhantomData::default(),
            gui,
            renderer,
            platform,
            last_cursor: None,
        }
    }
}

impl<Data, F: Fn(&Ui, &Engine, &mut Data)> Element<Data> for ImguiElement<Data, F> {
    fn update(
        &mut self,
        engine: &mut Engine,
        _data: &mut Data,
        event: &acidalia::winit::event::Event<()>,
    ) {
        self.platform
            .handle_event(self.gui.io_mut(), &engine.window, event);
    }

    fn render<'a: 'rp, 'rp>(
        &'a mut self,
        engine: &mut Engine,
        data: &mut Data,
        _frame: &wgpu::SurfaceTexture,
        render_pass: &mut wgpu::RenderPass<'rp>,
    ) {
        let gs = &engine.graphics_state;
        self.platform
            .prepare_frame(self.gui.io_mut(), &engine.window)
            .expect("Failed to prepare frame");

        let ui = self.gui.frame();
        (self.func)(&ui, engine, data);

        if self.last_cursor != Some(ui.mouse_cursor()) {
            self.last_cursor = Some(ui.mouse_cursor());
            self.platform.prepare_render(&ui, &engine.window);
        }

        self.renderer
            .render(ui.render(), &gs.queue, &gs.device, render_pass)
            .unwrap();

        std::mem::drop(render_pass);
    }
}
