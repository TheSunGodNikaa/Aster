use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

pub(crate) fn set_light_theme(ctx: &egui::Context) {
    ctx.set_zoom_factor(1.05);
    let mut fonts = egui::FontDefinitions::default();
    if let Some(windows_dir) = std::env::var_os("WINDIR") {
        let system_font = std::path::PathBuf::from(windows_dir.clone())
            .join("Fonts")
            .join("segoeui.ttf");
        if let Ok(bytes) = std::fs::read(system_font) {
            fonts.font_data.insert(
                "aster_segoe_ui".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "aster_segoe_ui".to_owned());
        }
        let brand_font = std::path::PathBuf::from(windows_dir)
            .join("Fonts")
            .join("segoeuib.ttf");
        if let Ok(bytes) = std::fs::read(brand_font) {
            fonts.font_data.insert(
                "aster_brand".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            fonts.families.insert(
                egui::FontFamily::Name("Aster Brand".into()),
                vec!["aster_brand".to_owned()],
            );
        }
    }
    ctx.set_fonts(fonts);

    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = Color32::from_rgb(255, 255, 255);
    visuals.window_fill = Color32::from_rgb(255, 255, 255);
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(248, 247, 251);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(249, 248, 251);
    visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(58, 53, 68);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(241, 237, 248);
    visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(66, 47, 99);
    visuals.override_text_color = Some(Color32::from_rgb(51, 47, 60));
    visuals.selection.bg_fill = Color32::from_rgb(108, 79, 163);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    ctx.set_visuals(visuals);
}

pub(crate) fn brand_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("Aster Brand".into()))
}

pub(crate) fn paint_cover(ui: &egui::Ui, texture: &egui::TextureHandle, bounds: Rect) {
    let image_size = texture.size_vec2();
    let viewport_ratio = bounds.width() / bounds.height();
    let image_ratio = image_size.x / image_size.y;
    let uv = if image_ratio > viewport_ratio {
        let visible_width = viewport_ratio / image_ratio;
        let margin = (1.0 - visible_width) / 2.0;
        Rect::from_min_max(Pos2::new(margin, 0.0), Pos2::new(1.0 - margin, 1.0))
    } else {
        let visible_height = image_ratio / viewport_ratio;
        let margin = (1.0 - visible_height) / 2.0;
        Rect::from_min_max(Pos2::new(0.0, margin), Pos2::new(1.0, 1.0 - margin))
    };
    ui.painter().image(texture.id(), bounds, uv, Color32::WHITE);
}
