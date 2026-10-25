use crate::{app::AsterApp, database::CustomerRecord};
use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Id, Layout, Margin, Rect, RichText,
    ScrollArea, Sense, Stroke, Vec2, pos2, vec2,
};
use std::sync::Arc;

// ───────────────────────────── Palette ─────────────────────────────
const INK: Color32 = Color32::from_rgb(35, 32, 43);
const MUTED: Color32 = Color32::from_rgb(112, 106, 122);
const FAINT: Color32 = Color32::from_rgb(166, 160, 178);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const PURPLE_DARK: Color32 = Color32::from_rgb(74, 50, 120);
const TINT: Color32 = Color32::from_rgb(243, 239, 249);
const HOVER: Color32 = Color32::from_rgb(248, 246, 252);
const BORDER: Color32 = Color32::from_rgb(232, 228, 238);
const BORDER_STRONG: Color32 = Color32::from_rgb(214, 208, 224);
const PANEL: Color32 = Color32::WHITE;
const CANVAS: Color32 = Color32::from_rgb(250, 249, 252);
const GREEN: Color32 = Color32::from_rgb(38, 110, 72);
const GREEN_BG: Color32 = Color32::from_rgb(230, 244, 236);
const RED: Color32 = Color32::from_rgb(176, 58, 58);
const RED_BG: Color32 = Color32::from_rgb(252, 236, 236);
const AMBER: Color32 = Color32::from_rgb(170, 111, 22);
const AMBER_BG: Color32 = Color32::from_rgb(252, 243, 224);

/// Soft background / foreground pairs used for customer initials.
const AVATARS: [(Color32, Color32); 6] = [
    (
        Color32::from_rgb(237, 231, 248),
        Color32::from_rgb(91, 65, 139),
    ),
    (
        Color32::from_rgb(226, 241, 235),
        Color32::from_rgb(38, 110, 72),
    ),
    (
        Color32::from_rgb(252, 238, 220),
        Color32::from_rgb(160, 96, 16),
    ),
    (
        Color32::from_rgb(226, 236, 250),
        Color32::from_rgb(40, 90, 160),
    ),
    (
        Color32::from_rgb(250, 228, 237),
        Color32::from_rgb(170, 50, 90),
    ),
    (
        Color32::from_rgb(224, 240, 244),
        Color32::from_rgb(30, 100, 120),
    ),
];

// ───────────────────────────── Data ─────────────────────────────
#[derive(Clone, PartialEq)]
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

impl Default for Form {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            phone: String::new(),
            email: String::new(),
            gstin: String::new(),
            drug_license: String::new(),
            address_line1: String::new(),
            address_line2: String::new(),
            city: String::new(),
            state: String::new(),
            state_code: String::new(),
            pincode: String::new(),
            credit_limit: String::new(),
            opening_balance: String::new(),
            // A brand-new customer should start out active.
            active: true,
        }
    }
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

    fn to_record(&self) -> Result<CustomerRecord, String> {
        if self.name.trim().is_empty() {
            return Err("Customer name is required.".into());
        }
        let money = |raw: &str, label: &str, nonnegative: bool| -> Result<i64, String> {
            let n = if raw.trim().is_empty() {
                0.0
            } else {
                raw.trim()
                    .replace(',', "")
                    .parse::<f64>()
                    .map_err(|_| format!("Enter a valid {label}."))?
            };
            if !n.is_finite() || n.abs() > (i64::MAX / 100) as f64 {
                return Err(format!("{label} is out of range."));
            }
            if nonnegative && n < 0.0 {
                return Err(format!("{label} cannot be negative."));
            }
            Ok((n * 100.0).round() as i64)
        };
        Ok(CustomerRecord {
            id: self.id.unwrap_or_default(),
            name: self.name.trim().into(),
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
            credit_limit_paise: money(&self.credit_limit, "credit limit", true)?,
            opening_balance_paise: money(&self.opening_balance, "opening balance", false)?,
            active: self.active,
        })
    }
}

