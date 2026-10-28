mod app;
mod database;
mod date;
mod logging;
mod screens;
mod theme;
mod utils;

fn app_icon() -> eframe::egui::IconData {
    let image = image::load_from_memory(include_bytes!("../assets/Aster-icon.png"))
        .expect("Aster-icon.png is not a valid image")
        .to_rgba8();
    let (width, height) = image.dimensions();
    eframe::egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

fn main() -> eframe::Result<()> {
    logging::log_message("INFO", "Starting ASTER Billing");

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("ASTER · Select date and financial year")
            .with_inner_size([540.0, 360.0])
            .with_min_inner_size([500.0, 330.0])
            .with_icon(app_icon())
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "ASTER Billing",
        options,
        Box::new(|cc| Ok(Box::new(app::AsterApp::new(cc)))),
    )
}
