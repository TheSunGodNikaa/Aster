use crate::{
    app::AsterApp,
    database::{ManufacturerOption, ProductRecord, SupplierProductLink, SupplierRecord},
};
use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Id, Layout, Margin, Rect, RichText,
    ScrollArea, Sense, Stroke, Vec2, pos2, vec2,
};
use std::sync::Arc;
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
const SHADOW: Color32 = Color32::from_rgba_premultiplied(35, 32, 43, 28);

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

// ───────────────────────────── Data ─────────────────────────────
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

    fn to_record(&self) -> Result<SupplierRecord, String> {
        if self.name.trim().is_empty() {
            return Err("Supplier legal name is required.".into());
        }
        let days = if self.payment_terms_days.trim().is_empty() {
            0
        } else {
            self.payment_terms_days
                .trim()
                .parse::<i64>()
                .map_err(|_| "Payment terms must be a whole number of days.".to_owned())?
        };
        if !(0..=3650).contains(&days) {
            return Err("Payment terms must be between 0 and 3650 days.".into());
        }
        let money = |v: &str, label: &str, nonnegative: bool| -> Result<i64, String> {
            let n = if v.trim().is_empty() {
                0.0
            } else {
                v.trim()
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
        Ok(SupplierRecord {
            id: self.id.unwrap_or_default(),
            name: self.name.trim().into(),
            trade_name: self.trade_name.trim().into(),
            contact_person: self.contact_person.trim().into(),
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
            payment_terms_days: days,
            credit_limit_paise: money(&self.credit_limit, "credit limit", true)?,
            opening_balance_paise: money(&self.opening_balance, "opening balance", false)?,
            active: self.active,
        })
    }
}

#[derive(Clone, Default)]
struct State {
    loaded: bool,
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
    // toast
    message: String,
    error: bool,
    toast_at: f64, // ctx.time() when set
}

fn sorted(v: &[i64]) -> Vec<i64> {
    let mut x = v.to_vec();
    x.sort_unstable();
    x
}

impl State {
    fn new_form(&mut self) {
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
        self.message.clear();
        self.error = false;
        self.toast_at = 0.0;
    }

    fn open(&mut self, r: &SupplierRecord, app: &AsterApp) {
        self.form = Form::from_record(r);
        self.baseline = self.form.clone();
        self.selected = Some(r.id);
        self.message.clear();
        self.error = false;
        self.toast_at = 0.0;
        if let Some(db) = app.db.as_ref() {
            self.manufacturer_ids = db
                .supplier_manufacturer_ids(app.selected_year, r.id)
                .unwrap_or_default();
            self.product_ids = db
                .supplier_product_links(app.selected_year, r.id)
                .unwrap_or_default()
                .into_iter()
                .map(|p| p.product_id)
                .collect();
        }
        self.base_mfr = sorted(&self.manufacturer_ids);
        self.base_prod = sorted(&self.product_ids);
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
    let mut s = ui
        .ctx()
        .data_mut(|d| d.get_temp::<State>(key).unwrap_or_default());
    if !s.loaded {
        reload(&mut s, app);
        s.new_form();
    }
    ui.scope(|ui| {
        apply_theme(ui);
        page(ui, &mut s, app);
    });
    ui.ctx().data_mut(|d| d.insert_temp(key, s));
}

fn apply_theme(ui: &mut egui::Ui) {
    let style = ui.style_mut();
    style.spacing.item_spacing = vec2(8.0, 6.0);
    style.spacing.button_padding = vec2(14.0, 7.0);
    let v = &mut style.visuals;
    v.extreme_bg_color = PANEL;
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

/// Fade/slide value in 0..=1 driven by egui's animation clock.
fn ease(ctx: &egui::Context, id: Id, on: bool) -> f32 {
    let t = ctx.animate_bool_with_time(id, on, 0.22);
    // smoothstep
    t * t * (3.0 - 2.0 * t)
}

fn page(ui: &mut egui::Ui, s: &mut State, app: &AsterApp) {
    // One-shot intro animation on first paint.
    let intro_id = Id::new("supplier-master-intro");
    let intro = ease(ui.ctx(), intro_id, true);
    let intro_off = (1.0 - intro) * 6.0;

    ui.horizontal(|ui| {
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
                s.new_form();
            }
            if button(ui, "Refresh", Kind::Secondary).clicked() {
                reload(s, app);
            }
        });
    });
    ui.add_space(12.0);

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
    ui.add_space(12.0);

    let w = ui.available_width();
    let col_gap = 14.0;
    let right = (w * 0.52).clamp(430.0, 650.0).min(w * 0.62);
    let left = (w - right - col_gap).max(240.0);
    let mut h = ui.available_height() - 4.0;
    if !h.is_finite() {
        h = 600.0;
    }
    let h = h.max(300.0);

    let top = ui.cursor().top() + intro_off;
    ui.allocate_ui_with_layout(vec2(w, h), Layout::left_to_right(Align::Min), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.allocate_ui_with_layout(vec2(left, h), Layout::top_down(Align::Min), |ui| {
            directory(ui, s, app);
        });
        ui.add_space(col_gap);
        ui.allocate_ui_with_layout(vec2(right, h), Layout::top_down(Align::Min), |ui| {
            editor(ui, s, app);
        });
    });
    let _ = top;
}

