use crate::graphics::GraphicsState;

pub struct RenderState {
    pub gfx_state: GraphicsState,
    pub color: (f64, f64),
}

impl RenderState {
    pub fn render(&mut self) {
        let gfx = &mut self.gfx_state;
        let frame = gfx.swapchain.get_next_texture().unwrap();
        let mut encoder = gfx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                load_op: wgpu::LoadOp::Clear,
                store_op: wgpu::StoreOp::Store,
                clear_color: wgpu::Color {
                    r: self.color.0,
                    g: self.color.1,
                    b: 0.0,
                    a: 1.0,
                },
            }],
            depth_stencil_attachment: None,
        });
        gfx.queue.submit(&[encoder.finish()]);
    }

    pub fn update(&mut self, event: &winit::event::WindowEvent) {
        let size = self.gfx_state.get_size();
        if let winit::event::WindowEvent::CursorMoved { position, .. } = event {
            self.color = (
                position.x / (size.width as f64),
                position.y / (size.height as f64),
            );
        }
    }
}
