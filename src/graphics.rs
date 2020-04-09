use futures::executor::block_on;

pub struct GraphicsState {
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub swapchain_descriptor: wgpu::SwapChainDescriptor,
    pub swapchain: wgpu::SwapChain,

    size: winit::dpi::PhysicalSize<u32>,
}

impl GraphicsState {
    pub fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();
        let surface = wgpu::Surface::create(window);

        let adapter_options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
            compatible_surface: Some(&surface),
        };

        let adapter = block_on(async {
            wgpu::Adapter::request(&adapter_options, wgpu::BackendBit::VULKAN)
                .await
                .unwrap()
        });

        let (device, queue) = block_on(async {
            adapter
                .request_device(&wgpu::DeviceDescriptor {
                    extensions: wgpu::Extensions {
                        anisotropic_filtering: false,
                    },
                    limits: Default::default(),
                })
                .await
        });

        let swapchain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        let swapchain = device.create_swap_chain(&surface, &swapchain_descriptor);

        Self {
            surface,
            adapter,
            device,
            queue,
            swapchain_descriptor,
            swapchain,
            size,
        }
    }

    pub fn set_size(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.size = size;
        self.swapchain_descriptor.width = size.width;
        self.swapchain_descriptor.height = size.height;
        self.swapchain = self
            .device
            .create_swap_chain(&self.surface, &self.swapchain_descriptor);
    }

    pub fn get_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }
}