// ───────────────────────────── Directory ─────────────────────────────
const ROW_H: f32 = 56.0;
const PAD: f32 = 12.0;
const AVATAR: f32 = 36.0;
const STATUS_W: f32 = 70.0;

fn directory(ui: &mut egui::Ui, s: &mut State, app: &AsterApp) {
    card(14.0).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
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

        let search_id = Id::new("supplier-master-search");
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
                                .hint_text("Search supplier, contact, phone, GSTIN…")
                                .desired_width(f32::INFINITY),
                        );
                        if r.changed() {
                            reload(s, app);
                        }
                    });
                });
            });
        ui.add_space(6.0);

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

        let mut clicked: Option<usize> = None;
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ScrollArea::vertical()
                .id_salt("supplier-directory")
                .auto_shrink([false, false])
                .show_rows(ui, ROW_H, idx.len(), |ui, range| {
                    for i in range {
                        let r = &s.rows[idx[i]];
                        if supplier_row(ui, r, s.selected == Some(r.id)).clicked() {
                            clicked = Some(idx[i]);
                        }
                    }
                });
        });
        if let Some(i) = clicked {
            let r = s.rows[i].clone();
            s.open(&r, app);
        }
    });
}

fn supplier_row(ui: &mut egui::Ui, r: &SupplierRecord, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let p = ui.painter();
    let inner = rect.shrink2(vec2(0.0, 1.0));

    // animated selection tint
    let sel_t = ease(ui.ctx(), Id::new(("row-sel", r.id)), selected);
    let hov_t = ease(ui.ctx(), Id::new(("row-hov", r.id)), resp.hovered());

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
    } else if hov_t > 0.0 {
        p.rect_filled(inner, 9.0, HOVER.gamma_multiply(hov_t));
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
fn editor(ui: &mut egui::Ui, s: &mut State, app: &AsterApp) {
    let now = ui.ctx().input(|i| i.time);
    let age = s.toast_age(now);
    let ttl = s.toast_ttl();
    if age > ttl {
        // auto-dismiss
        s.message.clear();
        s.error = false;
        s.toast_at = 0.0;
    } else if !s.message.is_empty() {
        // schedule wake-up exactly when the toast should expire
        let remaining = (ttl - age).max(0.0);
        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64(remaining.min(0.25)));
    }

    let card_hover = ease(ui.ctx(), Id::new("supplier-editor-hover"), false);
    let lift = card_hover * 0.0; // reserved for future hover lift

    card(16.0).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.add_space(lift);

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
                // Contextual subtitle that actually says something useful.
                let sub = match (s.form.id, dirty, s.form.active) {
                    (Some(id), true, _) => format!("Unsaved changes  ·  ID {id}"),
                    (Some(id), false, true) => format!("Active supplier  ·  ID {id}"),
                    (Some(id), false, false) => format!("Inactive supplier  ·  ID {id}"),
                    (None, _, _) if name.is_empty() => {
                        "New supplier  ·  enter a legal name to begin".into()
                    }
                    (None, _, _) => "New supplier  ·  not yet saved".into(),
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
        ui.add_space(10.0);
        hairline(ui);
        ui.add_space(6.0);

        // Reserve space for the footer buttons + toast so the scroll area
        // never hides the action row.
        let toast_reserve = if s.message.is_empty() { 0.0 } else { 68.0 };
        let footer_h = 58.0 + toast_reserve;
        ScrollArea::vertical()
            .id_salt("supplier-editor-scroll")
            .auto_shrink([false, false])
            .max_height((ui.available_height() - footer_h).max(150.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                let name_invalid = s.error && s.form.name.trim().is_empty();

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
                        invalid: name_invalid,
                        ..Opts::default()
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
                    field(&mut c[0], "Phone", &mut s.form.phone, "10-digit mobile");
                    field(&mut c[1], "Email", &mut s.form.email, "name@example.com");
                });
                let g = s.form.gstin.trim().chars().count();
                let note = if g == 0 {
                    None
                } else {
                    Some(format!("{g} of 15 characters"))
                };
                field_with(
                    ui,
                    "GSTIN",
                    &mut s.form.gstin,
                    "15-character GST identification number",
                    Opts {
                        note: note.as_deref(),
                        invalid: g > 0 && g != 15,
                        ..Opts::default()
                    },
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
                    field(
                        &mut c[0],
                        "GST state code",
                        &mut s.form.state_code,
                        "e.g. 33",
                    );
                    field(&mut c[1], "PIN code", &mut s.form.pincode, "6 digits");
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
                            ..Opts::default()
                        },
                    );
                    field_with(
                        &mut c[1],
                        "Credit limit (₹)",
                        &mut s.form.credit_limit,
                        "0.00",
                        Opts {
                            right: true,
                            ..Opts::default()
                        },
                    );
                    field_with(
                        &mut c[2],
                        "Opening bal. (₹)",
                        &mut s.form.opening_balance,
                        "0.00",
                        Opts {
                            right: true,
                            ..Opts::default()
                        },
                    );
                });

                manufacturers_section(ui, s, app);
                products_section(ui, s);

                ui.add_space(10.0);
                ui.checkbox(&mut s.form.active, "Supplier is active");
                ui.add_space(4.0);
            });

        ui.add_space(6.0);
        hairline(ui);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let label = if existing {
                "Save changes"
            } else {
                "Save supplier"
            };
            if button(ui, label, Kind::Primary).clicked() {
                save(s, app);
            }
            if button(ui, "Clear", Kind::Secondary).clicked() {
                s.new_form();
            }
            if existing {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (label, kind) = if s.form.active {
                        ("Deactivate", Kind::Danger)
                    } else {
                        ("Reactivate", Kind::Secondary)
                    };
                    if button(ui, label, kind).clicked() {
                        toggle_active(s, app);
                    }
                });
            }
        });

        // ── Toast ───────────────────────────────────────────────
        if !s.message.is_empty() {
            ui.add_space(6.0);
            toast(ui, s, now);
        }
    });
}

