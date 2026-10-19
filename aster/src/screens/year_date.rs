use eframe::egui::{self, Align, Color32, FontId, Rect, RichText, Vec2};

use crate::{
    app::{AsterApp, Stage},
    date::{local_date, parse_date},
};

impl AsterApp {
    pub(crate) fn show_year_and_date(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(248, 247, 251))
                    .inner_margin(0.0),
            )
            .show(ctx, |ui| {
                super::paint_wallpaper(ui, self);
                let card = Rect::from_center_size(ui.max_rect().center(), Vec2::new(474.0, 286.0));
                ui.painter().rect_filled(
                    card,
                    16.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 244),
                );
                ui.painter().rect_stroke(
                    card,
                    16.0,
                    egui::Stroke::new(1.0_f32, Color32::from_rgb(234, 231, 240)),
                    egui::StrokeKind::Inside,
                );

                let mut form = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(card.shrink2(Vec2::new(28.0, 20.0)))
                        .layout(egui::Layout::top_down(Align::Min)),
                );
                form.horizontal(|ui| {
                    ui.label(
                        RichText::new("ASTER")
                            .font(FontId::proportional(19.0))
                            .strong()
                            .color(Color32::from_rgb(75, 51, 117)),
                    );
                    ui.label(
                        RichText::new("BILLING")
                            .small()
                            .strong()
                            .color(Color32::from_rgb(132, 123, 147)),
                    );
                });
                form.add_space(12.0);
                form.label(
                    RichText::new("Choose date and financial year")
                        .font(FontId::proportional(22.0))
                        .strong()
                        .color(Color32::from_rgb(43, 38, 53)),
                );
                form.label(
                    RichText::new("Set the working period for this session.")
                        .color(Color32::from_rgb(116, 109, 128)),
                );
                form.add_space(18.0);

                form.horizontal(|ui| {
                    ui.label(RichText::new("Financial year").strong());
                    egui::ComboBox::from_id_salt("financial_year")
                        .width(174.0)
                        .selected_text(format!(
                            "{} – {}",
                            self.selected_year,
                            self.selected_year + 1
                        ))
                        .show_ui(ui, |ui| {
                            for year in &self.financial_years {
                                ui.selectable_value(
                                    &mut self.selected_year,
                                    *year,
                                    format!("{year} – {}", year + 1),
                                );
                            }
                        });
                    if ui
                        .small_button("+")
                        .on_hover_text("Add the next financial year")
                        .clicked()
                    {
                        if let Some(db) = &self.db {
                            let next_year = self
                                .financial_years
                                .iter()
                                .max()
                                .copied()
                                .unwrap_or(self.selected_year)
                                + 1;
                            match db.create_financial_year(next_year) {
                                Ok(()) => {
                                    self.financial_years.push(next_year);
                                    self.selected_year = next_year;
                                    self.message.clear();
                                }
                                Err(error) => self.message = error,
                            }
                        }
                    }
                });
                form.add_space(10.0);
                form.horizontal(|ui| {
                    ui.label(RichText::new("Working date").strong());
                    ui.add(
                        egui::TextEdit::singleline(&mut self.date_text)
                            .desired_width(112.0)
                            .hint_text("DD/MM/YYYY"),
                    );
                    if ui.small_button("Today").clicked() {
                        let (year, month, day) = local_date();
                        self.date_text = format!("{day:02}/{month:02}/{year}");
                    }
                });
                form.add_space(5.0);
                form.label(
                    RichText::new("Dates use the April 1 to March 31 financial-year boundary.")
                        .small()
                        .color(Color32::from_rgb(125, 118, 137)),
                );

                form.add_space(8.0);
                if !self.message.is_empty() {
                    form.colored_label(Color32::from_rgb(171, 54, 62), &self.message);
                }
                form.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let button = egui::Button::new(
                        RichText::new("Continue to sign in")
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(91, 65, 139))
                    .min_size(Vec2::new(158.0, 34.0));
                    if ui.add(button).clicked() {
                        match parse_date(&self.date_text) {
                            Some((year, month, _))
                                if (if month >= 4 { year } else { year - 1 })
                                    == self.selected_year =>
                            {
                                let has_users = self.db.as_ref().map(|db| db.has_users());
                                match has_users {
                                    Some(Ok(true)) => self.set_stage(Stage::Login, ctx),
                                    Some(Ok(false)) => self.set_stage(Stage::FirstRunSetup, ctx),
                                    Some(Err(error)) => self.set_stage(Stage::Error(error), ctx),
                                    None => self.set_stage(
                                        Stage::Error("Database is unavailable.".into()),
                                        ctx,
                                    ),
                                }
                                self.message.clear();
                            }
                            Some(_) => {
                                self.message =
                                    "Choose a date within the selected financial year.".into();
                            }
                            None => {
                                self.message = "Enter a valid date as DD/MM/YYYY.".into();
                            }
                        }
                    }
                });
                super::show_background_error(ui, self);
            });
    }
}