#[derive(Clone, Default)]
struct State {
    search: String,
    rows: Vec<CustomerRecord>,
    form: Form,
    /// The form as last loaded / saved, used to detect unsaved edits.
    baseline: Form,
    selected: Option<i64>,
    loaded: bool,
    show_inactive: bool,
    message: String,
    error: bool,
}

impl State {
    fn open_new(&mut self) {
        self.form = Form::default();
        self.baseline = self.form.clone();
        self.selected = None;
        self.message.clear();
        self.error = false;
    }

    fn open(&mut self, c: &CustomerRecord) {
        self.form = Form::from_record(c);
        self.baseline = self.form.clone();
        self.selected = Some(c.id);
        self.message.clear();
        self.error = false;
    }

    fn dirty(&self) -> bool {
        self.form != self.baseline
    }
}

// ───────────────────────────── Entry point ─────────────────────────────
pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let key = Id::new(("customer-master", app.selected_year));
    let mut s = ui
        .ctx()
        .data_mut(|d| d.get_temp::<State>(key).unwrap_or_default());
    if !s.loaded {
        reload(&mut s, app);
    }
    ui.scope(|ui| {
        apply_theme(ui);
        page(ui, &mut s, app);
    });
    ui.ctx().data_mut(|d| d.insert_temp(key, s));
}

/// Local style tweaks so every widget on this page shares one look.
fn apply_theme(ui: &mut egui::Ui) {
    let style = ui.style_mut();
    style.spacing.item_spacing = vec2(8.0, 6.0);
    style.spacing.button_padding = vec2(14.0, 7.0);

    let v = &mut style.visuals;
    v.extreme_bg_color = PANEL; // text-edit background
    v.selection.bg_fill = Color32::from_rgb(214, 202, 236);
    v.selection.stroke = Stroke::new(1.5, PURPLE);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_STRONG);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(184, 172, 208));
    v.widgets.inactive.corner_radius = 8.0.into();
    v.widgets.hovered.corner_radius = 8.0.into();
    v.widgets.active.corner_radius = 8.0.into();
    v.widgets.inactive.expansion = 0.0;
    v.widgets.hovered.expansion = 0.0;
    v.widgets.active.expansion = 0.0;
}

fn page(ui: &mut egui::Ui, s: &mut State, app: &AsterApp) {
    let min_h = ui.available_height().max(360.0);
    ui.set_min_height(min_h);

    // ── Page heading ──
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(
                RichText::new("MASTERS  /  ACCOUNTS RECEIVABLE")
                    .size(10.5)
                    .strong()
                    .color(PURPLE),
            );
            ui.label(
                RichText::new("Customer Master")
                    .size(28.0)
                    .strong()
                    .color(INK),
            );
            ui.label(
                RichText::new("Maintain customer profiles, tax identity and credit terms.")
                    .size(12.5)
                    .color(MUTED),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if button(ui, "+  New customer", Kind::Primary).clicked() {
                s.open_new();
            }
            if button(ui, "Refresh", Kind::Secondary).clicked() {
                reload(s, app);
            }
        });
    });
    ui.add_space(12.0);

    // ── Summary cards (equal widths, measured once) ──
    let active = s.rows.iter().filter(|c| c.active).count();
    let inactive = s.rows.len().saturating_sub(active);
    let gap = 10.0;
    let card_w = ((ui.available_width() - gap * 2.0) / 3.0).max(120.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        stat_card(
            ui,
            card_w,
            "TOTAL CUSTOMERS",
            s.rows.len().to_string(),
            "Across this financial year",
            PURPLE,
        );
        stat_card(
            ui,
            card_w,
            "ACTIVE ACCOUNTS",
            active.to_string(),
            "Available for transactions",
            GREEN,
        );
        stat_card(
            ui,
            card_w,
            "INACTIVE ACCOUNTS",
            inactive.to_string(),
            "Archived from selection",
            AMBER,
        );
    });
    ui.add_space(12.0);

    // ── Work area: directory on the left, editor on the right ──
    let available = ui.available_width();
    let editor_w = (available * 0.42).clamp(370.0, 455.0).min(available * 0.49);
    let col_gap = 14.0;
    let list_w = (available - editor_w - col_gap).max(250.0);
    let mut body_h = ui.available_height() - 4.0;
    if !body_h.is_finite() {
        body_h = 600.0;
    }
    let body_h = body_h.max(260.0);

    ui.allocate_ui_with_layout(
        vec2(available, body_h),
        Layout::left_to_right(Align::Min),
        |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.allocate_ui_with_layout(vec2(list_w, body_h), Layout::top_down(Align::Min), |ui| {
                directory(ui, s, app)
            });
            ui.add_space(col_gap);
            ui.allocate_ui_with_layout(
                vec2(editor_w, body_h),
                Layout::top_down(Align::Min),
                |ui| editor(ui, s, app),
            );
        },
    );
}

