mod sprite;

pub use sprite::Sprite;

use crate::Engine;


pub trait TextureProvider {
    fn view(engine: &mut Engine) -> wgpu::TextureView;
    fn sampler(engine: &mut Engine) -> wgpu::Sampler;
}