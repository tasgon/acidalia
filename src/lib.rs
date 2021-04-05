// TODO: uncomment this
// #![forbid(missing_docs)]

#[macro_use]
mod engine;
mod engine_builder;
mod fps;
pub mod graphics;
/// Everything related to managing shaders.
pub mod shaders;
pub use shaderc::ShaderKind;

/// Everything related to drawing user interfaces.
pub mod ui;

pub use engine::*;
pub use engine_builder::EngineBuilder;
pub use graphics::GraphicsState;

pub use wgpu;
pub use winit;

pub use acidalia_core::*;
pub use acidalia_proc_macros::*;
