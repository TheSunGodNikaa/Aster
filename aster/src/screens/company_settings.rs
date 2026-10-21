use eframe::egui::{
    self, Align, Color32, FontId, Id, Key, Margin, Rect, RichText, Sense, Stroke, Vec2,
};

use crate::{app::AsterApp, database::CompanySettings};

const INK: Color32 = Color32::from_rgb(43, 39, 52);
const MUTED: Color32 = Color32::from_rgb(100, 94, 111);
const SOFT: Color32 = Color32::from_rgb(127, 120, 139);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const PURPLE_DEEP: Color32 = Color32::from_rgb(74, 50, 120);
const PURPLE_TINT: Color32 = Color32::from_rgb(241, 237, 248);
const BORDER: Color32 = Color32::from_rgb(226, 223, 232);
const DANGER: Color32 = Color32::from_rgb(176, 58, 58);
const AMBER: Color32 = Color32::from_rgb(200, 133, 30);

/// GST state codes (code, name).
const STATES: &[(&str, &str)] = &[
    ("01", "Jammu & Kashmir"),
    ("02", "Himachal Pradesh"),
    ("03", "Punjab"),
    ("04", "Chandigarh"),
    ("05", "Uttarakhand"),
    ("06", "Haryana"),
    ("07", "Delhi"),
    ("08", "Rajasthan"),
    ("09", "Uttar Pradesh"),
    ("10", "Bihar"),
    ("11", "Sikkim"),
    ("12", "Arunachal Pradesh"),
    ("13", "Nagaland"),
    ("14", "Manipur"),
    ("15", "Mizoram"),
    ("16", "Tripura"),
    ("17", "Meghalaya"),
    ("18", "Assam"),
    ("19", "West Bengal"),
    ("20", "Jharkhand"),
    ("21", "Odisha"),
    ("22", "Chhattisgarh"),
    ("23", "Madhya Pradesh"),
    ("24", "Gujarat"),
    ("26", "Dadra & Nagar Haveli and Daman & Diu"),
    ("27", "Maharashtra"),
    ("29", "Karnataka"),
    ("30", "Goa"),
    ("31", "Lakshadweep"),
    ("32", "Kerala"),
    ("33", "Tamil Nadu"),
    ("34", "Puducherry"),
    ("35", "Andaman & Nicobar Islands"),
    ("36", "Telangana"),
    ("37", "Andhra Pradesh"),
    ("38", "Ladakh"),
    ("97", "Other Territory"),
    ("99", "Centre Jurisdiction"),
];

#[derive(Clone)]
struct CompanyForm {
    year: i32,
    settings: CompanySettings,
    /// Fingerprint of the last saved / loaded values, used to detect edits.
    original: String,
}

// ------------------------------------------------------------------ helpers

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

fn anim(ctx: &egui::Context) -> f32 {
    ctx.data(|d| d.get_temp::<f32>(Id::new("aster_anim_time")))
        .unwrap_or(0.14)
}

fn fingerprint(s: &CompanySettings) -> String {
    [
        &s.legal_name,
        &s.trade_name,
        &s.gstin,
        &s.drug_license_1,
        &s.drug_license_2,
        &s.address_line1,
        &s.address_line2,
        &s.city,
        &s.state,
        &s.state_code,
        &s.pincode,
        &s.phone,
        &s.email,
        &s.invoice_prefix,
        &s.bank_name,
        &s.bank_account_name,
        &s.bank_account_number,
        &s.bank_ifsc,
    ]
    .iter()
    .map(|v| v.as_str())
    .collect::<Vec<_>>()
    .join("\u{1f}")
}

fn flag(ui: &egui::Ui, id: Id) -> bool {
    ui.data(|d| d.get_temp::<bool>(id).unwrap_or(false))
}

fn set_flag(ui: &egui::Ui, id: Id, value: bool) {
    ui.data_mut(|d| d.insert_temp(id, value));
}

// --------------------------------------------------------------- validation

fn required(value: &str, what: &str) -> Option<String> {
    if value.trim().is_empty() {
        Some(format!("{what} is required."))
    } else {
        None
    }
}

