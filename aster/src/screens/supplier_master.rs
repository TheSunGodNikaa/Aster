use crate::{
    app::AsterApp,
    database::{ManufacturerOption, ProductRecord, SupplierProductLink, SupplierRecord},
};
use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Id, Key, Layout, Margin, Modifiers, Rect,
    RichText, ScrollArea, Sense, Stroke, Vec2, pos2, vec2,
};
use std::ops::Range;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

const TOAST_LIFE_OK: f64 = 3.5;
const TOAST_LIFE_ERR: f64 = 5.0;

// Layout metrics
const MIN_PANE_H: f32 = 560.0; // panes never get shorter than this; the page scrolls instead
const STACK_BELOW: f32 = 940.0; // below this width the two panes stack vertically
const STACKED_DIR_H: f32 = 460.0;
const STACKED_EDITOR_H: f32 = 720.0;
const COL_GAP: f32 = 14.0;

const ROW_H: f32 = 56.0;
const PAD: f32 = 12.0;
const AVATAR: f32 = 36.0;
const STATUS_W: f32 = 70.0;
const CHECK_ROW_H: f32 = 28.0;

// ───────────────────────────── Data ─────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldId {
    Name,
    Phone,
    Email,
    Gstin,
    StateCode,
    Pincode,
    Terms,
    Credit,
    Opening,
}

struct FormError {
    field: FieldId,
    msg: String,
}

fn ferr(field: FieldId, msg: impl Into<String>) -> FormError {
    FormError {
        field,
        msg: msg.into(),
    }
}

#[derive(Clone, Default, PartialEq)]
struct Form {
    id: Option<i64>,
    name: String,
    trade_name: String,
    contact_person: String,
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
    payment_terms_days: String,
    credit_limit: String,
    opening_balance: String,
    active: bool,
}

fn parse_money(v: &str, field: FieldId, label: &str, nonnegative: bool) -> Result<i64, FormError> {
    let n = if v.trim().is_empty() {
        0.0
    } else {
        v.trim()
            .replace(',', "")
            .parse::<f64>()
            .map_err(|_| ferr(field, format!("Enter a valid {label}.")))?
    };
    if !n.is_finite() || n.abs() > (i64::MAX / 100) as f64 {
        return Err(ferr(field, format!("The {label} is out of range.")));
    }
    if nonnegative && n < 0.0 {
        return Err(ferr(field, format!("The {label} cannot be negative.")));
    }
    Ok((n * 100.0).round() as i64)
}

impl Form {
    fn from_record(r: &SupplierRecord) -> Self {
        Self {
            id: Some(r.id),
            name: r.name.clone(),
            trade_name: r.trade_name.clone(),
            contact_person: r.contact_person.clone(),
            phone: r.phone.clone(),
            email: r.email.clone(),
            gstin: r.gstin.clone(),
            drug_license: r.drug_license.clone(),
            address_line1: r.address_line1.clone(),
            address_line2: r.address_line2.clone(),
            city: r.city.clone(),
            state: r.state.clone(),
            state_code: r.state_code.clone(),
            pincode: r.pincode.clone(),
            payment_terms_days: r.payment_terms_days.to_string(),
            credit_limit: format!("{:.2}", r.credit_limit_paise as f64 / 100.0),
            opening_balance: format!("{:.2}", r.opening_balance_paise as f64 / 100.0),
            active: r.active,
        }
    }

    fn to_record(&self) -> Result<SupplierRecord, FormError> {
        if self.name.trim().is_empty() {
            return Err(ferr(FieldId::Name, "Supplier legal name is required."));
        }

        let phone = self.phone.trim();
        if !phone.is_empty() {
            let digits = phone.chars().filter(|c| c.is_ascii_digit()).count();
            let chars_ok = phone
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '+' | ' ' | '-' | '(' | ')'));
            if !chars_ok || !(7..=15).contains(&digits) {
                return Err(ferr(
                    FieldId::Phone,
                    "Enter a valid phone number (7 to 15 digits).",
                ));
            }
        }

        let email = self.email.trim();
        if !email.is_empty() {
            let valid = !email.contains(char::is_whitespace)
                && match email.split_once('@') {
                    Some((local, domain)) => {
                        !local.is_empty()
                            && !domain.contains('@')
                            && domain.contains('.')
                            && !domain.starts_with('.')
                            && !domain.ends_with('.')
                    }
                    None => false,
                };
            if !valid {
                return Err(ferr(FieldId::Email, "Enter a valid email address."));
            }
        }

        let gstin = self.gstin.trim().to_uppercase();
        if !gstin.is_empty()
            && (gstin.chars().count() != 15 || !gstin.chars().all(|c| c.is_ascii_alphanumeric()))
        {
            return Err(ferr(
                FieldId::Gstin,
                "GSTIN must be exactly 15 letters or digits.",
            ));
        }

        // Derive the GST state code from the GSTIN when it was left blank.
        let mut state_code = self.state_code.trim().to_owned();
        if state_code.is_empty() {
            if let Some(prefix) = gstin.get(..2) {
                if prefix.chars().all(|c| c.is_ascii_digit()) {
                    state_code = prefix.to_owned();
                }
            }
        }
        if !state_code.is_empty()
            && !(state_code.len() == 2 && state_code.chars().all(|c| c.is_ascii_digit()))
        {
            return Err(ferr(
                FieldId::StateCode,
                "GST state code must be two digits.",
            ));
        }

        let pincode = self.pincode.trim();
        if !pincode.is_empty() && !(pincode.len() == 6 && pincode.chars().all(|c| c.is_ascii_digit()))
        {
            return Err(ferr(FieldId::Pincode, "PIN code must be 6 digits."));
        }

        let days = if self.payment_terms_days.trim().is_empty() {
            0
        } else {
            self.payment_terms_days.trim().parse::<i64>().map_err(|_| {
                ferr(
                    FieldId::Terms,
                    "Payment terms must be a whole number of days.",
                )
            })?
        };
        if !(0..=3650).contains(&days) {
            return Err(ferr(
                FieldId::Terms,
                "Payment terms must be between 0 and 3650 days.",
            ));
        }

        let credit = parse_money(&self.credit_limit, FieldId::Credit, "credit limit", true)?;
        let opening = parse_money(
            &self.opening_balance,
            FieldId::Opening,
            "opening balance",
            false,
        )?;

        Ok(SupplierRecord {
            id: self.id.unwrap_or_default(),
            name: self.name.trim().into(),
            trade_name: self.trade_name.trim().into(),
            contact_person: self.contact_person.trim().into(),
            phone: phone.into(),
            email: email.into(),
            gstin,
            drug_license: self.drug_license.trim().into(),
            address_line1: self.address_line1.trim().into(),
            address_line2: self.address_line2.trim().into(),
            city: self.city.trim().into(),
            state: self.state.trim().into(),
            state_code,
            pincode: pincode.into(),
            payment_terms_days: days,
            credit_limit_paise: credit,
            opening_balance_paise: opening,
            active: self.active,
        })
    }
}

