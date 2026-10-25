use crate::{app::AsterApp, database::CustomerRecord};
use eframe::egui::{self, Align, Color32, Id, Layout, RichText, ScrollArea, Stroke, Vec2};

const INK: Color32 = Color32::from_rgb(43, 39, 52);
const MUTED: Color32 = Color32::from_rgb(100, 94, 111);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const BORDER: Color32 = Color32::from_rgb(226, 223, 232);
const PANE: Color32 = Color32::from_rgb(249, 248, 252);
const RED: Color32 = Color32::from_rgb(176, 58, 58);
const GREEN: Color32 = Color32::from_rgb(38, 110, 72);
const FORM_W: f32 = 390.0;

#[derive(Clone, Default)]
struct Form {
    id: Option<i64>,
    name: String,
    phone: String,
    email: String,
    gstin: String,
    drug_license: String,
    address_line1: String,
    address_line2: String,
    city: String,
    state: String,
    state_code: String,
    pincode: String,
    credit_limit: String,
    opening_balance: String,
    active: bool,
}
impl Form {
    fn from_record(c: &CustomerRecord) -> Self {
        Self {
            id: Some(c.id),
            name: c.name.clone(),
            phone: c.phone.clone(),
            email: c.email.clone(),
            gstin: c.gstin.clone(),
            drug_license: c.drug_license.clone(),
            address_line1: c.address_line1.clone(),
            address_line2: c.address_line2.clone(),
            city: c.city.clone(),
            state: c.state.clone(),
            state_code: c.state_code.clone(),
            pincode: c.pincode.clone(),
            credit_limit: format!("{:.2}", c.credit_limit_paise as f64 / 100.0),
            opening_balance: format!("{:.2}", c.opening_balance_paise as f64 / 100.0),
            active: c.active,
        }
    }
    fn record(&self) -> Result<CustomerRecord, String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Customer name is required.".into());
        }
        let money = |s: &str, label: &str| -> Result<i64, String> {
            let value = if s.trim().is_empty() {
                0.0
            } else {
                s.trim()
                    .parse::<f64>()
                    .map_err(|_| format!("Enter a valid {label}."))?
            };
            if !value.is_finite() || value.abs() > (i64::MAX / 100) as f64 {
                return Err(format!("{label} is out of range."));
            }
            if label == "credit limit" && value < 0.0 {
                return Err("Credit limit cannot be negative.".into());
            }
            Ok((value * 100.0).round() as i64)
        };
        Ok(CustomerRecord {
            id: self.id.unwrap_or_default(),
            name: name.into(),
            phone: self.phone.trim().into(),
            email: self.email.trim().into(),
            gstin: self.gstin.trim().to_uppercase(),
            drug_license: self.drug_license.trim().into(),
            address_line1: self.address_line1.trim().into(),
            address_line2: self.address_line2.trim().into(),
            city: self.city.trim().into(),
            state: self.state.trim().into(),
            state_code: self.state_code.trim().into(),
            pincode: self.pincode.trim().into(),
            credit_limit_paise: money(&self.credit_limit, "credit limit")?,
            opening_balance_paise: money(&self.opening_balance, "opening balance")?,
            active: self.active,
        })
    }
}
#[derive(Clone, Default)]
struct UiState {
    search: String,
    rows: Vec<CustomerRecord>,
    form: Form,
    selected: Option<i64>,
    loaded: bool,
    show_inactive: bool,
    message: String,
    error: bool,
}

pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let id = Id::new(("customer-master-state", app.selected_year));
    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<UiState>(id).unwrap_or_default());
    if !state.loaded {
        reload(&mut state, app);
    }
    let height = ui.available_height().max(300.0);
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("CUSTOMER MASTER")
                        .size(11.0)
                        .strong()
                        .color(PURPLE),
                );
                ui.heading(RichText::new("Customers").size(25.0).color(INK));
                ui.label(RichText::new("Manage customer accounts and credit details").color(MUTED));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button(RichText::new("+ New customer").strong())
                    .clicked()
                {
                    state.form = Form::default();
                    state.selected = None;
                    state.message.clear();
                }
                if ui.button("Refresh").clicked() {
                    reload(&mut state, app);
                }
            });
        });
        ui.add_space(12.0);
        if !state.message.is_empty() {
            ui.colored_label(if state.error { RED } else { GREEN }, &state.message);
            ui.add_space(6.0);
        }
        ui.horizontal(|ui| {
            ui.label("Search");
            let response = ui.add_sized(
                [ui.available_width().min(350.0), 30.0],
                egui::TextEdit::singleline(&mut state.search)
                    .hint_text("Name, phone, GSTIN, city…"),
            );
            if response.changed() {
                reload(&mut state, app);
            }
            ui.checkbox(&mut state.show_inactive, "Include inactive");
        });
        ui.add_space(10.0);
        let total_w = ui.available_width();
        let form_w = FORM_W.min((total_w * 0.48).max(300.0));
        ui.allocate_ui_with_layout(
            Vec2::new(total_w, (height - 100.0).max(200.0)),
            Layout::left_to_right(Align::Min),
            |ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new((total_w - form_w - 12.0).max(220.0), ui.available_height()),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.group(|ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.strong("Customer list");
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("{} records", state.rows.len()))
                                            .color(MUTED),
                                    );
                                });
                            });
                            ui.add_space(6.0);
                            ScrollArea::vertical()
                                .id_salt("customer-list-scroll")
                                .auto_shrink([false, false])
                                .max_height((ui.available_height() - 8.0).max(120.0))
                                .show(ui, |ui| {
                                    for c in state
                                        .rows
                                        .iter()
                                        .filter(|c| state.show_inactive || c.active)
                                    {
                                        let selected = state.selected == Some(c.id);
                                        let frame = egui::Frame::new()
                                            .fill(if selected {
                                                Color32::from_rgb(241, 237, 248)
                                            } else {
                                                Color32::TRANSPARENT
                                            })
                                            .stroke(Stroke::new(
                                                1.0,
                                                if selected { PURPLE } else { BORDER },
                                            ))
                                            .corner_radius(8.0)
                                            .inner_margin(8.0);
                                        let response = frame
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.vertical(|ui| {
                                                        ui.label(
                                                            RichText::new(&c.name)
                                                                .strong()
                                                                .color(INK),
                                                        );
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "{}  ·  {}",
                                                                if c.phone.is_empty() {
                                                                    "No phone"
                                                                } else {
                                                                    &c.phone
                                                                },
                                                                if c.city.is_empty() {
                                                                    "No city"
                                                                } else {
                                                                    &c.city
                                                                }
                                                            ))
                                                            .size(12.0)
                                                            .color(MUTED),
                                                        );
                                                    });
                                                    ui.with_layout(
                                                        Layout::right_to_left(Align::Center),
                                                        |ui| {
                                                            ui.label(
                                                                RichText::new(if c.active {
                                                                    "ACTIVE"
                                                                } else {
                                                                    "INACTIVE"
                                                                })
                                                                .size(10.0)
                                                                .strong()
                                                                .color(if c.active {
                                                                    GREEN
                                                                } else {
                                                                    RED
                                                                }),
                                                            );
                                                        },
                                                    );
                                                });
                                            })
                                            .response;
                                        if response.interact(egui::Sense::click()).clicked() {
                                            state.selected = Some(c.id);
                                            state.form = Form::from_record(c);
                                            state.message.clear();
                                        }
                                        ui.add_space(4.0);
                                    }
                                    if state.rows.is_empty() {
                                        ui.centered_and_justified(|ui| {
                                            ui.label(
                                                RichText::new("No customers found").color(MUTED),
                                            );
                                        });
                                    }
                                });
                        });
                    },
                );
                ui.add_space(12.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(form_w, ui.available_height()),
                    Layout::top_down(Align::Min),
                    |ui| {
                        egui::Frame::new()
                            .fill(PANE)
                            .stroke(Stroke::new(1.0, BORDER))
                            .corner_radius(10.0)
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.heading(
                                    RichText::new(if state.form.id.is_some() {
                                        "Edit customer"
                                    } else {
                                        "Add customer"
                                    })
                                    .size(18.0)
                                    .color(INK),
                                );
                                ui.add_space(6.0);
                                ScrollArea::vertical()
                                    .id_salt("customer-form-scroll")
                                    .auto_shrink([false, false])
                                    .max_height((ui.available_height() - 78.0).max(120.0))
                                    .show(ui, |ui| {
                                        field(ui, "Customer name *", &mut state.form.name);
                                        field(ui, "Phone", &mut state.form.phone);
                                        field(ui, "Email", &mut state.form.email);
                                        field(ui, "GSTIN", &mut state.form.gstin);
                                        field(ui, "Drug licence", &mut state.form.drug_license);
                                        field(ui, "Address line 1", &mut state.form.address_line1);
                                        field(ui, "Address line 2", &mut state.form.address_line2);
                                        field(ui, "City", &mut state.form.city);
                                        field(ui, "State", &mut state.form.state);
                                        field(ui, "State code", &mut state.form.state_code);
                                        field(ui, "Pincode", &mut state.form.pincode);
                                        field(ui, "Credit limit (₹)", &mut state.form.credit_limit);
                                        field(
                                            ui,
                                            "Opening balance (₹)",
                                            &mut state.form.opening_balance,
                                        );
                                        if state.form.id.is_some() {
                                            ui.checkbox(&mut state.form.active, "Active customer");
                                        }
                                    });
                                ui.separator();
                                ui.horizontal(|ui| {
                                    if ui.button(RichText::new("Save customer").strong()).clicked()
                                    {
                                        match state.form.record() {
                                            Err(e) => {
                                                state.message = e;
                                                state.error = true;
                                            }
                                            Ok(record) => match app
                                                .db
                                                .as_ref()
                                                .ok_or_else(|| "Database is not open.".to_owned())
                                                .and_then(|db| {
                                                    db.save_customer(
                                                        app.selected_year,
                                                        state.form.id,
                                                        &record,
                                                    )
                                                }) {
                                                Ok(()) => {
                                                    state.message = "Customer saved.".into();
                                                    state.error = false;
                                                    reload(&mut state, app);
                                                }
                                                Err(e) => {
                                                    state.message = e;
                                                    state.error = true;
                                                }
                                            },
                                        }
                                    }
                                    if state.form.id.is_some() && ui.button("Deactivate").clicked()
                                    {
                                        if let Some(id) = state.form.id {
                                            match app
                                                .db
                                                .as_ref()
                                                .ok_or_else(|| "Database is not open.".to_owned())
                                                .and_then(|db| {
                                                    db.set_customer_active(
                                                        app.selected_year,
                                                        id,
                                                        false,
                                                    )
                                                }) {
                                                Ok(()) => {
                                                    state.message = "Customer deactivated.".into();
                                                    state.error = false;
                                                    state.form = Form::default();
                                                    state.selected = None;
                                                    reload(&mut state, app)
                                                }
                                                Err(e) => {
                                                    state.message = e;
                                                    state.error = true;
                                                }
                                            }
                                        }
                                    }
                                });
                            });
                    },
                );
            },
        );
    });
    ui.ctx().data_mut(|d| d.insert_temp(id, state));
}
fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(RichText::new(label).size(12.0).color(MUTED));
    ui.add_sized(
        [ui.available_width(), 28.0],
        egui::TextEdit::singleline(value),
    );
    ui.add_space(5.0);
}
fn reload(state: &mut UiState, app: &AsterApp) {
    state.loaded = true;
    if let Some(db) = app.db.as_ref() {
        match db.customers(app.selected_year, &state.search) {
            Ok(rows) => state.rows = rows,
            Err(e) => {
                state.message = e;
                state.error = true;
                state.rows.clear();
            }
        }
    } else {
        state.rows.clear();
    }
}