fn validate_gstin(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        return Some("GSTIN is required.".into());
    }
    let b: Vec<char> = v.chars().collect();
    let valid = b.len() == 15
        && b[0..2].iter().all(|c| c.is_ascii_digit())
        && b[2..7].iter().all(|c| c.is_ascii_uppercase())
        && b[7..11].iter().all(|c| c.is_ascii_digit())
        && b[11].is_ascii_uppercase()
        && b[12].is_ascii_alphanumeric()
        && b[13] == 'Z'
        && b[14].is_ascii_alphanumeric();
    if valid {
        None
    } else {
        Some("Enter a valid 15-character GSTIN, e.g. 33ABCDE1234F1Z5.".into())
    }
}

fn validate_pincode(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        Some("PIN code is required.".into())
    } else if v.len() != 6 || !v.chars().all(|c| c.is_ascii_digit()) {
        Some("PIN code must be 6 digits.".into())
    } else {
        None
    }
}

fn validate_phone(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        return Some("Phone number is required.".into());
    }
    let digits = v.chars().filter(|c| c.is_ascii_digit()).count();
    let clean = v
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | ' ' | '(' | ')'));
    if clean && (10..=13).contains(&digits) {
        None
    } else {
        Some("Enter a valid phone number (10 to 13 digits).".into())
    }
}

fn validate_email(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        return None; // optional
    }
    let mut parts = v.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return Some("Enter a valid email address.".into());
    };
    if !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !v.contains(char::is_whitespace)
    {
        None
    } else {
        Some("Enter a valid email address.".into())
    }
}

fn validate_prefix(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        Some("Invoice prefix is required.".into())
    } else if v.chars().count() > 10 {
        Some("Use 10 characters or fewer.".into())
    } else if !v
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '/' | '_'))
    {
        Some("Use letters, numbers, - / or _ only.".into())
    } else {
        None
    }
}

fn validate_ifsc(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }
    let b: Vec<char> = v.chars().collect();
    if b.len() == 11
        && b[0..4].iter().all(|c| c.is_ascii_uppercase())
        && b[4] == '0'
        && b[5..].iter().all(|c| c.is_ascii_alphanumeric())
    {
        None
    } else {
        Some("IFSC is 11 characters, e.g. SBIN0001234.".into())
    }
}

fn validate_account_number(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }
    if v.chars().all(|c| c.is_ascii_digit()) && (9..=18).contains(&v.len()) {
        None
    } else {
        Some("Account number is 9 to 18 digits.".into())
    }
}

fn bank_started(s: &CompanySettings) -> bool {
    !s.bank_name.trim().is_empty()
        || !s.bank_account_name.trim().is_empty()
        || !s.bank_account_number.trim().is_empty()
        || !s.bank_ifsc.trim().is_empty()
}

/// Every problem on the form, used to block saving.
fn all_errors(s: &CompanySettings) -> Vec<String> {
    let mut errors: Vec<Option<String>> = vec![
        required(&s.legal_name, "Legal name"),
        required(&s.trade_name, "Trade name"),
        validate_gstin(&s.gstin),
        required(&s.drug_license_1, "Drug licence 1"),
        required(&s.address_line1, "Address line 1"),
        required(&s.city, "City"),
        required(&s.state, "State"),
        validate_pincode(&s.pincode),
        validate_phone(&s.phone),
        validate_email(&s.email),
        validate_prefix(&s.invoice_prefix),
        validate_ifsc(&s.bank_ifsc),
        validate_account_number(&s.bank_account_number),
    ];
    if bank_started(s) {
        errors.push(required(&s.bank_name, "Bank name"));
        errors.push(required(&s.bank_account_name, "Account name"));
        errors.push(required(&s.bank_account_number, "Account number"));
        errors.push(required(&s.bank_ifsc, "IFSC"));
    }
    let gst_state = s.gstin.trim().get(0..2).unwrap_or("");
    if gst_state.len() == 2 && !s.state_code.is_empty() && gst_state != s.state_code {
        errors.push(Some(
            "GSTIN state does not match the selected state.".into(),
        ));
    }
    errors.into_iter().flatten().collect()
}

// --------------------------------------------------------------------- page

pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let id = Id::new("aster_company_settings_form");
    let year = app.selected_year;
    let ctx = ui.ctx().clone();

    let mut form = ctx.data_mut(|data| data.get_temp::<CompanyForm>(id));
    if form.as_ref().map_or(true, |form| form.year != year) {
        let settings = match app.db.as_ref() {
            Some(db) => match db.company_settings(year) {
                Ok(settings) => settings,
                Err(error) => {
                    app.profile_message = error;
                    CompanySettings::default()
                }
            },
            None => {
                app.profile_message = "Database is not available.".into();
                CompanySettings::default()
            }
        };
        form = Some(CompanyForm {
            year,
            original: fingerprint(&settings),
            settings,
        });
        set_flag(ui, Id::new("aster_company_submitted"), false);
    }
    let mut form = form.expect("company settings form initialised");

    let dirty = fingerprint(&form.settings) != form.original;
    let submitted_id = Id::new("aster_company_submitted");
    let mut save_clicked = false;
    let mut discard_clicked = false;

    // ---- header row
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("MASTERS").size(11.0).strong().color(SOFT));
            ui.add_space(4.0);
            ui.label(
                RichText::new("Company settings")
                    .size(28.0)
                    .strong()
                    .color(INK),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(format!(
                    "Business details for financial year {year}–{}. Fields marked * are required.",
                    year + 1
                ))
                .size(14.5)
                .color(MUTED),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            if button(ui, "Save changes", 142.0, dirty, true).clicked() {
                save_clicked = true;
            }
            ui.add_space(8.0);
            if button(ui, "Discard", 96.0, dirty, false).clicked() {
                discard_clicked = true;
            }
            ui.add_space(8.0);
            let t = ctx.animate_bool_with_time(Id::new("aster_company_dirty"), dirty, anim(&ctx));
            if t > 0.0 {
                ui.label(
                    RichText::new("Unsaved changes")
                        .size(12.0)
                        .color(AMBER.gamma_multiply(t)),
                );
                let (r, _) = ui.allocate_exact_size(Vec2::new(8.0, 20.0), Sense::hover());
                ui.painter()
                    .circle_filled(r.center(), 3.5, AMBER.gamma_multiply(t));
            }
        });
    });
    ui.add_space(22.0);

    if ui.input(|i| i.modifiers.command && i.key_pressed(Key::S)) && dirty {
        save_clicked = true;
    }

    let submitted = flag(ui, submitted_id);
    let s = &mut form.settings;

    let wide = ui.available_width() >= 900.0;
    let gap = 16.0;

    // ---- the four cards; placed in 2 columns when there is room
    if wide {
        let col_w = (ui.available_width() - gap) / 2.0;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.allocate_ui_with_layout(
                Vec2::new(col_w, 0.0),
                egui::Layout::top_down(Align::Min),
                |ui| {
                    card_business(ui, s, submitted);
                    ui.add_space(gap);
                    card_invoice(ui, s, submitted);
                },
            );
            ui.allocate_ui_with_layout(
                Vec2::new(col_w, 0.0),
                egui::Layout::top_down(Align::Min),
                |ui| {
                    card_contact(ui, s, submitted);
                    ui.add_space(gap);
                    card_bank(ui, s, submitted);
                },
            );
        });
    } else {
        card_business(ui, s, submitted);
        ui.add_space(gap);
        card_contact(ui, s, submitted);
        ui.add_space(gap);
        card_invoice(ui, s, submitted);
        ui.add_space(gap);
        card_bank(ui, s, submitted);
    }

    // ---- actions
    if discard_clicked {
        if let Some(db) = app.db.as_ref() {
            if let Ok(settings) = db.company_settings(year) {
                form.original = fingerprint(&settings);
                form.settings = settings;
            }
        }
        set_flag(ui, submitted_id, false);
    }

    if save_clicked {
        set_flag(ui, submitted_id, true);
        let errors = all_errors(&form.settings);
        if !errors.is_empty() {
            app.profile_message = format!(
                "Please correct the highlighted fields ({} to fix).",
                errors.len()
            );
        } else {
            // Normalise whitespace before saving.
            let st = &mut form.settings;
            for field in [
                &mut st.legal_name,
                &mut st.trade_name,
                &mut st.gstin,
                &mut st.drug_license_1,
                &mut st.drug_license_2,
                &mut st.address_line1,
                &mut st.address_line2,
                &mut st.city,
                &mut st.pincode,
                &mut st.phone,
                &mut st.email,
                &mut st.invoice_prefix,
                &mut st.bank_name,
                &mut st.bank_account_name,
                &mut st.bank_account_number,
                &mut st.bank_ifsc,
            ] {
                *field = field.trim().to_owned();
            }
            match app.db.as_ref() {
                Some(db) => match db.save_company_settings(year, &form.settings) {
                    Ok(()) => {
                        form.original = fingerprint(&form.settings);
                        set_flag(ui, submitted_id, false);
                        app.profile_message = "Company settings saved successfully.".into();
                    }
                    Err(error) => app.profile_message = error,
                },
                None => app.profile_message = "Database is not available.".into(),
            }
        }
    }

    ctx.data_mut(|data| data.insert_temp(id, form));
}

