use std::sync::Arc;

//use futures::executor::block_on;
use crate::wgpu::{self, BackendBit};
use crate::winit;
use futures;

use futures::executor::block_on;

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
        let instance = wgpu::Instance::new(BackendBit::VULKAN);
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
            present_mode: wgpu::PresentMode::Mailbox,
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
}