// ───────────────────────────── Directory (left) ─────────────────────────────
fn directory(ui: &mut egui::Ui, s: &mut State, app: &AsterApp) {
    card(14.0).show(ui, |ui| {
        ui.set_min_width(ui.available_width());

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Customer directory")
                    .size(16.0)
                    .strong()
                    .color(INK),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                pill(ui, format!("{} shown", visible_count(s)), TINT, PURPLE);
            });
        });
        ui.add_space(4.0);

        // Search box
        let search_id = Id::new("customer-master-search");
        let focused = ui.memory(|m| m.has_focus(search_id));
        egui::Frame::new()
            .fill(CANVAS)
            .stroke(Stroke::new(
                1.0,
                if focused { PURPLE } else { BORDER_STRONG },
            ))
            .corner_radius(9.0)
            .inner_margin(Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                    paint_search_icon(ui.painter(), r.center(), MUTED);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !s.search.is_empty()
                            && ui
                                .add(
                                    egui::Button::new(RichText::new("×").size(17.0).color(MUTED))
                                        .frame(false),
                                )
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                        {
                            s.search.clear();
                            reload(s, app);
                        }
                        let r = ui.add(
                            egui::TextEdit::singleline(&mut s.search)
                                .id(search_id)
                                .frame(false)
                                .hint_text("Search name, phone, GSTIN, city…")
                                .desired_width(f32::INFINITY),
                        );
                        if r.changed() {
                            reload(s, app);
                        }
                    });
                });
            });
        ui.add_space(2.0);

        // Filter chips
        ui.horizontal(|ui| {
            let active_n = s.rows.iter().filter(|c| c.active).count();
            let all_n = s.rows.len();
            egui::Frame::new()
                .fill(TINT)
                .corner_radius(9.0)
                .inner_margin(3.0)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if chip(ui, &format!("Active  {active_n}"), !s.show_inactive).clicked() {
                        s.show_inactive = false;
                    }
                    if chip(ui, &format!("All  {all_n}"), s.show_inactive).clicked() {
                        s.show_inactive = true;
                    }
                });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new("Click a row to edit").size(11.0).color(FAINT));
            });
        });
        ui.add_space(4.0);

        list_header(ui);
        hairline(ui);

        let idx: Vec<usize> = s
            .rows
            .iter()
            .enumerate()
            .filter(|(_, c)| s.show_inactive || c.active)
            .map(|(i, _)| i)
            .collect();

        if idx.is_empty() {
            empty_state(ui, s);
            return;
        }

        let mut clicked: Option<usize> = None;
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ScrollArea::vertical()
                .id_salt("customer-directory-scroll")
                .auto_shrink([false, false])
                .show_rows(ui, ROW_H, idx.len(), |ui, range| {
                    for i in range {
                        let c = &s.rows[idx[i]];
                        if customer_row(ui, c, s.selected == Some(c.id)).clicked() {
                            clicked = Some(idx[i]);
                        }
                    }
                });
        });
        if let Some(i) = clicked {
            let c = s.rows[i].clone();
            s.open(&c);
        }
    });
}

const ROW_H: f32 = 58.0;
const PAD: f32 = 12.0;
const AVATAR: f32 = 36.0;