struct FieldError {
    field: FieldId,
    msg: String,
    /// Form contents when the error was raised; editing anything clears the error.
    snapshot: Form,
}

/// Navigation that was blocked by unsaved changes and awaits confirmation.
#[derive(Clone, Copy)]
enum Nav {
    New,
    Open(i64),
}

#[derive(Default)]
struct State {
    loaded: bool,
    frames: u32,
    search: String,
    rows: Vec<SupplierRecord>,
    form: Form,
    baseline: Form,
    base_mfr: Vec<i64>,
    base_prod: Vec<i64>,
    selected: Option<i64>,
    show_inactive: bool,
    manufacturers: Vec<ManufacturerOption>,
    products: Vec<ProductRecord>,
    manufacturer_ids: Vec<i64>,
    product_ids: Vec<i64>,
    new_manufacturer_name: String,
    product_filter: String,
    product_selected_only: bool,
    /// Set when the supplier's links could not be read; saving is blocked so
    /// a failed read can never wipe existing links.
    links_failed: bool,
    err: Option<FieldError>,
    err_fresh: bool,
    pending: Option<Nav>,
    pending_at: f64,
    confirm_deactivate: bool,
    swap_at: f64,
    reset_scroll: bool,
    // toast
    message: String,
    error: bool,
    toast_at: f64,
}

fn sorted(v: &[i64]) -> Vec<i64> {
    let mut x = v.to_vec();
    x.sort_unstable();
    x
}

impl State {
    fn new_form(&mut self, now: f64) {
        self.form = Form {
            active: true,
            ..Form::default()
        };
        self.baseline = self.form.clone();
        self.selected = None;
        self.manufacturer_ids.clear();
        self.product_ids.clear();
        self.base_mfr.clear();
        self.base_prod.clear();
        self.new_manufacturer_name.clear();
        self.product_filter.clear();
        self.product_selected_only = false;
        self.links_failed = false;
        self.err = None;
        self.err_fresh = false;
        self.pending = None;
        self.confirm_deactivate = false;
        self.swap_at = now;
        self.reset_scroll = true;
        self.message.clear();
        self.error = false;
        self.toast_at = 0.0;
    }

    fn open(&mut self, r: &SupplierRecord, app: &AsterApp, now: f64) {
        self.form = Form::from_record(r);
        self.baseline = self.form.clone();
        self.selected = Some(r.id);
        self.new_manufacturer_name.clear();
        self.product_filter.clear();
        self.product_selected_only = false;
        self.err = None;
        self.err_fresh = false;
        self.pending = None;
        self.confirm_deactivate = false;
        self.swap_at = now;
        self.reset_scroll = true;
        self.message.clear();
        self.error = false;
        self.toast_at = 0.0;
        self.links_failed = false;
        if let Some(db) = app.db.as_ref() {
            let mfr = db.supplier_manufacturer_ids(app.selected_year, r.id);
            let prod = db.supplier_product_links(app.selected_year, r.id);
            self.links_failed = mfr.is_err() || prod.is_err();
            self.manufacturer_ids = mfr.unwrap_or_default();
            self.product_ids = prod
                .unwrap_or_default()
                .into_iter()
                .map(|p| p.product_id)
                .collect();
        } else {
            self.manufacturer_ids.clear();
            self.product_ids.clear();
        }
        self.base_mfr = sorted(&self.manufacturer_ids);
        self.base_prod = sorted(&self.product_ids);
        if self.links_failed {
            self.toast(
                "Could not load this supplier's links. Reopen it before saving.",
                true,
                now,
            );
        }
    }

    fn dirty(&self) -> bool {
        self.form != self.baseline
            || sorted(&self.manufacturer_ids) != self.base_mfr
            || sorted(&self.product_ids) != self.base_prod
    }

    fn toast(&mut self, msg: impl Into<String>, err: bool, now: f64) {
        self.message = msg.into();
        self.error = err;
        self.toast_at = now;
    }

    fn dismiss_toast(&mut self) {
        self.message.clear();
        self.error = false;
        self.toast_at = 0.0;
    }

    fn toast_age(&self, now: f64) -> f64 {
        if self.message.is_empty() {
            f64::INFINITY
        } else {
            now - self.toast_at
        }
    }

    fn toast_ttl(&self) -> f64 {
        if self.error {
            TOAST_LIFE_ERR
        } else {
            TOAST_LIFE_OK
        }
    }
}

// ───────────────────────────── Entry point ─────────────────────────────
pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let key = Id::new(("supplier-master", app.selected_year));
    // State lives behind an Arc so it is not deep-cloned on every frame.
    let cell: Arc<Mutex<State>> = ui.ctx().data_mut(|d| {
        d.get_temp_mut_or_insert_with(key, || Arc::new(Mutex::new(State::default())))
            .clone()
    });
    let mut guard = cell.lock().unwrap_or_else(|p| p.into_inner());
    let s: &mut State = &mut guard;

    let now = ui.ctx().input(|i| i.time);
    if !s.loaded {
        s.new_form(now);
        reload_all(s, app, now);
    }

    // First frame renders at 0, the next one animates to 1 (a fade-in).
    let intro = ease(
        ui.ctx(),
        Id::new(("supplier-master-intro", app.selected_year)),
        s.frames > 0,
    );
    if s.frames == 0 {
        ui.ctx().request_repaint();
    }
    s.frames = s.frames.saturating_add(1);

    ui.scope(|ui| {
        apply_theme(ui);
        ui.set_opacity(0.15 + 0.85 * intro);
        page(ui, s, app, now);
    });
}

fn apply_theme(ui: &mut egui::Ui) {
    let style = ui.style_mut();
    style.spacing.item_spacing = vec2(8.0, 6.0);
    style.spacing.button_padding = vec2(14.0, 7.0);
    let v = &mut style.visuals;
    v.extreme_bg_color = PANEL;
    v.selection.bg_fill = Color32::from_rgb(214, 202, 236);
    v.selection.stroke = Stroke::new(1.5_f32, PURPLE);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_STRONG);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(184, 172, 208));
    v.widgets.inactive.corner_radius = 8.0.into();
    v.widgets.hovered.corner_radius = 8.0.into();
    v.widgets.active.corner_radius = 8.0.into();
    v.widgets.inactive.expansion = 0.0;
    v.widgets.hovered.expansion = 0.0;
    v.widgets.active.expansion = 0.0;
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Eased 0..=1 value driven by egui's animation clock.
/// Note: the first time an id is seen it snaps to its target.
fn ease(ctx: &egui::Context, id: Id, on: bool) -> f32 {
    smooth(ctx.animate_bool_with_time(id, on, 0.2))
}

