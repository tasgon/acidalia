use crate::graphics::GraphicsState;
use iced_wgpu::wgpu;
use iced_winit::winit;

#[inline(always)]
fn color(c: (f64, f64, f64, f64)) -> wgpu::Color {
    wgpu::Color {
        r: c.0,
        g: c.1,
        b: c.2,
        a: c.3,
    }
}

pub struct RenderState {
    pub gfx_state: GraphicsState,
    pub fps_counter: crate::fps::FPSCounter,
    pub background_color: (f64, f64, f64, f64),
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
                clear_color: color(self.background_color),
            }],
            depth_stencil_attachment: None,
        });
        gfx.queue.submit(&[encoder.finish()]);
        self.fps_counter.tick();
        self.fps_counter.print();
    }

    pub fn update(&mut self, event: &winit::event::WindowEvent) {
        let size = self.gfx_state.get_size();
        if let winit::event::WindowEvent::CursorMoved { position, .. } = event {
            //println!("Got event: ({}, {})", position.x, position.y);
            self.background_color = (
                position.x / (size.width as f64),
                position.y / (size.height as f64),
                0.0,
                1.0,
            );
        }
    }
}