/// Column geometry shared by the header and every row so they always line up.
struct Cols {
    text_x: f32,
    name_w: f32,
    city_x: f32,
    city_w: f32,
    show_city: bool,
    status_x: f32,
    status_w: f32,
}

impl Cols {
    fn new(w: f32) -> Self {
        let status_w = 70.0;
        let status_x = w - PAD - status_w;
        let show_city = w >= 470.0;
        let city_w = if show_city {
            (w * 0.24).clamp(90.0, 170.0)
        } else {
            0.0
        };
        let city_x = status_x - 14.0 - city_w;
        let text_x = PAD + AVATAR + 12.0;
        let right = if show_city { city_x } else { status_x };
        Self {
            text_x,
            name_w: (right - 14.0 - text_x).max(40.0),
            city_x,
            city_w,
            show_city,
            status_x,
            status_w,
        }
    }
}

fn list_header(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
    let c = Cols::new(r.width());
    let p = ui.painter();
    let font = FontId::proportional(10.0);
    p.text(
        pos2(r.left() + c.text_x, r.center().y),
        Align2::LEFT_CENTER,
        "CUSTOMER",
        font.clone(),
        MUTED,
    );
    if c.show_city {
        p.text(
            pos2(r.left() + c.city_x, r.center().y),
            Align2::LEFT_CENTER,
            "CITY",
            font.clone(),
            MUTED,
        );
    }
    p.text(
        pos2(r.left() + c.status_x + c.status_w / 2.0, r.center().y),
        Align2::CENTER_CENTER,
        "STATUS",
        font,
        MUTED,
    );
}

fn customer_row(ui: &mut egui::Ui, c: &CustomerRecord, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let cols = Cols::new(rect.width());
    let p = ui.painter();

    // Background + selection accent
    let inner = rect.shrink2(vec2(0.0, 1.0));
    if selected {
        p.rect_filled(inner, 9.0, TINT);
        p.rect_filled(
            Rect::from_min_size(
                pos2(rect.left(), rect.top() + 12.0),
                vec2(3.0, ROW_H - 24.0),
            ),
            2.0,
            PURPLE,
        );
    } else if resp.hovered() {
        p.rect_filled(inner, 9.0, HOVER);
    }

    // Avatar
    let av = Rect::from_center_size(
        pos2(rect.left() + PAD + AVATAR / 2.0, rect.center().y),
        Vec2::splat(AVATAR),
    );
    paint_avatar(p, av, &c.name);

    // Name + phone
    let name_col = if c.active { INK } else { MUTED };
    let g = text_galley(ui, &c.name, 14.0, name_col, cols.name_w);
    p.galley(
        pos2(rect.left() + cols.text_x, rect.top() + 11.0),
        g,
        name_col,
    );
    let (phone, phone_col) = if c.phone.is_empty() {
        ("No phone", FAINT)
    } else {
        (c.phone.as_str(), MUTED)
    };
    let g = text_galley(ui, phone, 11.5, phone_col, cols.name_w);
    p.galley(
        pos2(rect.left() + cols.text_x, rect.top() + 32.0),
        g,
        phone_col,
    );

    // City + GSTIN
    if cols.show_city {
        let (city, city_col) = if c.city.is_empty() {
            ("—", FAINT)
        } else {
            (c.city.as_str(), INK)
        };
        let g = text_galley(ui, city, 12.5, city_col, cols.city_w);
        p.galley(
            pos2(rect.left() + cols.city_x, rect.top() + 11.0),
            g,
            city_col,
        );
        let (gst, gst_col) = if c.gstin.is_empty() {
            ("No GSTIN", FAINT)
        } else {
            (c.gstin.as_str(), MUTED)
        };
        let g = text_galley(ui, gst, 10.5, gst_col, cols.city_w);
        p.galley(
            pos2(rect.left() + cols.city_x, rect.top() + 33.0),
            g,
            gst_col,
        );
    }

    // Status pill
    let pr = Rect::from_center_size(
        pos2(
            rect.left() + cols.status_x + cols.status_w / 2.0,
            rect.center().y,
        ),
        vec2(cols.status_w, 22.0),
    );
    let (bg, fg, txt) = if c.active {
        (GREEN_BG, GREEN, "ACTIVE")
    } else {
        (RED_BG, RED, "INACTIVE")
    };
    p.rect_filled(pr, 11.0, bg);
    p.text(
        pr.center(),
        Align2::CENTER_CENTER,
        txt,
        FontId::proportional(9.5),
        fg,
    );

    resp
}

