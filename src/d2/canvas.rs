use crate::Element;

pub struct Canvas {

}

impl Canvas {
    pub fn new() -> Self {
        Self {

        }
    }
}

impl<Data> Element<Data> for Canvas {
    fn update(&mut self, engine: &mut crate::Engine, data: &mut Data, event: &winit::event::Event<()>) {
        todo!()
    }

    fn render<'a: 'rp, 'rp>(
        &'a mut self,
        engine: &mut crate::Engine,
        data: &mut Data,
        frame: &wgpu::SurfaceTexture,
        render_pass: &mut wgpu::RenderPass<'rp>,
    ) {
        todo!()
    }
}

pub trait CanvasElement<Data> {
    fn update(&mut self, data: &mut Data);

    fn draw(&mut self);
}