// ───────────────────────────── Page ─────────────────────────────
fn request_nav(s: &mut State, app: &AsterApp, nav: Nav, now: f64) {
    if let Nav::Open(id) = nav {
        if s.selected == Some(id) {
            return;
        }
    }
    if s.dirty() {
        s.pending = Some(nav);
        s.pending_at = now;
    } else {
        apply_nav(s, app, nav, now);
    }
}

fn apply_nav(s: &mut State, app: &AsterApp, nav: Nav, now: f64) {
    match nav {
        Nav::New => s.new_form(now),
        Nav::Open(id) => {
            if let Some(r) = s.rows.iter().find(|r| r.id == id).cloned() {
                s.open(&r, app, now);
            } else {
                s.pending = None;
            }
        }
    }
}

fn page(ui: &mut egui::Ui, s: &mut State, app: &AsterApp, now: f64) {
    // Ctrl/Cmd + S saves.
    if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::S)) {
        save(s, app, now);
        ui.ctx().request_repaint();
    }

    let mut viewport_h = ui.available_height();
    if !viewport_h.is_finite() {
        viewport_h = 720.0;
    }

    // The whole page scrolls, so nothing is ever cut off on small windows.
    ScrollArea::vertical()
        .id_salt("supplier-master-page")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let top = ui.cursor().top();
            header(ui, s, app, now);
            ui.add_space(12.0);
            stats(ui, s);
            ui.add_space(12.0);
            let used = ui.cursor().top() - top;

            let w = ui.available_width();
            let pane_h = (viewport_h - used - 6.0).max(MIN_PANE_H);

            if w < STACK_BELOW {
                ui.allocate_ui_with_layout(
                    vec2(w, STACKED_DIR_H),
                    Layout::top_down(Align::Min),
                    |ui| directory(ui, s, app, STACKED_DIR_H, now),
                );
                ui.add_space(COL_GAP);
                let eh = pane_h.max(STACKED_EDITOR_H);
                ui.allocate_ui_with_layout(vec2(w, eh), Layout::top_down(Align::Min), |ui| {
                    editor(ui, s, app, eh, now)
                });
            } else {
                let right = (w * 0.48).clamp(440.0, 680.0);
                let left = (w - right - COL_GAP).max(320.0);
                ui.allocate_ui_with_layout(
                    vec2(w, pane_h),
                    Layout::left_to_right(Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.allocate_ui_with_layout(
                            vec2(left, pane_h),
                            Layout::top_down(Align::Min),
                            |ui| directory(ui, s, app, pane_h, now),
                        );
                        ui.add_space(COL_GAP);
                        ui.allocate_ui_with_layout(
                            vec2(right, pane_h),
                            Layout::top_down(Align::Min),
                            |ui| editor(ui, s, app, pane_h, now),
                        );
                    },
                );
            }
            ui.add_space(10.0);
        });
}

fn header(ui: &mut egui::Ui, s: &mut State, app: &AsterApp, now: f64) {
    ui.horizontal(|ui| {
        ui.set_min_height(58.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(
                RichText::new("MASTERS  /  ACCOUNTS PAYABLE")
                    .size(10.5)
                    .strong()
                    .color(PURPLE),
            );
            ui.label(
                RichText::new("Supplier Master")
                    .size(28.0)
                    .strong()
                    .color(INK),
            );
            ui.label(
                RichText::new("Manage distributors, manufacturers and the medicines they supply.")
                    .size(12.5)
                    .color(MUTED),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if button(ui, "+  New supplier", Kind::Primary).clicked() {
                request_nav(s, app, Nav::New, now);
                ui.ctx().request_repaint();
            }
            if button(ui, "Refresh", Kind::Secondary).clicked() {
                reload_all(s, app, now);
                s.toast("Directory refreshed.", false, now);
                ui.ctx().request_repaint();
            }
        });
    });
}

fn stats(ui: &mut egui::Ui, s: &State) {
    let active = s.rows.iter().filter(|r| r.active).count();
    let inactive = s.rows.len().saturating_sub(active);
    let gap = 10.0;
    let card_w = ((ui.available_width() - gap * 2.0) / 3.0).max(120.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        stat_card(
            ui,
            card_w,
            "TOTAL SUPPLIERS",
            s.rows.len().to_string(),
            PURPLE,
        );
        stat_card(ui, card_w, "ACTIVE", active.to_string(), GREEN);
        stat_card(ui, card_w, "INACTIVE", inactive.to_string(), AMBER);
    });
}

// ───────────────────────────── Directory ─────────────────────────────
fn directory(ui: &mut egui::Ui, s: &mut State, app: &AsterApp, h: f32, now: f64) {
    card_sized(ui, 14.0, h, |ui| {
        let shown = s
            .rows
            .iter()
            .filter(|r| s.show_inactive || r.active)
            .count();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Supplier directory")
                    .size(16.0)
                    .strong()
                    .color(INK),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                pill(ui, format!("{shown} shown"), TINT, PURPLE);
            });
        });
        ui.add_space(6.0);

        // Search box
        let search_id = Id::new("supplier-master-search");
        let focused = ui.memory(|m| m.has_focus(search_id));
        let focus_t = ease(ui.ctx(), Id::new("supplier-search-focus"), focused);
        egui::Frame::new()
            .fill(CANVAS)
            .stroke(Stroke::new(
                1.0_f32,
                BORDER_STRONG.lerp_to_gamma(PURPLE, focus_t),
            ))
            .corner_radius(9.0)
            .inner_margin(Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                    paint_search_icon(ui.painter(), r.center(), MUTED.lerp_to_gamma(PURPLE, focus_t));
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
                            reload_rows(s, app, now);
                        }
                        let r = ui.add(
                            egui::TextEdit::singleline(&mut s.search)
                                .id(search_id)
                                .frame(false)
                                .hint_text("Search supplier, contact, phone, GSTIN…")
                                .desired_width(f32::INFINITY),
                        );
                        if r.changed() {
                            reload_rows(s, app, now);
                        }
                        if r.has_focus()
                            && !s.search.is_empty()
                            && ui.input(|i| i.key_pressed(Key::Escape))
                        {
                            s.search.clear();
                            reload_rows(s, app, now);
                        }
                    });
                });
            });
        ui.add_space(6.0);

        // Filter chips
        ui.horizontal(|ui| {
            let active_n = s.rows.iter().filter(|r| r.active).count();
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
        ui.add_space(6.0);
        hairline(ui);
        ui.add_space(4.0);

        let idx: Vec<usize> = s
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| s.show_inactive || r.active)
            .map(|(i, _)| i)
            .collect();

        if idx.is_empty() {
            let (t, sub) = if !s.search.trim().is_empty() {
                (
                    "No suppliers match your search",
                    "Try a different name, phone or GSTIN.",
                )
            } else if !s.rows.is_empty() {
                (
                    "No active suppliers",
                    "Switch to “All” to see inactive suppliers.",
                )
            } else {
                ("No suppliers yet", "Use the form on the right to add one.")
            };
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 24.0, TINT);
                paint_search_icon(ui.painter(), r.center(), PURPLE);
                ui.add_space(8.0);
                ui.label(RichText::new(t).size(14.0).strong().color(INK));
                ui.label(RichText::new(sub).size(12.0).color(MUTED));
            });
            return;
        }

        let mut clicked: Option<i64> = None;
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ScrollArea::vertical()
                .id_salt("supplier-directory")
                .auto_shrink([false, false])
                .show_rows(ui, ROW_H, idx.len(), |ui, range| {
                    for i in range {
                        let r = &s.rows[idx[i]];
                        if supplier_row(ui, r, s.selected == Some(r.id)).clicked() {
                            clicked = Some(r.id);
                        }
                    }
                });
        });
        if let Some(id) = clicked {
            request_nav(s, app, Nav::Open(id), now);
            ui.ctx().request_repaint();
        }
    });
}