fn empty_state(ui: &mut egui::Ui, s: &State) {
    let (title, sub) = if !s.search.trim().is_empty() {
        (
            "No customers match your search",
            "Try a different name, phone, GSTIN or city.",
        )
    } else if !s.rows.is_empty() {
        (
            "No active customers",
            "Switch to “All” to see inactive accounts.",
        )
    } else {
        (
            "No customers yet",
            "Use the form on the right to add your first customer.",
        )
    };
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
        ui.painter().circle_filled(r.center(), 24.0, TINT);
        paint_search_icon(ui.painter(), r.center(), PURPLE);
        ui.add_space(6.0);
        ui.label(RichText::new(title).size(14.0).strong().color(INK));
        ui.label(RichText::new(sub).size(12.0).color(MUTED));
    });
}

// ───────────────────────────── Editor (right) ─────────────────────────────
fn editor(ui: &mut egui::Ui, s: &mut State, app: &AsterApp) {
    card(16.0).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        let existing = s.form.id.is_some();
        let dirty = s.dirty();

        // Header: avatar, title, status
        ui.horizontal(|ui| {
            avatar(ui, &s.form.name, 40.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                let reserve = if existing && dirty { 190.0 } else { 100.0 };
                ui.set_max_width((ui.available_width() - reserve).max(80.0));
                let name = s.form.name.trim();
                ui.add(
                    egui::Label::new(
                        RichText::new(if name.is_empty() {
                            "New customer"
                        } else {
                            name
                        })
                        .size(16.5)
                        .strong()
                        .color(INK),
                    )
                    .truncate(),
                );
                let sub = match s.form.id {
                    Some(id) => format!("Customer record  ·  ID {id}"),
                    None => "Fill in the details below".to_owned(),
                };
                ui.label(RichText::new(sub).size(11.5).color(MUTED));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if existing {
                    let (bg, fg, t) = if s.form.active {
                        (GREEN_BG, GREEN, "ACTIVE")
                    } else {
                        (RED_BG, RED, "INACTIVE")
                    };
                    pill(ui, t.into(), bg, fg);
                    if dirty {
                        pill(ui, "UNSAVED".into(), AMBER_BG, AMBER);
                    }
                } else {
                    pill(ui, "DRAFT".into(), TINT, PURPLE);
                }
            });
        });
        ui.add_space(8.0);
        hairline(ui);
        ui.add_space(4.0);

        let footer_h = 58.0 + if s.message.is_empty() { 0.0 } else { 44.0 };
        ScrollArea::vertical()
            .id_salt("customer-editor-scroll")
            .auto_shrink([false, false])
            .max_height((ui.available_height() - footer_h).max(150.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                let name_invalid = s.error && s.form.name.trim().is_empty();

                section(ui, "BASIC INFORMATION", "Identity and primary contact");
                field_with(
                    ui,
                    "Customer / business name",
                    &mut s.form.name,
                    "e.g. Sri Lakshmi Medicals",
                    Opts {
                        required: true,
                        invalid: name_invalid,
                        ..Opts::default()
                    },
                );
                ui.columns(2, |c| {
                    field(
                        &mut c[0],
                        "Phone number",
                        &mut s.form.phone,
                        "10-digit mobile",
                    );
                    field(
                        &mut c[1],
                        "Email address",
                        &mut s.form.email,
                        "name@example.com",
                    );
                });

                section(ui, "TAX & LICENCE", "For invoicing and compliance");
                let g = s.form.gstin.trim().chars().count();
                let gst_note = format!("GSTIN has {g} of 15 characters");
                field_with(
                    ui,
                    "GSTIN",
                    &mut s.form.gstin,
                    "15-character GST identification number",
                    Opts {
                        note: (g > 0 && g != 15).then_some(gst_note.as_str()),
                        ..Opts::default()
                    },
                );
                field(
                    ui,
                    "Drug licence number",
                    &mut s.form.drug_license,
                    "Optional licence reference",
                );

                section(ui, "BILLING ADDRESS", "Customer location details");
                field(
                    ui,
                    "Address line 1",
                    &mut s.form.address_line1,
                    "Door no., street, area",
                );
                field(
                    ui,
                    "Address line 2",
                    &mut s.form.address_line2,
                    "Landmark / additional address",
                );
                ui.columns(2, |c| {
                    field(&mut c[0], "City / town", &mut s.form.city, "e.g. Cuddalore");
                    field(&mut c[1], "State", &mut s.form.state, "e.g. Tamil Nadu");
                });
                ui.columns(2, |c| {
                    field(&mut c[0], "State code", &mut s.form.state_code, "e.g. 33");
                    field(&mut c[1], "PIN code", &mut s.form.pincode, "6 digits");
                });

                section(
                    ui,
                    "CREDIT & OPENING BALANCE",
                    "Account setup for this financial year",
                );
                ui.columns(2, |c| {
                    field_with(
                        &mut c[0],
                        "Credit limit (₹)",
                        &mut s.form.credit_limit,
                        "0.00",
                        Opts {
                            right: true,
                            ..Opts::default()
                        },
                    );
                    field_with(
                        &mut c[1],
                        "Opening balance (₹)",
                        &mut s.form.opening_balance,
                        "0.00",
                        Opts {
                            right: true,
                            ..Opts::default()
                        },
                    );
                });
                if existing {
                    ui.add_space(2.0);
                    ui.checkbox(&mut s.form.active, "Customer account is active");
                }
                ui.add_space(6.0);
            });

        // Status banner
        if !s.message.is_empty() {
            ui.add_space(4.0);
            let (bg, fg) = if s.error {
                (RED_BG, RED)
            } else {
                (GREEN_BG, GREEN)
            };
            let mut dismiss = false;
            egui::Frame::new()
                .fill(bg)
                .corner_radius(8.0)
                .inner_margin(Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("×").size(16.0).color(fg))
                                    .frame(false),
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            dismiss = true;
                        }
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                            ui.painter().circle_filled(r.center(), 4.0, fg);
                            ui.add(
                                egui::Label::new(RichText::new(&s.message).size(12.5).color(fg))
                                    .truncate(),
                            );
                        });
                    });
                });
            if dismiss {
                s.message.clear();
                s.error = false;
            }
        }

        // Footer actions
        ui.add_space(6.0);
        hairline(ui);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let label = if existing {
                "Save changes"
            } else {
                "Save customer"
            };
            if button(ui, label, Kind::Primary).clicked() {
                save(s, app);
            }
            if button(ui, "Clear", Kind::Secondary).clicked() {
                s.open_new();
            }
            if existing {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (label, kind) = if s.form.active {
                        ("Deactivate", Kind::Danger)
                    } else {
                        ("Activate", Kind::Secondary)
                    };
                    if button(ui, label, kind).clicked() {
                        toggle_active(s, app);
                    }
                });
            }
        });
    });
}

