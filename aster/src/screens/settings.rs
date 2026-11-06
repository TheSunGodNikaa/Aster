use crate::app::AsterApp;
use eframe::egui::{self, Align, Color32, CornerRadius, Layout, Margin, RichText, Sense, Stroke};

const INK: Color32 = Color32::from_rgb(43, 39, 52);
const MUTED: Color32 = Color32::from_rgb(100, 94, 111);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const PURPLE_TINT: Color32 = Color32::from_rgb(241, 237, 248);
const BORDER: Color32 = Color32::from_rgb(226, 223, 232);
const HAIRLINE: Color32 = Color32::from_rgb(238, 235, 243);
const GREEN: Color32 = Color32::from_rgb(45, 146, 94);
const GREEN_TINT: Color32 = Color32::from_rgb(236, 247, 241);
const GREEN_BORDER: Color32 = Color32::from_rgb(197, 228, 211);
const RED: Color32 = Color32::from_rgb(177, 57, 57);
const RED_TINT: Color32 = Color32::from_rgb(253, 244, 244);
const RED_BORDER: Color32 = Color32::from_rgb(236, 205, 205);
const AMBER: Color32 = Color32::from_rgb(176, 112, 20);
const AMBER_TINT: Color32 = Color32::from_rgb(255, 249, 237);
const AMBER_BORDER: Color32 = Color32::from_rgb(240, 223, 187);
const WHITE: Color32 = Color32::WHITE;

pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    // ── page header ─────────────────────────────────────────────
    ui.label(RichText::new("SYSTEM").size(11.0).strong().color(PURPLE));
    ui.add_space(3.0);
    ui.heading(
        RichText::new("System settings")
            .size(28.0)
            .strong()
            .color(INK),
    );
    ui.label(
        RichText::new("Manage application preferences, database information and maintenance.")
            .size(13.5)
            .color(MUTED),
    );
    ui.add_space(16.0);

    // ── result banner (appears after a check / backup / restore) ─
    draw_message(ui, app);

    // Read once; every card below uses it.
    let years_result = app.db.as_ref().map(|db| db.financial_years());
    let db_connected = app.db.is_some();
    let years_count = match &years_result {
        Some(Ok(years)) => years.len().to_string(),
        _ => "—".to_owned(),
    };

    // ── overview tiles ──────────────────────────────────────────
    let selected = format!("{}–{}", app.selected_year, app.selected_year + 1);
    ui.columns(4, |columns| {
        tile(
            &mut columns[0],
            "Version",
            env!("CARGO_PKG_VERSION"),
            "ASTER",
            PURPLE,
        );
        tile(
            &mut columns[1],
            "Financial year",
            &selected,
            "Currently selected",
            PURPLE,
        );
        tile(
            &mut columns[2],
            "Registered years",
            &years_count,
            "In this installation",
            PURPLE,
        );
        tile(
            &mut columns[3],
            "Database",
            if db_connected {
                "Connected"
            } else {
                "Unavailable"
            },
            "SQLite storage",
            if db_connected { GREEN } else { RED },
        );
    });

    ui.add_space(14.0);

    // ── preferences ─────────────────────────────────────────────
    card(ui, |ui| {
        section_heading(
            ui,
            "Application preferences",
            "Changes apply immediately to this session.",
        );
        ui.add_space(10.0);

        setting_row(
            ui,
            "Compact spacing",
            "Use tighter workspace spacing to fit more on screen.",
            &mut app.compact_density,
        );
        hairline(ui);
        setting_row(
            ui,
            "Navigation tooltips",
            "Show page names when the sidebar is collapsed.",
            &mut app.show_tooltips,
        );
        hairline(ui);
        setting_row(
            ui,
            "Reduce motion",
            "Disable interface animations.",
            &mut app.reduced_motion,
        );
    });

    ui.add_space(14.0);

    // ── database information + maintenance ──────────────────────
    ui.columns(2, |columns| {
        card(&mut columns[0], |ui| {
            ui.set_min_height(250.0);
            section_heading(ui, "Database information", "Current installation");
            ui.add_space(10.0);

            info_row(ui, "Application", "ASTER");
            hairline(ui);
            info_row(ui, "Version", env!("CARGO_PKG_VERSION"));
            hairline(ui);
            info_row(ui, "Selected financial year", &selected);
            hairline(ui);

            match &years_result {
                Some(Ok(years)) => {
                    info_row(ui, "Registered financial years", &years.len().to_string());
                    hairline(ui);

                    ui.add_space(6.0);
                    ui.label(RichText::new("Available years").size(12.5).color(MUTED));
                    ui.add_space(6.0);
                    if years.is_empty() {
                        ui.label(RichText::new("None").size(12.5).strong().color(INK));
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                            for year in years {
                                let current = *year == app.selected_year;
                                pill(
                                    ui,
                                    &format!("{}–{}", year, year + 1),
                                    if current { WHITE } else { PURPLE },
                                    if current { PURPLE } else { PURPLE_TINT },
                                );
                            }
                        });
                    }
                    ui.add_space(8.0);
                    hairline(ui);
                    status_row(ui, "Database connection", "Connected", GREEN, GREEN_TINT);
                }
                Some(Err(error)) => {
                    ui.add_space(4.0);
                    notice(ui, error, RED, RED_TINT, RED_BORDER);
                    ui.add_space(8.0);
                    status_row(ui, "Database connection", "Connected", GREEN, GREEN_TINT);
                }
                None => {
                    ui.add_space(4.0);
                    notice(ui, "Database unavailable", RED, RED_TINT, RED_BORDER);
                    ui.add_space(8.0);
                    status_row(ui, "Database connection", "Unavailable", RED, RED_TINT);
                }
            }
        });

        card(&mut columns[1], |ui| {
            ui.set_min_height(250.0);
            section_heading(ui, "Maintenance", "Basic application checks");
            ui.add_space(10.0);

            status_row(
                ui,
                "SQLite database",
                "Available",
                GREEN,
                GREEN_TINT,
            );
            hairline(ui);

            ui.add_space(6.0);
            ui.label(
                RichText::new("Financial-year database check")
                    .size(13.5)
                    .strong()
                    .color(INK),
            );
            ui.add_space(3.0);
            ui.label(
                RichText::new(
                    "Runs SQLite quick_check on every registered financial-year database to detect corruption early.",
                )
                .size(12.0)
                .color(MUTED),
            );
            ui.add_space(12.0);

            if ui.add(primary_button("Run database check")).clicked() {
                app.profile_message = run_database_check(app);
            }
        });
    });

    ui.add_space(14.0);

    // ── backup & restore ────────────────────────────────────────
    card(ui, |ui| {
        section_heading(ui, "Backup & restore", "Protect your business data.");
        ui.add_space(8.0);

        ui.label(
            RichText::new(
                "Create a complete backup of the global database and every registered financial year. \
                 Restore validates the archive and creates a safety backup before replacing data.",
            )
            .size(13.0)
            .color(MUTED),
        );

        ui.add_space(14.0);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            if ui.add(primary_button("Create backup")).clicked() {
                create_backup(app);
            }

            if ui.add(warning_button("Restore backup")).clicked() {
                restore_backup(ui.ctx(), app);
            }
        });

        ui.add_space(12.0);
        notice(
            ui,
            "Important: close other ASTER instances before restoring. Keep the safety backup until you have verified the restored data.",
            AMBER,
            AMBER_TINT,
            AMBER_BORDER,
        );
    });

    ui.add_space(24.0);
}