fn supplier_row(ui: &mut egui::Ui, r: &SupplierRecord, selected: bool) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::hover());
    // Stable id so hover/click survive scrolling of the virtualised list.
    let resp = ui
        .interact(rect, Id::new(("supplier-row", r.id)), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);
    let sel_t = ease(ui.ctx(), Id::new(("row-sel", r.id)), selected);
    let hov_t = ease(ui.ctx(), Id::new(("row-hov", r.id)), resp.hovered());
    let p = ui.painter();
    let inner = rect.shrink2(vec2(0.0, 1.0));

    if sel_t > 0.0 {
        p.rect_filled(inner, 9.0, TINT.gamma_multiply(sel_t));
        let bar_h = (ROW_H - 24.0) * sel_t;
        p.rect_filled(
            Rect::from_min_size(
                pos2(rect.left(), rect.center().y - bar_h / 2.0),
                vec2(3.0, bar_h),
            ),
            2.0,
            PURPLE,
        );
    }
    if hov_t > 0.0 && sel_t < 1.0 {
        p.rect_filled(inner, 9.0, HOVER.gamma_multiply(hov_t * (1.0 - sel_t)));
    }

    let av = Rect::from_center_size(
        pos2(rect.left() + PAD + AVATAR / 2.0, rect.center().y),
        Vec2::splat(AVATAR),
    );
    paint_avatar(p, av, &r.name);

    let text_x = PAD + AVATAR + 12.0;
    let status_x = rect.width() - PAD - STATUS_W;
    let text_w = (status_x - 14.0 - text_x).max(40.0);

    let name_col = if r.active { INK } else { MUTED };
    let g = text_galley(ui, &r.name, 14.0, name_col, text_w);
    p.galley(pos2(rect.left() + text_x, rect.top() + 10.0), g, name_col);

    let lead = if r.trade_name.is_empty() {
        if r.phone.is_empty() {
            "No phone"
        } else {
            r.phone.as_str()
        }
    } else {
        r.trade_name.as_str()
    };
    let sub = if r.city.is_empty() {
        lead.to_owned()
    } else {
        format!("{lead}  ·  {}", r.city)
    };
    let g = text_galley(ui, &sub, 11.5, MUTED, text_w);
    p.galley(pos2(rect.left() + text_x, rect.top() + 31.0), g, MUTED);

    let pr = Rect::from_center_size(
        pos2(rect.left() + status_x + STATUS_W / 2.0, rect.center().y),
        vec2(STATUS_W, 22.0),
    );
    let (bg, fg, t) = if r.active {
        (GREEN_BG, GREEN, "ACTIVE")
    } else {
        (RED_BG, RED, "INACTIVE")
    };
    p.rect_filled(pr, 11.0, bg);
    p.text(
        pr.center(),
        Align2::CENTER_CENTER,
        t,
        FontId::proportional(9.5),
        fg,
    );
    resp
}

// ───────────────────────────── Editor ─────────────────────────────

/// Per-frame view of the current validation error, handed to each field.
struct Fx {
    err: Option<(FieldId, String)>,
    fresh: bool,
}

impl Fx {
    fn opts(&self, f: FieldId) -> Opts {
        match &self.err {
            Some((id, msg)) if *id == f => Opts {
                invalid: true,
                note: Some(msg.clone()),
                note_error: true,
                focus: self.fresh,
                ..Opts::default()
            },
            _ => Opts::default(),
        }
    }
}

fn editor(ui: &mut egui::Ui, s: &mut State, app: &AsterApp, h: f32, now: f64) {
    // Toast lifetime
    let age = s.toast_age(now);
    let ttl = s.toast_ttl();
    if age > ttl {
        s.dismiss_toast();
    } else if !s.message.is_empty() {
        let remaining = (ttl - age).max(0.0);
        let step = if age < 0.3 { 0.016 } else { remaining.min(0.25) };
        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64(step.max(0.016)));
    }

    // Editing anything clears the field error.
    if s.err.as_ref().is_some_and(|e| e.snapshot != s.form) {
        s.err = None;
    }

    card_sized(ui, 16.0, h, |ui| {
        // Footer is laid out first (bottom-up) so it is always reachable and the
        // scrolling body gets exactly the space that is left.
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            footer(ui, s, app, now);
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                editor_header(ui, s);
                ui.add_space(10.0);
                hairline(ui);
                ui.add_space(6.0);
                pending_banner(ui, s, app, now);
                editor_body(ui, s, app, now);
            });
        });
    });
}

fn editor_header(ui: &mut egui::Ui, s: &State) {
    let existing = s.form.id.is_some();
    let dirty = s.dirty();
    ui.horizontal(|ui| {
        avatar(ui, &s.form.name, 40.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            let reserve = if existing && dirty { 190.0 } else { 110.0 };
            ui.set_max_width((ui.available_width() - reserve).max(80.0));
            let name = s.form.name.trim();
            ui.add(
                egui::Label::new(
                    RichText::new(if name.is_empty() {
                        "New supplier"
                    } else {
                        name
                    })
                    .size(16.5)
                    .strong()
                    .color(INK),
                )
                .truncate(),
            );
            let sub = match (s.form.id, dirty, s.form.active) {
                (Some(id), true, _) => format!("Unsaved changes  ·  ID {id}"),
                (Some(id), false, true) => format!("Active supplier  ·  ID {id}"),
                (Some(id), false, false) => format!("Inactive supplier  ·  ID {id}"),
                (None, _, _) if name.is_empty() => {
                    "New supplier  ·  enter a legal name to begin".into()
                }
                (None, _, _) => "New supplier  ·  not yet saved".into(),
            };
            ui.add(egui::Label::new(RichText::new(sub).size(11.5).color(MUTED)).truncate());
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
}

fn pending_banner(ui: &mut egui::Ui, s: &mut State, app: &AsterApp, now: f64) {
    let Some(nav) = s.pending else { return };
    let t = smooth(((now - s.pending_at) / 0.2) as f32);
    if t < 1.0 {
        ui.ctx().request_repaint();
    }
    let mut resolve: Option<bool> = None; // Some(true) = discard and continue
    ui.scope(|ui| {
        ui.set_opacity(t);
        egui::Frame::new()
            .fill(AMBER_BG)
            .stroke(Stroke::new(1.0_f32, AMBER.gamma_multiply(0.35)))
            .corner_radius(10.0)
            .inner_margin(Margin::symmetric(12, 9))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(
                    RichText::new("You have unsaved changes")
                        .size(12.5)
                        .strong()
                        .color(AMBER),
                );
                ui.label(
                    RichText::new("Continuing will discard them.")
                        .size(11.5)
                        .color(MUTED),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if button(ui, "Discard & continue", Kind::Danger).clicked() {
                        resolve = Some(true);
                    }
                    if button(ui, "Keep editing", Kind::Secondary).clicked() {
                        resolve = Some(false);
                    }
                });
            });
    });
    ui.add_space(6.0);
    match resolve {
        Some(true) => {
            apply_nav(s, app, nav, now);
            ui.ctx().request_repaint();
        }
        Some(false) => {
            s.pending = None;
            ui.ctx().request_repaint();
        }
        None => {}
    }
}

