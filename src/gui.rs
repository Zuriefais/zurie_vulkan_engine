use std::sync::Arc;

use egui_winit_vulkano::{egui, Gui, GuiConfig};
use log::info;
use strum::IntoEnumIterator;
use vulkano::{
    device::Queue, format::Format, image::view::ImageView, swapchain::Surface, sync::GpuFuture,
};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop};

use crate::{
    compute_sand::{CellType, SandComputePipeline},
    state::SimClock,
};

pub struct GameGui {
    gui: Gui,
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
    ) {
        let (simulate_ui_togle, cur_sim, sim_rate) = sim_clock.ui_togles();
        self.gui.immediate_ui(|gui| {
            let ctx = gui.context();
            let mut pointer_on_debug_window = false;
            egui::Window::new("Debug window").show(&ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(egui::widgets::Label::new("Hi there!"));
                    if ui.button("Click me else you die").clicked() {
                        info!("it's joke")
                    }
                    ui.checkbox(simulate_ui_togle, "Simulate");
                    integer_edit_field(ui, cur_sim);
                    if ui.button("New Random Grid").clicked() {
                        compute.new_rand_grid()
                    }
                    ui.label(format!("sim_rate: {}", sim_rate));
                });
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

            if pointer_on_debug_window || pointer_on_selector_window {
                *is_hovered = true
            } else {
                *is_hovered = false
            }
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

fn integer_edit_field(ui: &mut egui::Ui, value: &mut u16) -> egui::Response {
    let mut tmp_value = format!("{}", value);
    let res = ui.text_edit_singleline(&mut tmp_value);
    if let Ok(result) = tmp_value.parse() {
        *value = result;
    }
    res
}