// ───────────────────────────── actions (logic unchanged) ─────────────────────────────

fn run_database_check(app: &AsterApp) -> String {
    match &app.db {
        Some(db) => match db.financial_years() {
            Ok(years) => {
                let mut result = None;
                for year in years {
                    match db.year_connection(year) {
                        Ok(connection) => {
                            let check: Result<String, _> =
                                connection.query_row("PRAGMA quick_check", [], |row| row.get(0));
                            match check {
                                Ok(value) if value == "ok" => {}
                                Ok(value) => {
                                    result =
                                        Some(format!("Database check failed for {year}: {value}"));
                                    break;
                                }
                                Err(error) => {
                                    result =
                                        Some(format!("Database check failed for {year}: {error}"));
                                    break;
                                }
                            }
                        }
                        Err(error) => {
                            result = Some(error);
                            break;
                        }
                    }
                }

                result.unwrap_or_else(|| "Database checks completed successfully.".into())
            }
            Err(error) => error,
        },
        None => "Database is unavailable.".into(),
    }
}

fn create_backup(app: &mut AsterApp) {
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Save ASTER backup")
        .add_filter("ASTER backup", &["asterbackup"])
        .set_file_name(format!("ASTER-{}.asterbackup", unique_backup_date()))
        .save_file()
    {
        app.profile_message = match &app.db {
            Some(db) => match db.backup_to_path(&path) {
                Ok(()) => format!("Backup created successfully: {}", path.display()),
                Err(error) => format!("Backup failed: {error}"),
            },
            None => "Database is unavailable.".into(),
        };
    }
}

fn restore_backup(ctx: &egui::Context, app: &mut AsterApp) {
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Select ASTER backup")
        .add_filter("ASTER backup", &["asterbackup"])
        .pick_file()
    {
        let restore_result = match app.db.as_ref() {
            Some(db) => db.restore_from_path(&path),
            None => Err("Database is unavailable.".into()),
        };

        match restore_result {
            Ok(safety_path) => {
                let years_result = app
                    .db
                    .as_ref()
                    .ok_or_else(|| "Database is unavailable.".to_owned())
                    .and_then(|db| db.financial_years());

                match years_result {
                    Ok(years) => {
                        app.financial_years = years;

                        if !app.financial_years.contains(&app.selected_year) {
                            if let Some(year) = app.financial_years.first() {
                                app.selected_year = *year;
                            }
                        }

                        app.workspace_page = crate::screens::WorkspacePage::Dashboard;
                        app.open_pages = vec![crate::screens::WorkspacePage::Dashboard];

                        app.profile_message = format!(
                            "Restore completed successfully. Safety backup saved at: {}",
                            safety_path.display()
                        );
                    }
                    Err(error) => {
                        app.profile_message = format!(
                            "Restore completed, but refreshing financial years failed: {error}"
                        );
                    }
                }

                ctx.request_repaint();
            }
            Err(error) => {
                app.profile_message = format!("Restore failed: {error}");
            }
        }
    }
}