fn save(s: &mut State, app: &AsterApp) {
    let record = match s.form.to_record() {
        Ok(r) => r,
        Err(e) => {
            s.message = e;
            s.error = true;
            return;
        }
    };
    let result = app
        .db
        .as_ref()
        .ok_or_else(|| "Database is not open.".to_owned())
        .and_then(|db| db.save_customer(app.selected_year, s.form.id, &record));
    match result {
        Ok(()) => {
            reload(s, app);
            let found = s
                .rows
                .iter()
                .find(|c| c.name.eq_ignore_ascii_case(&record.name) && c.phone == record.phone)
                .cloned();
            match found {
                Some(c) => s.open(&c),
                None => s.baseline = s.form.clone(),
            }
            s.message = "Customer saved successfully.".into();
            s.error = false;
        }
        Err(e) => {
            s.message = e;
            s.error = true;
        }
    }
}

fn toggle_active(s: &mut State, app: &AsterApp) {
    let Some(id) = s.form.id else { return };
    let target = !s.form.active;
    let result = app
        .db
        .as_ref()
        .ok_or_else(|| "Database is not open.".to_owned())
        .and_then(|db| db.set_customer_active(app.selected_year, id, target));
    match result {
        Ok(()) => {
            s.form.active = target;
            s.baseline.active = target;
            reload(s, app);
            s.message = if target {
                "Customer activated."
            } else {
                "Customer deactivated."
            }
            .into();
            s.error = false;
        }
        Err(e) => {
            s.message = e;
            s.error = true;
        }
    }
}