fn toast(ui: &mut egui::Ui, s: &mut State, now: f64) {
    let age = s.toast_age(now);
    let ttl = s.toast_ttl();
    // entry animation
    let entry = (age / 0.18).clamp(0.0, 1.0) as f32;
    let entry = entry * entry * (3.0 - 2.0 * entry);
    // exit fade in the last 0.25s
    let remaining = (ttl - age).max(0.0);
    let exit = (remaining / 0.25).clamp(0.0, 1.0) as f32;
    let alpha = (entry * exit).clamp(0.0, 1.0);
    let slide = (1.0 - entry) * 8.0;

    let (bg, fg, icon) = if s.error {
        (RED_BG, RED, "!")
    } else {
        (GREEN_BG, GREEN, "✓")
    };

    let full_w = ui.available_width();
    let mut dismiss = false;

    egui::Frame::new()
        .fill(bg.gamma_multiply(alpha))
        .stroke(Stroke::new(1.0, fg.gamma_multiply(0.35 * alpha)))
        .corner_radius(10.0)
        .inner_margin(Margin::symmetric(12, 9))
        .show(ui, |ui| {
            ui.set_min_width((full_w - 24.0).max(60.0));
            ui.set_width((full_w - 24.0).max(60.0));
            ui.horizontal(|ui| {
                // icon
                let (r, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                let c = r.center();
                ui.painter().circle_filled(c, 8.0, fg.gamma_multiply(alpha));
                ui.painter().text(
                    c,
                    Align2::CENTER_CENTER,
                    icon,
                    FontId::proportional(11.0),
                    Color32::WHITE.gamma_multiply(alpha),
                );
                ui.add_space(4.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("×")
                                    .size(16.0)
                                    .color(fg.gamma_multiply(alpha)),
                            )
                            .frame(false),
                        )
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .clicked()
                    {
                        dismiss = true;
                    }
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(&s.message)
                                    .size(12.5)
                                    .color(fg.gamma_multiply(alpha)),
                            )
                            .truncate(),
                        );
                    });
                });
            });
        });
    let _ = slide;
    if dismiss {
        s.message.clear();
        s.error = false;
        s.toast_at = 0.0;
    }
}