// -------------------------------------------------------------------- cards

fn card_business(ui: &mut egui::Ui, s: &mut CompanySettings, submitted: bool) {
    card(
        ui,
        0,
        "Business information",
        "Legal identity and drug licences",
        |ui| {
            text_field(
                ui,
                "Legal name",
                true,
                "As registered with GST",
                &mut s.legal_name,
                submitted,
                &|v: &str| required(v, "Legal name"),
            );
            text_field(
                ui,
                "Trade name",
                true,
                "Name printed on invoices",
                &mut s.trade_name,
                submitted,
                &|v: &str| required(v, "Trade name"),
            );
            let gstin = text_field(
                ui,
                "GSTIN",
                true,
                "33ABCDE1234F1Z5",
                &mut s.gstin,
                submitted,
                &validate_gstin,
            );
            if gstin.changed() {
                s.gstin = s.gstin.to_uppercase().replace(' ', "");
                s.gstin.truncate(15);
                // Pre-fill the state from the first two digits of the GSTIN.
                if let Some(code) = s.gstin.get(0..2) {
                    if s.state.is_empty() {
                        if let Some((c, n)) = STATES.iter().find(|(c, _)| *c == code) {
                            s.state = (*n).to_owned();
                            s.state_code = (*c).to_owned();
                        }
                    }
                }
            }
            ui.columns(2, |cols| {
                text_field(
                    &mut cols[0],
                    "Drug licence 1",
                    true,
                    "Form 20B",
                    &mut s.drug_license_1,
                    submitted,
                    &|v: &str| required(v, "Drug licence 1"),
                );
                text_field(
                    &mut cols[1],
                    "Drug licence 2",
                    false,
                    "Form 21B",
                    &mut s.drug_license_2,
                    submitted,
                    &|_: &str| None,
                );
            });
        },
    );
}

fn card_contact(ui: &mut egui::Ui, s: &mut CompanySettings, submitted: bool) {
    card(
        ui,
        1,
        "Address & contact",
        "Printed on invoices and reports",
        |ui| {
            text_field(
                ui,
                "Address line 1",
                true,
                "Door no., street",
                &mut s.address_line1,
                submitted,
                &|v: &str| required(v, "Address line 1"),
            );
            text_field(
                ui,
                "Address line 2",
                false,
                "Area, landmark",
                &mut s.address_line2,
                submitted,
                &|_: &str| None,
            );
            ui.columns(2, |cols| {
                text_field(
                    &mut cols[0],
                    "City",
                    true,
                    "City / town",
                    &mut s.city,
                    submitted,
                    &|v: &str| required(v, "City"),
                );
                text_field(
                    &mut cols[1],
                    "PIN code",
                    true,
                    "6 digits",
                    &mut s.pincode,
                    submitted,
                    &validate_pincode,
                );
            });
            state_picker(ui, s, submitted);
            ui.columns(2, |cols| {
                text_field(
                    &mut cols[0],
                    "Phone",
                    true,
                    "10-digit number",
                    &mut s.phone,
                    submitted,
                    &validate_phone,
                );
                text_field(
                    &mut cols[1],
                    "Email",
                    false,
                    "name@company.com",
                    &mut s.email,
                    submitted,
                    &validate_email,
                );
            });
        },
    );
}

fn card_invoice(ui: &mut egui::Ui, s: &mut CompanySettings, submitted: bool) {
    card(
        ui,
        2,
        "Invoice settings",
        "How your bill numbers begin",
        |ui| {
            text_field(
                ui,
                "Invoice prefix",
                true,
                "e.g. AS/26-27/",
                &mut s.invoice_prefix,
                submitted,
                &validate_prefix,
            );
            let sample = format!("{}00001", s.invoice_prefix.trim());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("First invoice number:")
                        .size(12.5)
                        .color(MUTED),
                );
                ui.label(RichText::new(sample).size(12.5).strong().color(PURPLE));
            });
        },
    );
}

