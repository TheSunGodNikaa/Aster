use eframe::egui::{self, Align, Color32, FontId, Rect, RichText, Vec2};

use crate::{
    app::{AsterApp, Stage},
    screens,
};

impl AsterApp {
    pub(crate) fn show_first_run_setup(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(248, 247, 251))
                    .inner_margin(0.0),
            )
            .show(ctx, |ui| {
                screens::paint_wallpaper(ui, self);
                let card = Rect::from_center_size(ui.max_rect().center(), Vec2::new(438.0, 318.0));
                paint_card(ui, card);
                let mut form = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(card.shrink2(Vec2::new(28.0, 18.0)))
                        .layout(egui::Layout::top_down(Align::Min)),
                );
                brand(&mut form);
                form.add_space(9.0);
                form.label(
                    RichText::new("Create administrator")
                        .font(FontId::proportional(22.0))
                        .strong()
                        .color(Color32::from_rgb(43, 38, 53)),
                );
                form.label(
                    RichText::new("Set up the first account for this installation.")
                        .color(Color32::from_rgb(116, 109, 128)),
                );
                form.add_space(13.0);
                field(&mut form, "User name", |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.username)
                            .desired_width(230.0)
                            .hint_text("Administrator name"),
                    );
                });
                field(&mut form, "Password", |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.password)
                            .password(true)
                            .desired_width(230.0)
                            .hint_text("At least 10 characters"),
                    );
                });
                field(&mut form, "Confirm", |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.confirm_password)
                            .password(true)
                            .desired_width(230.0)
                            .hint_text("Repeat password"),
                    );
                });
                if !self.message.is_empty() {
                    form.colored_label(Color32::from_rgb(171, 54, 62), &self.message);
                }
                form.add_space(6.0);
                form.horizontal(|ui| {
                    if ui.button("Back").clicked() {
                        self.set_stage(Stage::SelectYearAndDate, ctx);
                    }
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if primary_button(ui, "Create account").clicked() {
                            let username = self.username.trim();
                            if username.is_empty() {
                                self.message = "Enter a user name.".into();
                            } else if self.password.len() < 10 {
                                self.message = "Use a password with at least 10 characters.".into();
                            } else if self.password != self.confirm_password {
                                self.message = "The passwords do not match.".into();
                            } else if let Some(db) = &self.db {
                                match db.create_admin(username, &self.password) {
                                    Ok(()) => {
                                        self.username = username.to_owned();
                                        self.password.clear();
                                        self.confirm_password.clear();
                                        self.message.clear();
                                        self.set_stage(Stage::Login, ctx);
                                    }
                                    Err(error) => self.message = error,
                                }
                            } else {
                                self.message = "Database is unavailable.".into();
                            }
                        }
                    });
                });
                screens::show_background_error(ui, self);
            });
    }

    pub(crate) fn show_login(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(248, 247, 251))
                    .inner_margin(0.0),
            )
            .show(ctx, |ui| {
                screens::paint_wallpaper(ui, self);
                let card = Rect::from_center_size(ui.max_rect().center(), Vec2::new(438.0, 284.0));
                paint_card(ui, card);
                let mut form = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(card.shrink2(Vec2::new(28.0, 21.0)))
                        .layout(egui::Layout::top_down(Align::Min)),
                );
                brand(&mut form);
                form.add_space(10.0);
                form.label(
                    RichText::new("Welcome back")
                        .font(FontId::proportional(23.0))
                        .strong()
                        .color(Color32::from_rgb(43, 38, 53)),
                );
                form.horizontal(|ui| {
                    ui.label("Sign in to ");
                    ui.label(
                        RichText::new(format!(
                            "{} – {}",
                            self.selected_year,
                            self.selected_year + 1
                        ))
                        .strong()
                        .color(Color32::from_rgb(75, 51, 117)),
                    );
                });
                form.add_space(13.0);
                field(&mut form, "User name", |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.username)
                            .desired_width(238.0)
                            .hint_text("Enter your user name"),
                    );
                });
                field(&mut form, "Password", |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.password)
                            .password(true)
                            .desired_width(238.0)
                            .hint_text("Enter your password"),
                    );
                });
                if !self.message.is_empty() {
                    form.colored_label(Color32::from_rgb(171, 54, 62), &self.message);
                }
                form.add_space(7.0);
                form.horizontal(|ui| {
                    if ui.button("Back").clicked() {
                        self.password.clear();
                        self.set_stage(Stage::SelectYearAndDate, ctx);
                    }
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if primary_button(ui, "Sign in").clicked() {
                            let result = self
                                .db
                                .as_ref()
                                .map(|db| db.verify_user(self.username.trim(), &self.password));
                            self.password.clear();
                            match result {
                                Some(Ok(true)) => {
                                    self.current_user = self.username.trim().to_owned();
                                    self.display_name = self.current_user.clone();
                                    self.profile_message.clear();
                                    self.message.clear();
                                    self.workspace_page = super::WorkspacePage::Dashboard;
                                    self.set_stage(Stage::Main, ctx);
                                }
                                Some(Ok(false)) => {
                                    self.message = "User name or password is incorrect.".into();
                                }
                                Some(Err(error)) => self.message = error,
                                None => self.message = "Database is unavailable.".into(),
                            }
                        }
                    });
                });
                screens::show_background_error(ui, self);
            });
    }
}

fn paint_card(ui: &egui::Ui, card: Rect) {
    ui.painter().rect_filled(
        card,
        16.0,
        Color32::from_rgba_unmultiplied(255, 255, 255, 246),
    );
    ui.painter().rect_stroke(
        card,
        16.0,
        egui::Stroke::new(1.0_f32, Color32::from_rgb(234, 231, 240)),
        egui::StrokeKind::Inside,
    );
}

fn brand(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
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
}

fn field(ui: &mut egui::Ui, label: &str, add_input: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            Vec2::new(112.0, 28.0),
            egui::Label::new(RichText::new(label).strong()),
        );
        add_input(ui);
    });
    ui.add_space(6.0);
}

fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).strong().color(Color32::WHITE))
            .fill(Color32::from_rgb(91, 65, 139))
            .min_size(Vec2::new(128.0, 34.0)),
    )
}
