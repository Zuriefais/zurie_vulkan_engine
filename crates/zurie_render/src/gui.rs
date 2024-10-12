use std::{fmt::Display, str::FromStr, sync::Arc};

use egui_winit_vulkano::{egui, Gui, GuiConfig};
use strum::IntoEnumIterator;
use vulkano::{
    device::Queue, format::Format, image::view::ImageView, swapchain::Surface, sync::GpuFuture,
};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop};
use zurie_shared::sim_clock::SimClock;

use crate::compute_sand::{BrushType, CellType, SandComputePipeline};

pub struct GameGui {
    pub gui: Gui,
}

impl GameGui {
    pub fn new(
        event_loop: &ActiveEventLoop,
        surface: Arc<Surface>,
        gfx_queue: Arc<Queue>,
        output_format: Format,
    ) -> Self {
        let gui = Gui::new(
            event_loop,
            surface,
            gfx_queue,
            output_format,
            GuiConfig {
                allow_srgb_render_target: true,
                is_overlay: true,
                samples: vulkano::image::SampleCount::Sample1,
            },
        );
        GameGui { gui }
    }

    pub fn event(&mut self, event: &WindowEvent) {
        self.gui.update(event);
    }

    pub fn draw_gui(
        &mut self,
        sim_clock: &mut SimClock,
        compute: &mut SandComputePipeline,
        is_hovered: &mut bool,
        selected_cell_type: &mut CellType,
        size: [u32; 2],
        background_color: &mut [f32; 4],
    ) {
        let (simulate_ui_togle, cur_sim, &mut sim_rate) = sim_clock.ui_togles();
        self.gui.immediate_ui(|gui| {
            let ctx = gui.context();
            let mut pointer_on_debug_window = false;
            egui::Window::new("Grid setup").show(&ctx, |ui| {
                ui.checkbox(simulate_ui_togle, "Simulate");
                let sim_speed_slider =
                    ui.add(egui::Slider::new(cur_sim, 0..=100).text("Sim speed"));
                if sim_speed_slider.changed() {
                    //*sim_rate = 0u16
                }
                if ui
                    .add(
                        egui::Slider::new(&mut compute.scale_factor, 0..=100)
                            .text("Grid scale factor"),
                    )
                    .changed()
                {
                    compute.resize(size)
                }
                if ui.button("New Random Grid").clicked() {
                    compute.new_rand_grid()
                }
                ui.label(format!("sim_rate: {}", sim_rate));

                pointer_on_debug_window = ui.ui_contains_pointer();
            });
            let mut pointer_on_selector_window = false;
            egui::Window::new("Cell Type selector").show(&ctx, |ui| {
                for (i, cell_type) in CellType::iter().enumerate() {
                    if i != 0 {
                        ui.radio_value(selected_cell_type, cell_type, cell_type.to_string());
                    }
                }
                pointer_on_selector_window = ui.ui_contains_pointer();
            });
            let mut pointer_on_color_window = false;
            egui::Window::new("Palette editor").show(&ctx, |ui| {
                ui.label("Background color:");
                ui.color_edit_button_rgba_premultiplied(background_color);
                ui.label("Cells pallete:");
                for color in compute.pallete.iter_mut() {
                    ui.color_edit_button_rgba_premultiplied(color);
                }
                pointer_on_color_window = ui.ui_contains_pointer();
            });

            let mut pointer_on_brush_window = false;
            egui::Window::new("Brush editor").show(&ctx, |ui| {
                ui.add(egui::Slider::new(&mut compute.brush_size, 0..=100).text("Brush size"));
                for brush_type in BrushType::iter() {
                    ui.radio_value(
                        &mut compute.selected_brush,
                        brush_type,
                        brush_type.to_string(),
                    );
                }
                pointer_on_brush_window = ui.ui_contains_pointer();
            });

            *is_hovered = pointer_on_debug_window
                || pointer_on_selector_window
                || pointer_on_color_window
                || pointer_on_brush_window;
        });
    }

    pub fn draw_on_image<F>(
        &mut self,
        before_future: F,
        final_image: Arc<ImageView>,
    ) -> Box<dyn GpuFuture>
    where
        F: GpuFuture + 'static,
    {
        self.gui.draw_on_image(before_future, final_image)
    }
}

pub fn integer_edit_field<T>(ui: &mut egui::Ui, value: &mut T) -> egui::Response
where
    T: Display,
    T: FromStr,
{
    let mut tmp_value = format!("{}", value);
    let res = ui.text_edit_singleline(&mut tmp_value);
    if let Ok(result) = tmp_value.parse() {
        *value = result;
    }
    res
}