fn unique_backup_date() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}

// ───────────────────────────── message banner ─────────────────────────────

fn draw_message(ui: &mut egui::Ui, app: &mut AsterApp) {
    if app.profile_message.is_empty() {
        return;
    }

    let message = app.profile_message.clone();
    let lower = message.to_lowercase();
    let is_error = lower.contains("failed") || lower.contains("unavailable");
    let (title, color, tint, border) = if is_error {
        ("Action needed", RED, RED_TINT, RED_BORDER)
    } else {
        ("Success", GREEN, GREEN_TINT, GREEN_BORDER)
    };

    let mut dismiss = false;

    egui::Frame::new()
        .fill(tint)
        .stroke(Stroke::new(1.0_f32, border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).size(13.0).strong().color(color));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Dismiss").size(11.5).color(color))
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE),
                        )
                        .clicked()
                    {
                        dismiss = true;
                    }
                });
            });
            ui.add_space(2.0);
            ui.add(
                egui::Label::new(RichText::new(&message).size(12.5).color(INK)).selectable(true),
            );
        });

    ui.add_space(14.0);

    if dismiss {
        app.profile_message.clear();
    }
}

// ───────────────────────────── building blocks ─────────────────────────────

fn card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(18.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add_contents(ui)
        })
        .inner
}

fn tile(ui: &mut egui::Ui, label: &str, value: &str, note: &str, accent: Color32) {
    card(ui, |ui| {
        ui.set_min_height(76.0);
        let (bar, _) = ui.allocate_exact_size(egui::vec2(26.0, 3.0), Sense::hover());
        ui.painter().rect_filled(bar, CornerRadius::same(2), accent);
        ui.add_space(6.0);
        ui.label(
            RichText::new(label.to_uppercase())
                .size(10.5)
                .strong()
                .color(MUTED),
        );
        ui.add_space(1.0);
        ui.label(RichText::new(value).size(19.0).strong().color(INK));
        ui.label(RichText::new(note).size(11.0).color(MUTED));
    });
}

fn section_heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(16.0).strong().color(INK));
    ui.add_space(2.0);
    ui.label(RichText::new(subtitle).size(12.5).color(MUTED));
}

fn hairline(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::ZERO, HAIRLINE);
    ui.add_space(6.0);
}

fn info_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.5).color(MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).size(12.5).strong().color(INK));
        });
    });
}

fn status_row(ui: &mut egui::Ui, label: &str, status: &str, color: Color32, tint: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.5).color(MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            pill(ui, status, color, tint);
        });
    });
}

fn pill(ui: &mut egui::Ui, text: &str, color: Color32, tint: Color32) {
    egui::Frame::new()
        .fill(tint)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(9, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).strong().color(color));
        });
}

fn notice(ui: &mut egui::Ui, text: &str, color: Color32, tint: Color32, border: Color32) {
    egui::Frame::new()
        .fill(tint)
        .stroke(Stroke::new(1.0_f32, border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(text).size(12.0).color(color));
        });
}

fn setting_row(ui: &mut egui::Ui, title: &str, description: &str, value: &mut bool) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(title).size(13.5).strong().color(INK));
            ui.label(RichText::new(description).size(12.0).color(MUTED));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            toggle(ui, value);
            ui.label(
                RichText::new(if *value { "On" } else { "Off" })
                    .size(11.5)
                    .strong()
                    .color(if *value { PURPLE } else { MUTED }),
            );
        });
    });
}

fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = egui::vec2(40.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

    if response.clicked() {
        *on = !*on;
    }

    let radius = rect.height() / 2.0;
    let track = if *on {
        PURPLE
    } else {
        Color32::from_rgb(204, 200, 213)
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(radius as u8), track);

    let knob_x = if *on {
        rect.right() - radius
    } else {
        rect.left() + radius
    };
    ui.painter()
        .circle_filled(egui::pos2(knob_x, rect.center().y), radius - 3.0, WHITE);

    response
}

fn primary_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(
        RichText::new(text.to_owned())
            .size(12.5)
            .strong()
            .color(WHITE),
    )
    .fill(PURPLE)
    .stroke(Stroke::new(1.0_f32, PURPLE))
    .corner_radius(CornerRadius::same(8))
    .min_size(egui::vec2(0.0, 32.0))
}

fn warning_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(
        RichText::new(text.to_owned())
            .size(12.5)
            .strong()
            .color(AMBER),
    )
    .fill(AMBER_TINT)
    .stroke(Stroke::new(1.0_f32, AMBER_BORDER))
    .corner_radius(CornerRadius::same(8))
    .min_size(egui::vec2(0.0, 32.0))
}
