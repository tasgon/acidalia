mod sprite;
pub use sprite::Sprite;

mod canvas;
pub use canvas::Canvas;

use acidalia::{wgpu, Engine};

pub trait TextureProvider {
    fn view(engine: &mut Engine) -> wgpu::TextureView;
    fn sampler(engine: &mut Engine) -> wgpu::Sampler;
}