// ───────────────────────────── Form widgets ─────────────────────────────
fn section(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(3.0, 13.0), Sense::hover());
        ui.painter().rect_filled(r, 2.0, PURPLE);
        ui.label(RichText::new(title).size(10.5).strong().color(INK));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(subtitle).size(10.5).color(FAINT));
        });
    });
    ui.add_space(2.0);
    hairline(ui);
    ui.add_space(4.0);
}

#[derive(Default)]
struct Opts<'a> {
    required: bool,
    invalid: bool,
    right: bool,
    note: Option<&'a str>,
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    field_with(ui, label, value, hint, Opts::default());
}

fn field_with(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str, o: Opts) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.label(RichText::new(label).size(11.5).strong().color(MUTED));
        if o.required {
            ui.label(RichText::new("*").size(11.5).color(RED));
        }
    });
    ui.scope(|ui| {
        if o.invalid {
            let v = ui.visuals_mut();
            v.widgets.inactive.bg_stroke = Stroke::new(1.0, RED);
            v.widgets.hovered.bg_stroke = Stroke::new(1.0, RED);
            v.selection.stroke = Stroke::new(1.5, RED);
        }
        ui.add(
            egui::TextEdit::singleline(value)
                .hint_text(hint)
                .desired_width(f32::INFINITY)
                .margin(Margin::symmetric(10, 7))
                .horizontal_align(if o.right { Align::RIGHT } else { Align::LEFT }),
        );
    });
    if let Some(n) = o.note {
        ui.label(RichText::new(n).size(11.0).color(AMBER));
    }
    ui.add_space(4.0);
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Primary,
    Secondary,
    Danger,
}

fn button(ui: &mut egui::Ui, label: &str, kind: Kind) -> egui::Response {
    ui.scope(|ui| {
        // (fill, hover fill, text, stroke, hover stroke)
        let (fill, hover, fg, stroke, hover_stroke) = match kind {
            Kind::Primary => (
                PURPLE,
                PURPLE_DARK,
                Color32::WHITE,
                Stroke::NONE,
                Stroke::NONE,
            ),
            Kind::Secondary => (
                PANEL,
                TINT,
                INK,
                Stroke::new(1.0, BORDER_STRONG),
                Stroke::new(1.0, Color32::from_rgb(184, 172, 208)),
            ),
            Kind::Danger => (
                PANEL,
                RED_BG,
                RED,
                Stroke::new(1.0, Color32::from_rgb(232, 196, 196)),
                Stroke::new(1.0, RED),
            ),
        };
        // egui buttons paint with `weak_bg_fill`, not `bg_fill`.
        let v = ui.visuals_mut();
        v.widgets.inactive.weak_bg_fill = fill;
        v.widgets.hovered.weak_bg_fill = hover;
        v.widgets.active.weak_bg_fill = hover;
        v.widgets.inactive.bg_stroke = stroke;
        v.widgets.hovered.bg_stroke = hover_stroke;
        v.widgets.active.bg_stroke = hover_stroke;
        ui.add(
            egui::Button::new(RichText::new(label).size(13.0).strong().color(fg))
                .min_size(vec2(0.0, 34.0))
                .corner_radius(8.0),
        )
        .on_hover_cursor(CursorIcon::PointingHand)
    })
    .inner
}