fn manufacturers_section(ui: &mut egui::Ui, s: &mut State, app: &AsterApp) {
    let total = s.manufacturers.len();
    let sub = if total == 0 {
        "None yet — add one below".to_owned()
    } else {
        format!("{} of {} selected", s.manufacturer_ids.len(), total)
    };
    section(ui, "MANUFACTURERS", &sub);

    ui.label(
        RichText::new("Add a manufacturer by name, or tick the ones this supplier carries.")
            .size(11.0)
            .color(MUTED),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let w = (ui.available_width() - 96.0).max(120.0);
        ui.add(
            egui::TextEdit::singleline(&mut s.new_manufacturer_name)
                .hint_text("e.g. Cipla Ltd")
                .desired_width(w)
                .margin(Margin::symmetric(10, 7)),
        );
        if button(ui, "+ Add", Kind::Secondary).clicked() {
            let name = s.new_manufacturer_name.trim().to_owned();
            if name.is_empty() {
                let now = ui.ctx().input(|i| i.time);
                s.toast("Enter a manufacturer name first.", true, now);
            } else {
                let result = app
                    .db
                    .as_ref()
                    .ok_or_else(|| "Database is not open.".to_owned())
                    .and_then(|db| db.create_manufacturer(app.selected_year, &name));
                let now = ui.ctx().input(|i| i.time);
                match result {
                    Ok(id) => {
                        s.new_manufacturer_name.clear();
                        reload(s, app);
                        if !s.manufacturer_ids.contains(&id) {
                            s.manufacturer_ids.push(id);
                        }
                        s.toast(format!("Manufacturer '{name}' created."), false, now);
                    }
                    Err(e) => s.toast(e, true, now),
                }
            }
        }
    });
    ui.add_space(4.0);
    list_box(
        ui,
        |ui| {
            if s.manufacturers.is_empty() {
                ui.label(
                    RichText::new("No manufacturers yet.")
                        .size(11.5)
                        .color(MUTED),
                );
            }
            for m in &s.manufacturers {
                let mut checked = s.manufacturer_ids.contains(&m.id);
                if ui.checkbox(&mut checked, &m.name).changed() {
                    if checked {
                        if !s.manufacturer_ids.contains(&m.id) {
                            s.manufacturer_ids.push(m.id);
                        }
                    } else {
                        s.manufacturer_ids.retain(|id| *id != m.id);
                    }
                }
            }
        },
        "supplier-mfr-list",
        130.0,
    );
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
            .hint_text("Filter products…")
            .desired_width(f32::INFINITY)
            .margin(Margin::symmetric(10, 7)),
    );
    ui.add_space(4.0);
    let q = s.product_filter.trim().to_lowercase();
    list_box(
        ui,
        |ui| {
            if s.products.is_empty() {
                ui.label(
                    RichText::new("No products found. Create products first in Product Master.")
                        .size(11.5)
                        .color(MUTED),
                );
            }
            for p in &s.products {
                if !q.is_empty()
                    && !p.name.to_lowercase().contains(&q)
                    && !p.manufacturer_name.to_lowercase().contains(&q)
                    && !p.brand_name.to_lowercase().contains(&q)
                {
                    continue;
                }
                let mut checked = s.product_ids.contains(&p.id);
                let label = if p.manufacturer_name.is_empty() {
                    format!(
                        "{}  ·  {}",
                        p.name,
                        if p.brand_name.is_empty() {
                            "No brand"
                        } else {
                            &p.brand_name
                        }
                    )
                } else {
                    format!("{}  ·  {}", p.name, p.manufacturer_name)
                };
                if ui.checkbox(&mut checked, label).changed() {
                    if checked {
                        if !s.product_ids.contains(&p.id) {
                            s.product_ids.push(p.id);
                        }
                    } else {
                        s.product_ids.retain(|id| *id != p.id);
                    }
                }
            }
        },
        "supplier-prod-list",
        180.0,
    );
}