fn card_bank(ui: &mut egui::Ui, s: &mut CompanySettings, submitted: bool) {
    card(
        ui,
        3,
        "Bank details",
        "Optional. Shown on invoices for payments",
        |ui| {
            let started = bank_started(s);
            text_field(
                ui,
                "Bank name",
                started,
                "e.g. State Bank of India",
                &mut s.bank_name,
                submitted,
                &|v: &str| {
                    if started {
                        required(v, "Bank name")
                    } else {
                        None
                    }
                },
            );
            text_field(
                ui,
                "Account name",
                started,
                "Name on the account",
                &mut s.bank_account_name,
                submitted,
                &|v: &str| {
                    if started {
                        required(v, "Account name")
                    } else {
                        None
                    }
                },
            );
            ui.columns(2, |cols| {
                text_field(
                    &mut cols[0],
                    "Account number",
                    started,
                    "9 to 18 digits",
                    &mut s.bank_account_number,
                    submitted,
                    &|v: &str| {
                        if started && v.trim().is_empty() {
                            required(v, "Account number")
                        } else {
                            validate_account_number(v)
                        }
                    },
                );
                let ifsc = text_field(
                    &mut cols[1],
                    "IFSC",
                    started,
                    "SBIN0001234",
                    &mut s.bank_ifsc,
                    submitted,
                    &|v: &str| {
                        if started && v.trim().is_empty() {
                            required(v, "IFSC")
                        } else {
                            validate_ifsc(v)
                        }
                    },
                );
                if ifsc.changed() {
                    s.bank_ifsc = s.bank_ifsc.to_uppercase().replace(' ', "");
                    s.bank_ifsc.truncate(11);
                }
            });
        },
    );
}

// ------------------------------------------------------------------ widgets

/// White card that fades and slides in once when the tab is opened.
fn card(
    ui: &mut egui::Ui,
    index: usize,
    title: &str,
    subtitle: &str,
    content: impl FnOnce(&mut egui::Ui),
) {
    let ctx = ui.ctx().clone();
    let now = ui.input(|i| i.time);
    let key = Id::new("aster_company_enter");
    let start = ui.data_mut(|d| *d.get_temp_mut_or_insert_with(key, || now));
    let t = if anim(&ctx) == 0.0 {
        1.0
    } else {
        (((now - start) - index as f64 * 0.06) / 0.3).clamp(0.0, 1.0) as f32
    };
    let eased = 1.0 - (1.0 - t).powi(3);
    if t < 1.0 {
        ctx.request_repaint();
    }
    ui.vertical(|ui| {
        ui.set_opacity(eased);
        ui.add_space((1.0 - eased) * 10.0);
        egui::Frame::new()
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0_f32, BORDER))
            .corner_radius(12.0)
            .inner_margin(22.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(title).size(16.5).strong().color(INK));
                ui.add_space(2.0);
                ui.label(RichText::new(subtitle).size(12.5).color(MUTED));
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(12.0);
                content(ui);
            });
    });
}

fn label_row(ui: &mut egui::Ui, label: &str, required: bool) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.label(RichText::new(label).size(12.0).strong().color(MUTED));
        if required {
            ui.label(RichText::new("*").size(12.0).strong().color(DANGER));
        } else {
            ui.label(RichText::new("(optional)").size(11.0).color(SOFT));
        }
    });
    ui.add_space(3.0);
}

fn error_line(ui: &mut egui::Ui, label: &str, error: &Option<String>, show: bool) {
    let ctx = ui.ctx().clone();
    let message_id = Id::new(("aster_co_msg", label));
    if let Some(message) = error {
        ui.data_mut(|d| d.insert_temp(message_id, message.clone()));
    }
    let t = ctx.animate_bool_with_time(Id::new(("aster_co_err", label)), show, anim(&ctx));
    if t > 0.0 {
        let message = error
            .clone()
            .or_else(|| ui.data(|d| d.get_temp::<String>(message_id)))
            .unwrap_or_default();
        ui.vertical(|ui| {
            ui.set_opacity(t);
            ui.add_space(3.0 * t);
            ui.label(RichText::new(message).size(12.0).color(DANGER));
        });
    }
}

