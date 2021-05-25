use std::{num::NonZeroU32, sync::Arc};

//use futures::executor::block_on;
use crate::wgpu::{self, BackendBit};
use crate::winit;
use futures;

use futures::executor::block_on;
use wgpu::{
    BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingResource, BindingType,
    CommandEncoder, CommandEncoderDescriptor, Device, PipelineLayout,
    PipelineLayoutDescriptor, PushConstantRange, ShaderStage,
};
use winit::dpi::PhysicalSize;

/// A struct containing everything necessary to interact with wgpu.
pub struct GraphicsState {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: Arc<wgpu::Device>,
    pub queue: wgpu::Queue,
    pub swapchain_descriptor: wgpu::SwapChainDescriptor,
    pub swapchain: wgpu::SwapChain,

    size: winit::dpi::PhysicalSize<u32>,
}

impl GraphicsState {
    /// Creates a new `GraphicsState` from a `winit` window.
    pub(crate) fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };

        let adapter_options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
        };

        let adapter = block_on(async { instance.request_adapter(&adapter_options).await.unwrap() });

        let (device, queue) = block_on(async {
            adapter
                .request_device(&wgpu::DeviceDescriptor::default(), None)
                .await
                .unwrap()
        });
        let device = Arc::new(device);

        let swapchain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Immediate,
        };
        let swapchain = device.create_swap_chain(&surface, &swapchain_descriptor);

        Self {
            instance,
            surface,
            adapter,
            device,
            queue,
            swapchain_descriptor,
            swapchain,
            size,
        }
    }

    /// Sets the size and updates the swapchain & descriptor.
    pub(crate) fn set_size(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.size = size;
        self.swapchain_descriptor.width = size.width;
        self.swapchain_descriptor.height = size.height;
        self.swapchain = self
            .device
            .create_swap_chain(&self.surface, &self.swapchain_descriptor);
    }

    /// Gets the swapchain's frame size.
    pub fn get_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    /// Quickly create a [`wgpu::CommandEncoder`] given a `label`.
    pub fn command_encoder<'a>(&self, label: impl Into<Option<&'a str>>) -> CommandEncoder {
        self.device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: label.into(),
            })
    }

    /// Start creating a bind group layout with a given `label`.
    pub fn bind_group_layout<'a, 'b: 'a>(
        &'b self,
        label: impl Into<Option<&'a str>>,
    ) -> BindGroupLayoutConstructor<'a, 'b> {
        BindGroupLayoutConstructor {
            device: &self.device,
            label: label.into(),
            entries: vec![],
        }
    }

    /// Start creating a bind group with a given `label` and bind group `layout`.
    pub fn bind_group<'a, 'b: 'a>(
        &'b self,
        label: impl Into<Option<&'a str>>,
        layout: &'a BindGroupLayout,
    ) -> BindGroupConstructor<'a, 'b> {
        BindGroupConstructor {
            device: &self.device,
            label: label.into(),
            layout,
            entries: vec![],
        }
    }

    /// Create a pipeline layout.
    pub fn pipeline_layout<'a>(
        &self,
        label: impl Into<Option<&'a str>>,
        bind_group_layouts: &'a [&'a BindGroupLayout],
        push_constant_ranges: &[PushConstantRange],
    ) -> PipelineLayout {
        self.device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: label.into(),
                bind_group_layouts,
                push_constant_ranges,
            })
    }
}

/// Convenience trait to convert something into an [`wgpu::Extent3d`].
pub trait ToExtent {
    /// Do the conversion.
    fn to_extent(self, depth: u32) -> wgpu::Extent3d;
}

impl ToExtent for PhysicalSize<u32> {
    fn to_extent(self, depth_or_array_layers: u32) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers,
        }
    }
}

/// Constructor type for a [`wgpu::BindGroupLayout`].
pub struct BindGroupLayoutConstructor<'a, 'b: 'a> {
    device: &'b Device,
    label: Option<&'a str>,
    entries: Vec<BindGroupLayoutEntry>,
}

impl<'a, 'b: 'a> BindGroupLayoutConstructor<'a, 'b> {
    /// Add a [`wgpu::BindGroupEntry`] with the associated `binding`, `count`, shader stage `visibility` and `binding_type.
    pub fn entry(
        mut self,
        binding: u32,
        count: impl Into<Option<NonZeroU32>>,
        visibility: ShaderStage,
        binding_type: BindingType,
    ) -> Self {
        self.entries.push(BindGroupLayoutEntry {
            binding,
            count: count.into(),
            visibility,
            ty: binding_type,
        });
        self
    }

    /// Convenience function to add an entry, calculating the binding from
    /// the index of the new item.
    pub fn add(
        mut self,
        count: impl Into<Option<NonZeroU32>>,
        visibility: ShaderStage,
        binding_type: BindingType,
    ) -> Self {
        let binding = self.entries.len() as u32;
        self.entries.push(BindGroupLayoutEntry {
            binding,
            count: count.into(),
            visibility,
            ty: binding_type,
        });
        self
    }

    /// Build this into a [`wgpu::BindGroupLayout`].
    pub fn build(self) -> BindGroupLayout {
        self.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: self.label,
                entries: self.entries.as_slice(),
            })
    }
}

/// Constructor type for a [`wgpu::BindGroup`].
pub struct BindGroupConstructor<'a, 'b: 'a> {
    device: &'b Device,
    label: Option<&'a str>,
    layout: &'a BindGroupLayout,
    entries: Vec<BindGroupEntry<'a>>,
}

impl<'a, 'b: 'a> BindGroupConstructor<'a, 'b> {
    /// Add a [`wgpu::BindGroupEntry`] with the associated `binding` and `resource`.
    pub fn entry(mut self, binding: u32, resource: impl AsBindingResource<'a>) -> Self {
        self.entries.push(BindGroupEntry {
            binding,
            resource: resource.as_binding_resource(),
        });
        self
    }

    /// Convenience function to add an entry, calculating the binding from
    /// the index of the new item.
    pub fn add(mut self, resource: impl AsBindingResource<'a>) -> Self {
        let binding = self.entries.len() as u32;
        self.entries.push(BindGroupEntry {
            binding,
            resource: resource.as_binding_resource(),
        });
        self
    }

    /// Build this into a [`wgpu::BindGroup`].
    pub fn build(self) -> BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.label,
            layout: self.layout,
            entries: self.entries.as_slice(),
        })
    }
}

/// Convenience trait for converting things to a [`wgpu::BindingResource`].
pub trait AsBindingResource<'a> {
    /// Do the conversion.
    fn as_binding_resource(self) -> BindingResource<'a>;
}

impl<'a> AsBindingResource<'a> for wgpu::BindingResource<'a> {
    fn as_binding_resource(self) -> BindingResource<'a> {
        self
    }
}

impl<'a> AsBindingResource<'a> for &'a wgpu::Sampler {
    fn as_binding_resource(self) -> BindingResource<'a> {
        wgpu::BindingResource::Sampler(self)
    }
}

impl<'a> AsBindingResource<'a> for &'a wgpu::TextureView {
    fn as_binding_resource(self) -> BindingResource<'a> {
        wgpu::BindingResource::TextureView(self)
    }
}

impl<'a> AsBindingResource<'a> for &'a [&'a wgpu::TextureView] {
    fn as_binding_resource(self) -> BindingResource<'a> {
        wgpu::BindingResource::TextureViewArray(self)
    }
}
