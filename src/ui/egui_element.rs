pub use egui::{self, *};

use crate::engine::Engine;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

pub struct EguiElement {}

impl EguiElement {
    pub fn new(engine: &Engine) -> Self {
        unimplemented!()
    }
}
