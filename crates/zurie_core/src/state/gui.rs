use std::{fmt::Display, str::FromStr, sync::Arc};

use egui_winit_vulkano::{
    egui::{self, Context},
    Gui, GuiConfig,
};
use strum::IntoEnumIterator;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop};
use zurie_shared::sim_clock::SimClock;

use zurie_render::compute_sand::{BrushType, CellType, SandComputePipeline};

pub struct GameGui {
    pub context: Context,
}

impl GameGui {
    pub fn new(context: Context) -> Self {
        GameGui { context }
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
        let ctx = self.context.clone();
        let mut pointer_on_debug_window = false;
        egui::Window::new("Grid setup").show(&ctx, |ui| {
            ui.checkbox(simulate_ui_togle, "Simulate");
            let sim_speed_slider = ui.add(egui::Slider::new(cur_sim, 0..=100).text("Sim speed"));
            if sim_speed_slider.changed() {
                //*sim_rate = 0u16
            }
            if ui
                .add(
                    egui::Slider::new(&mut compute.scale_factor, 0..=100).text("Grid scale factor"),
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