fn editor_body(ui: &mut egui::Ui, s: &mut State, app: &AsterApp, now: f64) {
    let swap_t = smooth(((now - s.swap_at) / 0.22) as f32);
    if swap_t < 1.0 {
        ui.ctx().request_repaint();
    }
    let fx = Fx {
        err: s.err.as_ref().map(|e| (e.field, e.msg.clone())),
        fresh: std::mem::take(&mut s.err_fresh),
    };
    let existing = s.form.id.is_some();

    let mut area = ScrollArea::vertical()
        .id_salt("supplier-editor-scroll")
        .auto_shrink([false, false])
        .max_height(ui.available_height());
    if std::mem::take(&mut s.reset_scroll) {
        area = area.vertical_scroll_offset(0.0);
    }

    area.show(ui, |ui| {
        ui.set_opacity(0.35 + 0.65 * swap_t);
        ui.set_min_width(ui.available_width());

        section(
            ui,
            "IDENTITY & CONTACT",
            "Registered details and how to reach them",
        );
        field_with(
            ui,
            "Legal name",
            &mut s.form.name,
            "Registered business name",
            Opts {
                required: true,
                ..fx.opts(FieldId::Name)
            },
        );
        ui.columns(2, |c| {
            field(
                &mut c[0],
                "Trade name",
                &mut s.form.trade_name,
                "Name they trade under",
            );
            field(
                &mut c[1],
                "Contact person",
                &mut s.form.contact_person,
                "Primary contact",
            );
        });
        ui.columns(2, |c| {
            field_with(
                &mut c[0],
                "Phone",
                &mut s.form.phone,
                "e.g. 98765 43210",
                fx.opts(FieldId::Phone),
            );
            field_with(
                &mut c[1],
                "Email",
                &mut s.form.email,
                "name@example.com",
                fx.opts(FieldId::Email),
            );
        });
        let g = s.form.gstin.trim().chars().count();
        let mut gstin_opts = fx.opts(FieldId::Gstin);
        if gstin_opts.note.is_none() && g > 0 && g != 15 {
            gstin_opts.note = Some(format!("{g} of 15 characters"));
        }
        field_with(
            ui,
            "GSTIN",
            &mut s.form.gstin,
            "15-character GST identification number",
            gstin_opts,
        );
        field(
            ui,
            "Drug licence number(s)",
            &mut s.form.drug_license,
            "Licence reference(s)",
        );

        section(ui, "ADDRESS", "Where goods are dispatched from");
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
            field(&mut c[0], "City / town", &mut s.form.city, "e.g. Chennai");
            field(&mut c[1], "State", &mut s.form.state, "e.g. Tamil Nadu");
        });
        ui.columns(2, |c| {
            field_with(
                &mut c[0],
                "GST state code",
                &mut s.form.state_code,
                "e.g. 33 (auto from GSTIN)",
                fx.opts(FieldId::StateCode),
            );
            field_with(
                &mut c[1],
                "PIN code",
                &mut s.form.pincode,
                "6 digits",
                fx.opts(FieldId::Pincode),
            );
        });

        section(
            ui,
            "CREDIT & PAYMENT TERMS",
            "Account setup and opening balance",
        );
        ui.columns(3, |c| {
            field_with(
                &mut c[0],
                "Terms (days)",
                &mut s.form.payment_terms_days,
                "0",
                Opts {
                    right: true,
                    ..fx.opts(FieldId::Terms)
                },
            );
            field_with(
                &mut c[1],
                "Credit limit (₹)",
                &mut s.form.credit_limit,
                "0.00",
                Opts {
                    right: true,
                    ..fx.opts(FieldId::Credit)
                },
            );
            field_with(
                &mut c[2],
                "Opening bal. (₹)",
                &mut s.form.opening_balance,
                "0.00",
                Opts {
                    right: true,
                    ..fx.opts(FieldId::Opening)
                },
            );
        });

        manufacturers_section(ui, s, app, now);
        products_section(ui, s);

        if !existing {
            section(ui, "STATUS", "");
            switch(ui, "supplier-active", &mut s.form.active, "Supplier is active");
        }
        ui.add_space(12.0);
    });
}

fn footer(ui: &mut egui::Ui, s: &mut State, app: &AsterApp, now: f64) {
    let existing = s.form.id.is_some();
    let dirty = s.dirty();
    let w = ui.available_width();

    // Action row (bottom-most)
    ui.allocate_ui_with_layout(vec2(w, 34.0), Layout::left_to_right(Align::Center), |ui| {
        let label = if existing {
            "Save changes"
        } else {
            "Save supplier"
        };
        let can_save = !existing || dirty;
        let clicked = ui
            .add_enabled_ui(can_save, |ui| button(ui, label, Kind::Primary).clicked())
            .inner;
        if clicked {
            save(s, app, now);
            ui.ctx().request_repaint();
        }
        if button(ui, if existing { "Close" } else { "Clear" }, Kind::Secondary).clicked() {
            if existing {
                request_nav(s, app, Nav::New, now);
            } else {
                s.new_form(now);
            }
            ui.ctx().request_repaint();
        }
        if existing {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (label, kind) = if !s.form.active {
                    ("Reactivate", Kind::Secondary)
                } else if s.confirm_deactivate {
                    ("Confirm deactivate", Kind::Danger)
                } else {
                    ("Deactivate", Kind::Danger)
                };
                if button(ui, label, kind).clicked() {
                    if s.form.active && !s.confirm_deactivate {
                        s.confirm_deactivate = true;
                    } else {
                        s.confirm_deactivate = false;
                        toggle_active(s, app, now);
                    }
                    ui.ctx().request_repaint();
                }
                if s.confirm_deactivate && button(ui, "Cancel", Kind::Secondary).clicked() {
                    s.confirm_deactivate = false;
                    ui.ctx().request_repaint();
                }
            });
        }
    });

    // Toast sits directly above the buttons
    if !s.message.is_empty() {
        ui.add_space(8.0);
        toast(ui, s, now);
    }

    ui.add_space(8.0);
    hairline(ui);
    ui.add_space(8.0);
}