fn chip(ui: &mut egui::Ui, text: &str, on: bool) -> egui::Response {
    let (fill, fg) = if on {
        (PANEL, PURPLE)
    } else {
        (Color32::TRANSPARENT, MUTED)
    };
    ui.add(
        egui::Button::new(RichText::new(text).size(12.0).strong().color(fg))
            .fill(fill)
            .stroke(Stroke::NONE)
            .corner_radius(7.0),
    )
    .on_hover_cursor(CursorIcon::PointingHand)
}

// ───────────────────────────── Small pieces ─────────────────────────────
fn card(margin: f32) -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(12.0)
        .inner_margin(margin)
}

fn stat_card(
    ui: &mut egui::Ui,
    w: f32,
    label: &str,
    value: String,
    caption: &str,
    accent: Color32,
) {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10.0)
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            // outer width = inner + 2×14 margin + 2×1 stroke
            ui.set_width((w - 30.0).max(60.0));
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 4.0, accent);
                ui.label(RichText::new(label).size(10.0).strong().color(MUTED));
            });
            ui.label(RichText::new(value).size(26.0).strong().color(INK));
            ui.label(RichText::new(caption).size(11.0).color(MUTED));
        });
}

fn pill(ui: &mut egui::Ui, text: String, bg: Color32, fg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(20.0)
        .inner_margin(Margin::symmetric(9, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(9.5).strong().color(fg));
        });
}

fn hairline(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, BORDER);
}

fn avatar(ui: &mut egui::Ui, name: &str, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint_avatar(ui.painter(), rect, name);
}

fn paint_avatar(p: &egui::Painter, rect: Rect, name: &str) {
    let radius = rect.width() / 2.0;
    if name.trim().is_empty() {
        p.circle_filled(rect.center(), radius, TINT);
        p.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "+",
            FontId::proportional(rect.width() * 0.55),
            PURPLE,
        );
        return;
    }
    let hash = name
        .bytes()
        .fold(0usize, |a, b| a.wrapping_mul(31).wrapping_add(b as usize));
    let (bg, fg) = AVATARS[hash % AVATARS.len()];
    p.circle_filled(rect.center(), radius, bg);
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        initials(name),
        FontId::proportional(rect.width() * 0.38),
        fg,
    );
}

fn initials(name: &str) -> String {
    let s: String = name
        .split_whitespace()
        .filter_map(|w| w.chars().find(|c| c.is_alphanumeric()))
        .take(2)
        .collect::<String>()
        .to_uppercase();
    if s.is_empty() { "?".into() } else { s }
}

/// Magnifier drawn with strokes so it doesn't depend on font glyph coverage.
fn paint_search_icon(p: &egui::Painter, center: egui::Pos2, color: Color32) {
    let c = center + vec2(-1.5, -1.5);
    p.circle_stroke(c, 5.0, Stroke::new(1.6, color));
    p.line_segment(
        [c + Vec2::splat(3.6), c + Vec2::splat(7.4)],
        Stroke::new(1.8, color),
    );
}

/// Single-line text that ends in “…” instead of overflowing its column.
fn text_galley(
    ui: &egui::Ui,
    text: &str,
    size: f32,
    color: Color32,
    max_w: f32,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(
        text.to_owned(),
        FontId::proportional(size),
        color,
    );
    job.wrap.max_width = max_w.max(1.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    ui.painter().layout_job(job)
}

// ───────────────────────────── Data helpers ─────────────────────────────
fn visible_count(s: &State) -> usize {
    s.rows
        .iter()
        .filter(|c| s.show_inactive || c.active)
        .count()
}

fn reload(s: &mut State, app: &AsterApp) {
    s.loaded = true;
    if let Some(db) = app.db.as_ref() {
        match db.customers(app.selected_year, &s.search) {
            Ok(rows) => {
                s.rows = rows;
                s.error = false;
            }
            Err(e) => {
                s.message = e;
                s.error = true;
                s.rows.clear();
            }
        }
    } else {
        s.rows.clear();
    }
}
