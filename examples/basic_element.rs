use acidalia::ui::{imgui, ImguiElement};
use acidalia::{screen, wgpu, Engine, EngineBuilder};
use imgui::im_str;
use winit::window::WindowBuilder;

#[derive(Default)]
struct Data {
    count: u32,
}

fn main() {
    let engine = EngineBuilder::new(|wb| wb.with_maximized(true))
        .bg_color(acidalia::wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        })
        .build();
    let ui_el = ImguiElement::new(
        |ui, _engine, d: &mut Data| {
            imgui::Window::new(im_str!("Main")).build(ui, || {
                if ui.small_button(im_str!("Increment count")) {
                    d.count += 1;
                }

                ui.text(im_str!("Count: {}", d.count));
            });
        },
        &engine,
    );

    let data = Data::default();
    // engine.run(, data);
}