fn text_field(
    ui: &mut egui::Ui,
    label: &str,
    required: bool,
    hint: &str,
    value: &mut String,
    submitted: bool,
    validate: &dyn Fn(&str) -> Option<String>,
) -> egui::Response {
    label_row(ui, label, required);
    let error = validate(value);
    let touched_id = Id::new(("aster_co_touched", label));
    let touched = flag(ui, touched_id);
    let show_error = error.is_some() && (submitted || touched);

    let response = ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(ui.available_width())
            .margin(Margin::symmetric(10, 8))
            .hint_text(hint),
    );
    if response.lost_focus() {
        set_flag(ui, touched_id, true);
    }
    if show_error {
        ui.painter().rect_stroke(
            response.rect,
            ui.visuals().widgets.inactive.corner_radius,
            Stroke::new(1.5_f32, DANGER),
            egui::StrokeKind::Outside,
        );
    }
    error_line(ui, label, &error, show_error);
    ui.add_space(10.0);
    response
}

fn state_picker(ui: &mut egui::Ui, s: &mut CompanySettings, submitted: bool) {
    label_row(ui, "State", true);
    let error = if s.state.trim().is_empty() {
        Some("State is required.".to_owned())
    } else {
        let gst_state = s.gstin.trim().get(0..2).unwrap_or("");
        if gst_state.len() == 2 && !s.state_code.is_empty() && gst_state != s.state_code {
            Some("GSTIN state does not match the selected state.".to_owned())
        } else {
            None
        }
    };
    let show_error = error.is_some() && (submitted || !s.state.is_empty());

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let code_w = 64.0;
        let combo_w = (ui.available_width() - code_w - 8.0).max(100.0);
        let selected = if s.state.is_empty() {
            "Select state".to_owned()
        } else {
            s.state.clone()
        };
        let before = ui.cursor().min;
        egui::ComboBox::from_id_salt("aster_company_state")
            .width(combo_w)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                for (code, name) in STATES {
                    if ui
                        .selectable_label(s.state_code == *code, format!("{code}  ·  {name}"))
                        .clicked()
                    {
                        s.state = (*name).to_owned();
                        s.state_code = (*code).to_owned();
                    }
                }
            });
        if show_error {
            let rect = Rect::from_min_size(before, Vec2::new(combo_w, 30.0));
            ui.painter().rect_stroke(
                rect.expand(1.0),
                6.0,
                Stroke::new(1.5_f32, DANGER),
                egui::StrokeKind::Outside,
            );
        }
        // read-only code chip
        let (rect, _) = ui.allocate_exact_size(Vec2::new(code_w, 30.0), Sense::hover());
        ui.painter().rect_filled(rect, 8.0, PURPLE_TINT);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            if s.state_code.is_empty() {
                "--"
            } else {
                &s.state_code
            },
            FontId::proportional(13.0),
            PURPLE,
        );
    });
    error_line(ui, "State", &error, show_error);
    ui.add_space(10.0);
}

fn button(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
    enabled: bool,
    primary: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(width, 36.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let ctx = ui.ctx().clone();
    let t = ctx.animate_bool_with_time(response.id, enabled && response.hovered(), anim(&ctx));
    let r = if enabled && response.is_pointer_button_down_on() {
        rect.shrink(0.75)
    } else {
        rect
    };
    if primary {
        let fill = if enabled {
            mix(PURPLE, PURPLE_DEEP, t)
        } else {
            Color32::from_rgb(208, 201, 222)
        };
        ui.painter().rect_filled(r, 9.0, fill);
        ui.painter().text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(13.5),
            Color32::WHITE,
        );
    } else {
        ui.painter().rect_filled(
            r,
            9.0,
            mix(Color32::WHITE, Color32::from_rgb(243, 240, 249), t),
        );
        ui.painter().rect_stroke(
            r,
            9.0,
            Stroke::new(1.0_f32, mix(BORDER, Color32::from_rgb(207, 195, 226), t)),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(13.5),
            if enabled { mix(INK, PURPLE, t) } else { SOFT },
        );
    }
    if enabled && response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}
