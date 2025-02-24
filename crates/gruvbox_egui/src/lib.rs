use egui::Color32;

struct GruvboxColors {
    bg: egui::Color32,
    fg: egui::Color32,
    red: egui::Color32,
    green: egui::Color32,
    yellow: egui::Color32,
    blue: egui::Color32,
    purple: egui::Color32,
    aqua: egui::Color32,
    gray: egui::Color32,
    orange: egui::Color32,
    shadow: egui::Color32,
}

fn gruvbox_dark_colors() -> GruvboxColors {
    GruvboxColors {
        bg: egui::Color32::from_rgb(40, 40, 40), // Background (#282828)
        fg: egui::Color32::from_rgb(235, 219, 178), // Foreground (#ebdbb2)
        red: egui::Color32::from_rgb(204, 36, 29), // Red (#cc241d)
        green: egui::Color32::from_rgb(152, 151, 26), // Green (#98971a)
        yellow: egui::Color32::from_rgb(215, 153, 33), // Yellow (#d79921)
        blue: egui::Color32::from_rgb(69, 133, 136), // Blue (#458588)
        purple: egui::Color32::from_rgb(177, 98, 134), // Purple (#b16286)
        aqua: egui::Color32::from_rgb(104, 157, 106), // Aqua (#689d6a)
        gray: egui::Color32::from_rgb(146, 131, 116), // Gray (#928374)
        orange: egui::Color32::from_rgb(214, 93, 14), // Orange (#d65d0e)
        shadow: egui::Color32::from_rgba_premultiplied(0, 0, 0, 100), // Dark Gray Shadow (#1d2021)
    }
}

pub fn gruvbox_dark_theme() -> egui::Style {
    let colors = gruvbox_dark_colors();
    let mut style = egui::Style::default();
    let visuals = &mut style.visuals;

    // Background and foreground
    visuals.window_fill = colors.bg;
    visuals.panel_fill = colors.bg;
    visuals.override_text_color = Some(colors.fg);

    // Button Colors
    visuals.widgets.noninteractive.bg_fill = colors.bg;
    visuals.widgets.noninteractive.fg_stroke.color = colors.fg;

    visuals.widgets.active.bg_fill = colors.blue;
    visuals.widgets.active.fg_stroke.color = colors.fg;

    visuals.widgets.hovered.bg_fill = colors.gray;
    visuals.widgets.hovered.fg_stroke.color = colors.fg;

    visuals.widgets.inactive.bg_fill = colors.bg;
    visuals.widgets.inactive.fg_stroke.color = colors.fg;

    visuals.widgets.open.bg_fill = colors.orange;
    visuals.widgets.open.fg_stroke.color = colors.fg;

    // Selection color
    visuals.selection.bg_fill = colors.blue;
    visuals.selection.stroke.color = colors.fg;

    // Window shadow color (Updated)
    visuals.window_shadow.color = colors.shadow;

    style
}
