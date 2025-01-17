use egui::Id;

use super::zurie::engine::gui::{Widget, WidgetResponse};
use super::ScriptingState;
use crate::functions::zurie::engine::gui;

impl gui::Host for ScriptingState {
    #[doc = " window handle only valid for one frame"]
    fn create_window(&mut self, title: String, widgets: Vec<Widget>) -> Vec<WidgetResponse> {
        let mut responses = vec![];
        egui::Window::new(title).show(&self.gui_context, |ui| {
            for widget in widgets {
                responses.push(match widget {
                    Widget::Label(text) => WidgetResponse::Clicked(ui.label(text).clicked()),
                    Widget::Button(text) => WidgetResponse::Clicked(ui.button(text).clicked()),
                    Widget::Input(mut text) => {
                        ui.add(egui::TextEdit::singleline(&mut text));
                        WidgetResponse::Input(text)
                    }
                    Widget::Checkbox((mut checked, text)) => {
                        ui.checkbox(&mut checked, text);
                        WidgetResponse::Checked(checked)
                    }
                })
            }
        });
        responses
    }
}