fn toast(ui: &mut egui::Ui, s: &mut State, now: f64) {
    let age = s.toast_age(now);
    let ttl = s.toast_ttl();
    let entry = smooth((age / 0.18) as f32);
    let exit = ((ttl - age).max(0.0) / 0.25).clamp(0.0, 1.0) as f32;
    let alpha = (entry * exit).clamp(0.0, 1.0);

    let (bg, fg, icon) = if s.error {
        (RED_BG, RED, "!")
    } else {
        (GREEN_BG, GREEN, "✓")
    };

    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::hover());
    let hover = ui.interact(rect, Id::new("toast-body"), Sense::hover());
    let x_rect = Rect::from_center_size(pos2(rect.right() - 20.0, rect.center().y), Vec2::splat(24.0));
    let x_resp = ui
        .interact(x_rect, Id::new("toast-close"), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);

    let msg_w = (rect.width() - 38.0 - 40.0).max(40.0);
    let g = text_galley(ui, &s.message, 12.5, fg.gamma_multiply(alpha), msg_w);
    let p = ui.painter();
    p.rect_filled(rect, 10.0, fg.gamma_multiply(0.35 * alpha));
    p.rect_filled(rect.shrink(1.0), 9.0, bg.gamma_multiply(alpha));
    let c = pos2(rect.left() + 20.0, rect.center().y);
    p.circle_filled(c, 8.0, fg.gamma_multiply(alpha));
    p.text(
        c,
        Align2::CENTER_CENTER,
        icon,
        FontId::proportional(11.0),
        Color32::WHITE.gamma_multiply(alpha),
    );
    let gy = rect.center().y - g.size().y / 2.0;
    p.galley(pos2(rect.left() + 38.0, gy), g, fg.gamma_multiply(alpha));
    let x_col = if x_resp.hovered() { fg } else { fg.gamma_multiply(0.7) };
    p.text(
        x_rect.center(),
        Align2::CENTER_CENTER,
        "×",
        FontId::proportional(16.0),
        x_col.gamma_multiply(alpha),
    );
    hover.on_hover_text(s.message.clone());
    if x_resp.clicked() {
        s.dismiss_toast();
    }
}

// ───────────────────────────── Manufacturers & products ─────────────────────────────
fn manufacturers_section(ui: &mut egui::Ui, s: &mut State, app: &AsterApp, now: f64) {
    let total = s.manufacturers.len();
    let sub = if total == 0 {
        "None yet — add one below".to_owned()
    } else {
        format!("{} of {} selected", s.manufacturer_ids.len(), total)
    };
    section(ui, "MANUFACTURERS", &sub);

    ui.label(
        RichText::new("Type to search, press Enter or “Add” to create a new one, tick to link.")
            .size(11.0)
            .color(MUTED),
    );
    ui.add_space(4.0);
    let mut add = false;
    ui.horizontal(|ui| {
        let w = (ui.available_width() - 84.0).max(120.0);
        let r = ui.add(
            egui::TextEdit::singleline(&mut s.new_manufacturer_name)
                .hint_text("Search or add, e.g. Cipla Ltd")
                .desired_width(w)
                .margin(Margin::symmetric(10, 7)),
        );
        if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            add = true;
        }
        if button(ui, "+ Add", Kind::Secondary).clicked() {
            add = true;
        }
    });
    if add {
        add_manufacturer(s, app, now);
    }
    ui.add_space(4.0);

    let q = s.new_manufacturer_name.trim().to_lowercase();
    let idx: Vec<usize> = s
        .manufacturers
        .iter()
        .enumerate()
        .filter(|(_, m)| q.is_empty() || m.name.to_lowercase().contains(&q))
        .map(|(i, _)| i)
        .collect();

    if idx.is_empty() {
        let msg = if s.manufacturers.is_empty() {
            "No manufacturers yet.".to_owned()
        } else {
            format!("No match for “{}”. Press Add to create it.", s.new_manufacturer_name.trim())
        };
        empty_box(ui, &msg);
        return;
    }

    let mut toggles: Vec<(i64, bool)> = Vec::new();
    list_box(ui, "supplier-mfr-list", 140.0, idx.len(), |ui, range| {
        for i in range {
            let m = &s.manufacturers[idx[i]];
            let on = s.manufacturer_ids.contains(&m.id);
            if check_row(ui, Id::new(("chk", "mfr", m.id)), on, &m.name, "") {
                toggles.push((m.id, !on));
            }
        }
    });
    for (id, on) in toggles {
        set_membership(&mut s.manufacturer_ids, id, on);
    }
}

fn add_manufacturer(s: &mut State, app: &AsterApp, now: f64) {
    let name = s.new_manufacturer_name.trim().to_owned();
    if name.is_empty() {
        s.toast("Enter a manufacturer name first.", true, now);
        return;
    }
    // Re-use an existing manufacturer instead of creating a duplicate.
    let existing = s
        .manufacturers
        .iter()
        .find(|m| m.name.trim().eq_ignore_ascii_case(&name))
        .map(|m| m.id);
    if let Some(id) = existing {
        set_membership(&mut s.manufacturer_ids, id, true);
        s.new_manufacturer_name.clear();
        s.toast(format!("'{name}' already exists and is now selected."), false, now);
        return;
    }
    let result = app
        .db
        .as_ref()
        .ok_or_else(|| "Database is not open.".to_owned())
        .and_then(|db| db.create_manufacturer(app.selected_year, &name));
    match result {
        Ok(id) => {
            s.new_manufacturer_name.clear();
            reload_masters(s, app);
            set_membership(&mut s.manufacturer_ids, id, true);
            s.toast(format!("Manufacturer '{name}' created."), false, now);
        }
        Err(e) => s.toast(e, true, now),
    }
}

