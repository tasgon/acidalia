use std::{num::NonZeroU32, path::Path};

use image::ImageError;
use wgpu::{AddressMode, Extent3d, FilterMode, ImageCopyTexture, Origin3d, Sampler, Texture, TextureDescriptor, TextureUsages, TextureView, TextureViewDescriptor};

use crate::{Engine, GraphicsState};

pub struct Sprite {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
    pub size: Extent3d,
}

impl Sprite {
    /// Create a `Sprite` object from a file.
    pub fn from_file(
        gs: impl AsRef<GraphicsState>,
        p: impl AsRef<Path>,
        custom_sampler: Option<&wgpu::SamplerDescriptor>,
    ) -> Result<Self, ImageError> {
        let gs = gs.as_ref();
        let path = p.as_ref();
        let img = image::io::Reader::open(path)?.decode()?;
        let data = img.as_rgba8().unwrap();
        let size = Extent3d {
            width: data.dimensions().0,
            height: data.dimensions().1,
            depth_or_array_layers: 1,
        };
        let texture = gs.device.create_texture(&TextureDescriptor {
            label: Some(path.to_string_lossy().as_ref()),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        });
        gs.queue.write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: wgpu::TextureAspect::default(),
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(4 * size.width),
                rows_per_image: NonZeroU32::new(size.height),
            },
            size,
        );
        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = gs.device.create_sampler(custom_sampler.unwrap_or(&wgpu::SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            ..Default::default()
        }));
        Ok(Self {
            texture,
            view,
            sampler,
            size
        })
    }

    /// Draw the sprite to a screen.
    pub fn draw(engine: &mut Engine) {
        
    }
}