fn list_box(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui), id: &str, max_h: f32) {
    egui::Frame::new()
        .fill(CANVAS)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(9.0)
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ScrollArea::vertical()
                .id_salt(id)
                .max_height(max_h)
                .auto_shrink([false, true])
                .show(ui, body);
        });
}

fn save(s: &mut State, app: &AsterApp) {
    let now = app_clock(app);
    let record = match s.form.to_record() {
        Ok(r) => r,
        Err(e) => {
            s.toast(e, true, now);
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
            reload(s, app);
            if let Some(r) = s.rows.iter().find(|r| r.id == id).cloned() {
                s.open(&r, app);
            }
            let now = app_clock(app);
            s.toast(
                "Supplier saved with its product and manufacturer links.",
                false,
                now,
            );
        }
        Err(e) => {
            let now = app_clock(app);
            s.toast(e, true, now);
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
        .and_then(|db| db.set_supplier_active(app.selected_year, id, target));
    match result {
        Ok(()) => {
            s.form.active = target;
            s.baseline.active = target;
            reload(s, app);
            let now = app_clock(app);
            let msg = if target {
                "Supplier reactivated."
            } else {
                "Supplier deactivated; historical records retained."
            };
            s.toast(msg, false, now);
        }
        Err(e) => {
            let now = app_clock(app);
            s.toast(e, true, now);
        }
    }
}

fn app_clock(_app: &AsterApp) -> f64 {
    // Placeholder: real callers pass ctx time. Kept for call-site symmetry.
    0.0
}

fn reload(s: &mut State, app: &AsterApp) {
    s.loaded = true;
    if let Some(db) = app.db.as_ref() {
        match db.suppliers(app.selected_year, &s.search) {
            Ok(rows) => s.rows = rows,
            Err(e) => {
                s.message = e;
                s.error = true;
                s.rows.clear();
            }
        }
        s.manufacturers = db.manufacturers(app.selected_year).unwrap_or_default();
        s.products = db.products(app.selected_year, "").unwrap_or_default();
    } else {
        s.rows.clear();
        s.manufacturers.clear();
        s.products.clear();
    }
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
    ui.add_space(2.0);
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
        ui.add_space(2.0);
        ui.label(RichText::new(n).size(11.0).color(AMBER));
    }
    ui.add_space(6.0);
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

fn card(margin: f32) -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(12.0)
        .inner_margin(margin)
}

fn stat_card(ui: &mut egui::Ui, w: f32, label: &str, value: String, accent: Color32) {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
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
    p.circle_stroke(c, 5.0, Stroke::new(1.6, color));
    p.line_segment(
        [c + Vec2::splat(3.6), c + Vec2::splat(7.4)],
        Stroke::new(1.8, color),
    );
}

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
