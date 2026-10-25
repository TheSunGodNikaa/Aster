mod login;
mod workspace;
mod year_date;

pub(crate) use workspace::WorkspacePage;

pub(crate) mod company_settings;

pub(crate) mod product_master;

pub(crate) mod customer_master;

pub(crate) fn show_error(ctx: &egui::Context, error: &str) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(48.0);
            ui.heading("ASTER could not start");
            ui.add_space(8.0);
            ui.label(error);
            ui.add_space(8.0);
            ui.label("The existing database was not replaced. Check the log for details.");
        });
    });
}

pub(crate) fn paint_wallpaper(ui: &egui::Ui, app: &crate::app::AsterApp) {
    if let Some(background) = &app.background {
        crate::theme::paint_cover(ui, background, ui.max_rect());
    }
}

pub(crate) fn show_background_error(ui: &mut egui::Ui, app: &crate::app::AsterApp) {
    if let Some(error) = &app.background_error {
        ui.colored_label(
            egui::Color32::DARK_RED,
            format!("Background unavailable: {error}"),
        );
    }
}

pub(crate) mod supplier_master;

pub(crate) mod purchase_entry;