fn products_section(ui: &mut egui::Ui, s: &mut State) {
    let total = s.products.len();
    let sub = if total == 0 {
        "No products in master".to_owned()
    } else {
        format!("{} of {} linked", s.product_ids.len(), total)
    };
    section(ui, "PRODUCTS SUPPLIED", &sub);
    ui.label(
        RichText::new("Pick from Product Master. One product can have several suppliers.")
            .size(11.0)
            .color(MUTED),
    );
    ui.add_space(4.0);
    ui.add(
        egui::TextEdit::singleline(&mut s.product_filter)
            .hint_text("Filter products by name, brand or manufacturer…")
            .desired_width(f32::INFINITY)
            .margin(Margin::symmetric(10, 7)),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        egui::Frame::new()
            .fill(TINT)
            .corner_radius(9.0)
            .inner_margin(3.0)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if chip(ui, "All", !s.product_selected_only).clicked() {
                    s.product_selected_only = false;
                }
                if chip(ui, "Selected", s.product_selected_only).clicked() {
                    s.product_selected_only = true;
                }
            });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if !s.product_ids.is_empty() && link_button(ui, "Clear selection").clicked() {
                s.product_ids.clear();
            }
        });
    });
    ui.add_space(4.0);

    let q = s.product_filter.trim().to_lowercase();
    let idx: Vec<usize> = s
        .products
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            (!s.product_selected_only || s.product_ids.contains(&p.id))
                && (q.is_empty()
                    || p.name.to_lowercase().contains(&q)
                    || p.manufacturer_name.to_lowercase().contains(&q)
                    || p.brand_name.to_lowercase().contains(&q))
        })
        .map(|(i, _)| i)
        .collect();

    if idx.is_empty() {
        let msg = if s.products.is_empty() {
            "No products found. Create products first in Product Master."
        } else if s.product_selected_only && q.is_empty() {
            "No products selected yet."
        } else {
            "No products match your filter."
        };
        empty_box(ui, msg);
        return;
    }

    let mut toggles: Vec<(i64, bool)> = Vec::new();
    list_box(ui, "supplier-prod-list", 190.0, idx.len(), |ui, range| {
        for i in range {
            let p = &s.products[idx[i]];
            let on = s.product_ids.contains(&p.id);
            let sub = if !p.manufacturer_name.is_empty() {
                p.manufacturer_name.as_str()
            } else {
                p.brand_name.as_str()
            };
            if check_row(ui, Id::new(("chk", "prod", p.id)), on, &p.name, sub) {
                toggles.push((p.id, !on));
            }
        }
    });
    for (id, on) in toggles {
        set_membership(&mut s.product_ids, id, on);
    }
}

fn set_membership(v: &mut Vec<i64>, id: i64, on: bool) {
    if on {
        if !v.contains(&id) {
            v.push(id);
        }
    } else {
        v.retain(|x| *x != id);
    }
}

fn list_box(
    ui: &mut egui::Ui,
    id: &str,
    max_h: f32,
    rows: usize,
    add_rows: impl FnOnce(&mut egui::Ui, Range<usize>),
) {
    egui::Frame::new()
        .fill(CANVAS)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(9.0)
        .inner_margin(Margin::symmetric(6, 6))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 2.0;
            ScrollArea::vertical()
                .id_salt(id)
                .max_height(max_h)
                .auto_shrink([false, true])
                .show_rows(ui, CHECK_ROW_H, rows, add_rows);
        });
}

fn empty_box(ui: &mut egui::Ui, msg: &str) {
    egui::Frame::new()
        .fill(CANVAS)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(9.0)
        .inner_margin(Margin::symmetric(12, 14))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(msg).size(11.5).color(MUTED));
        });
}

/// A fixed-height, fully painted checkbox row (works inside `show_rows`).
fn check_row(ui: &mut egui::Ui, key: Id, checked: bool, title: &str, sub: &str) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), CHECK_ROW_H), Sense::hover());
    let resp = ui
        .interact(rect, key, Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);
    let hov = ease(ui.ctx(), key.with("hover"), resp.hovered());
    let on = ease(ui.ctx(), key.with("check"), checked);

    let mut job = LayoutJob::default();
    job.append(
        title,
        0.0,
        TextFormat {
            font_id: FontId::proportional(12.5),
            color: INK,
            ..Default::default()
        },
    );
    if !sub.is_empty() {
        job.append(
            &format!("   {sub}"),
            0.0,
            TextFormat {
                font_id: FontId::proportional(11.0),
                color: MUTED,
                ..Default::default()
            },
        );
    }
    job.wrap.max_width = (rect.width() - 34.0 - 8.0).max(20.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let g = ui.painter().layout_job(job);

    let p = ui.painter();
    if hov > 0.0 {
        p.rect_filled(rect, 7.0, TINT.gamma_multiply(hov));
    }
    let bx = Rect::from_center_size(pos2(rect.left() + 18.0, rect.center().y), Vec2::splat(16.0));
    p.rect_filled(bx, 4.5, BORDER_STRONG.lerp_to_gamma(PURPLE, on));
    p.rect_filled(bx.shrink(1.2), 3.5, PANEL.lerp_to_gamma(PURPLE, on));
    if on > 0.01 {
        let o = bx.left_top();
        let st = Stroke::new(1.8_f32, Color32::WHITE.gamma_multiply(on));
        p.line_segment([o + vec2(3.8, 8.2), o + vec2(6.8, 11.2)], st);
        p.line_segment([o + vec2(6.8, 11.2), o + vec2(12.2, 4.8)], st);
    }
    let gy = rect.center().y - g.size().y / 2.0;
    p.galley(pos2(rect.left() + 34.0, gy), g, INK);
    resp.clicked()
}

// ───────────────────────────── Persistence ─────────────────────────────
fn save(s: &mut State, app: &AsterApp, now: f64) {
    if s.form.id.is_some() && !s.dirty() {
        s.toast("No changes to save.", false, now);
        return;
    }
    if s.form.id.is_some() && s.links_failed {
        s.toast(
            "This supplier's links failed to load. Reopen it before saving.",
            true,
            now,
        );
        return;
    }
    let record = match s.form.to_record() {
        Ok(r) => r,
        Err(e) => {
            s.err = Some(FieldError {
                field: e.field,
                msg: e.msg.clone(),
                snapshot: s.form.clone(),
            });
            s.err_fresh = true;
            s.toast(e.msg, true, now);
            return;
        }
    };
    let links: Vec<SupplierProductLink> = s
        .product_ids
        .iter()
        .map(|id| SupplierProductLink {
            product_id: *id,
            ..SupplierProductLink::default()
        })
        .collect();
    let result = app
        .db
        .as_ref()
        .ok_or_else(|| "Database is not open.".to_owned())
        .and_then(|db| {
            db.save_supplier(
                app.selected_year,
                s.form.id,
                &record,
                &s.manufacturer_ids,
                &links,
            )
        });
    match result {
        Ok(id) => {
            let was_new = s.form.id.is_none();
            if !record.active {
                s.show_inactive = true;
            }
            reload_rows(s, app, now);
            // The saved supplier may be hidden by the current search; clear it so it can be reopened.
            if !s.rows.iter().any(|r| r.id == id) && !s.search.trim().is_empty() {
                s.search.clear();
                reload_rows(s, app, now);
            }
            match s.rows.iter().find(|r| r.id == id).cloned() {
                Some(r) => s.open(&r, app, now),
                None => {
                    // Still make the editor point at the saved row so the next save updates it.
                    s.form.id = Some(id);
                    s.baseline = s.form.clone();
                    s.selected = Some(id);
                    s.base_mfr = sorted(&s.manufacturer_ids);
                    s.base_prod = sorted(&s.product_ids);
                }
            }
            let msg = if was_new {
                "Supplier created with its product and manufacturer links."
            } else {
                "Supplier saved with its product and manufacturer links."
            };
            s.toast(msg, false, now);
        }
        Err(e) => s.toast(e, true, now),
    }
}

