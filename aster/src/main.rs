use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "ASTER",
        options,
        Box::new(|_cc| {
            Ok(Box::new(AsterApp))
        }),
    )
}

struct AsterApp;

impl eframe::App for AsterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("ASTER Billing ERP");
            ui.label("Development environment verified.");
        });
    }
}