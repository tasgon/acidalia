use crate::winit::window::WindowBuilder;

use crate::Engine;


/// The tool that builds your engine for you.
#[derive(Default)]
pub struct EngineBuilder {
    pub(crate) wb: WindowBuilder,
    pub bg_color: crate::wgpu::Color,
}

impl EngineBuilder {
    /// Construct a new `EngineBuilder`, providing a function describing the window appearance.
    /// Refer to 
    pub fn new(mut window_fn: impl (FnMut(WindowBuilder) -> WindowBuilder)) -> Self {
        Self {
            wb: window_fn(WindowBuilder::new()),
            bg_color: Default::default(),
        }
    }

    pub fn bg_color(mut self, color: crate::wgpu::Color) -> Self {
        self.bg_color = color;
        self
    }

    pub fn build(self) -> Engine {
        Engine::new(self)
    }
}