fn toggle_active(s: &mut State, app: &AsterApp, now: f64) {
    let Some(id) = s.form.id else { return };
    let target = !s.form.active;
    let result = app
        .db
        .as_ref()
        .ok_or_else(|| "Database is not open.".to_owned())
        .and_then(|db| db.set_supplier_active(app.selected_year, id, target));
    match result {
        Ok(()) => {
            s.form.active = target;
            s.baseline.active = target;
            if !target {
                s.show_inactive = true; // keep the row visible after deactivating
            }
            reload_rows(s, app, now);
            let msg = if target {
                "Supplier reactivated."
            } else {
                "Supplier deactivated; historical records retained."
            };
            s.toast(msg, false, now);
        }
        Err(e) => s.toast(e, true, now),
    }
}

fn reload_rows(s: &mut State, app: &AsterApp, now: f64) {
    s.loaded = true;
    match app.db.as_ref() {
        Some(db) => match db.suppliers(app.selected_year, &s.search) {
            Ok(rows) => s.rows = rows,
            Err(e) => {
                s.rows.clear();
                s.toast(e, true, now);
            }
        },
        None => s.rows.clear(),
    }
}

fn reload_masters(s: &mut State, app: &AsterApp) {
    if let Some(db) = app.db.as_ref() {
        s.manufacturers = db.manufacturers(app.selected_year).unwrap_or_default();
        s.products = db.products(app.selected_year, "").unwrap_or_default();
    } else {
        s.manufacturers.clear();
        s.products.clear();
    }
}

fn reload_all(s: &mut State, app: &AsterApp, now: f64) {
    reload_rows(s, app, now);
    reload_masters(s, app);
}

// ───────────────────────────── Widgets ─────────────────────────────
fn section(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(3.0, 13.0), Sense::hover());
        ui.painter().rect_filled(r, 2.0, PURPLE);
        ui.label(RichText::new(title).size(10.5).strong().color(INK));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if !subtitle.is_empty() {
                ui.add(
                    egui::Label::new(RichText::new(subtitle).size(10.5).color(FAINT)).truncate(),
                );
            }
        });
    });
    ui.add_space(3.0);
    hairline(ui);
    ui.add_space(6.0);
}

#[derive(Default)]
struct Opts {
    required: bool,
    invalid: bool,
    right: bool,
    note: Option<String>,
    note_error: bool,
    focus: bool,
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
    ui.add_space(2.0);
    ui.scope(|ui| {
        if o.invalid {
            let v = ui.visuals_mut();
            v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, RED);
            v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, RED);
            v.selection.stroke = Stroke::new(1.5_f32, RED);
        }
        let resp = ui.add(
            egui::TextEdit::singleline(value)
                .hint_text(hint)
                .desired_width(f32::INFINITY)
                .margin(Margin::symmetric(10, 7))
                .horizontal_align(if o.right { Align::RIGHT } else { Align::LEFT }),
        );
        if o.focus {
            resp.request_focus();
            resp.scroll_to_me(Some(Align::Center));
        }
    });
    if let Some(n) = o.note.as_deref() {
        ui.add_space(2.0);
        ui.label(
            RichText::new(n)
                .size(11.0)
                .color(if o.note_error { RED } else { AMBER }),
        );
    }
    ui.add_space(6.0);
}

fn switch(ui: &mut egui::Ui, salt: &str, on: &mut bool, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(34.0, 18.0), Sense::hover());
        let resp = ui
            .interact(rect, Id::new(("switch", salt)), Sense::click())
            .on_hover_cursor(CursorIcon::PointingHand);
        if resp.clicked() {
            *on = !*on;
        }
        let t = ease(ui.ctx(), Id::new(("switch-anim", salt)), *on);
        let p = ui.painter();
        p.rect_filled(rect, 9.0, BORDER_STRONG.lerp_to_gamma(PURPLE, t));
        let knob = pos2(rect.left() + 9.0 + 16.0 * t, rect.center().y);
        p.circle_filled(knob, 7.0, Color32::WHITE);
        let lab = ui.add(
            egui::Label::new(RichText::new(label).size(12.5).color(INK)).sense(Sense::click()),
        );
        if lab.clicked() {
            *on = !*on;
        }
    });
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Primary,
    Secondary,
    Danger,
}

fn button(ui: &mut egui::Ui, label: &str, kind: Kind) -> egui::Response {
    ui.scope(|ui| {
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
                Stroke::new(1.0_f32, BORDER_STRONG),
                Stroke::new(1.0_f32, Color32::from_rgb(184, 172, 208)),
            ),
            Kind::Danger => (
                PANEL,
                RED_BG,
                RED,
                Stroke::new(1.0_f32, Color32::from_rgb(232, 196, 196)),
                Stroke::new(1.0_f32, RED),
            ),
        };
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

fn link_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(11.5).strong().color(PURPLE)).frame(false),
    )
    .on_hover_cursor(CursorIcon::PointingHand)
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

fn card(margin: f32) -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(12.0)
        .inner_margin(margin)
}

/// A card that is exactly `h` tall, so inner scroll areas know their space.
fn card_sized<R>(
    ui: &mut egui::Ui,
    margin: f32,
    h: f32,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    card(margin)
        .show(ui, |ui| {
            let inner_h = (h - margin * 2.0 - 2.0).max(120.0);
            ui.set_min_height(inner_h);
            ui.set_max_height(inner_h);
            ui.set_min_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn stat_card(ui: &mut egui::Ui, w: f32, label: &str, value: String, accent: Color32) {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(10.0)
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width((w - 30.0).max(60.0));
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 4.0, accent);
                ui.label(RichText::new(label).size(10.0).strong().color(MUTED));
            });
            ui.label(RichText::new(value).size(26.0).strong().color(INK));
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

fn paint_search_icon(p: &egui::Painter, center: egui::Pos2, color: Color32) {
    let c = center + vec2(-1.5, -1.5);
    p.circle_stroke(c, 5.0, Stroke::new(1.6_f32, color));
    p.line_segment(
        [c + Vec2::splat(3.6), c + Vec2::splat(7.4)],
        Stroke::new(1.8_f32, color),
    );
}

fn text_galley(
    ui: &egui::Ui,
    text: &str,
    size: f32,
    color: Color32,
    max_w: f32,
) -> Arc<egui::Galley> {
    let mut job =
        LayoutJob::simple_singleline(text.to_owned(), FontId::proportional(size), color);
    job.wrap.max_width = max_w.max(1.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    ui.painter().layout_job(job)
}
