use anyhow::Ok;
use egui_winit_vulkano::egui::{self, Context};
use wasmtime::{Caller, Linker, Store};
use zurie_types::GuiTextMessage;

use crate::utils::get_obj_by_ptr;

pub fn register_gui_button(
    linker: &mut Linker<()>,
    store: &Store<()>,
    gui_context: Context,
) -> anyhow::Result<()> {
    linker.func_new(
        "env",
        "gui_button_sys",
        wasmtime::FuncType::new(
            store.engine(),
            [wasmtime::ValType::I32, wasmtime::ValType::I32]
                .iter()
                .cloned(),
            [wasmtime::ValType::I32].iter().cloned(),
        ),
        move |mut caller, params, results| {
            let obj = get_obj_by_ptr::<GuiTextMessage>(
                &mut caller,
                params[0].unwrap_i32() as u32,
                params[1].unwrap_i32() as u32,
            )?;
            let mut clicked = 0;
            let window = egui::Window::new(obj.window_title);
            window.show(&gui_context, |ui| {
                clicked = ui.button(obj.label_text).clicked() as i32;
            });
            results[0] = wasmtime::Val::I32(clicked);
            Ok(())
        },
    )?;
    Ok(())
}
pub fn register_gui_text(linker: &mut Linker<()>, gui_context: Context) -> anyhow::Result<()> {
    linker.func_wrap(
        "env",
        "gui_text_sys",
        move |mut caller: Caller<'_, ()>, ptr: u32, len: u32| {
            let obj = get_obj_by_ptr::<GuiTextMessage>(&mut caller, ptr, len).unwrap();
            let window = egui::Window::new(obj.window_title);
            window.show(&gui_context, |ui| ui.label(obj.label_text));
        },
    )?;
    Ok(())
}
