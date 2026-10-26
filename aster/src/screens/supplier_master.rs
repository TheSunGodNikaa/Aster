use crate::{
    app::AsterApp,
    database::{ManufacturerOption, ProductRecord, SupplierProductLink, SupplierRecord},
};
use eframe::egui::{
    self, Align, Color32, FontId, Id, Key, Layout, Margin, Pos2, Rect, RichText, Sense, Stroke,
    Vec2,
};
use std::collections::BTreeSet;

// ------------------------------------------------------------------ palette

const INK: Color32 = Color32::from_rgb(43, 39, 52);
const MUTED: Color32 = Color32::from_rgb(100, 94, 111);
const FAINT: Color32 = Color32::from_rgb(150, 145, 162);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const PURPLE_DEEP: Color32 = Color32::from_rgb(74, 50, 120);
const TINT: Color32 = Color32::from_rgb(243, 239, 249);
const BORDER: Color32 = Color32::from_rgb(226, 223, 232);
const BORDER_STRONG: Color32 = Color32::from_rgb(208, 201, 221);
const CANVAS: Color32 = Color32::from_rgb(248, 247, 251);
const RED: Color32 = Color32::from_rgb(176, 58, 58);
const RED_TINT: Color32 = Color32::from_rgb(252, 232, 232);
const GREEN: Color32 = Color32::from_rgb(38, 110, 72);
const GREEN_TINT: Color32 = Color32::from_rgb(232, 245, 238);
const AMBER: Color32 = Color32::from_rgb(170, 111, 22);
const AMBER_TINT: Color32 = Color32::from_rgb(253, 242, 222);

const ROW_H: f32 = 64.0;
const CHECK_ROW_H: f32 = 46.0;
const FOOTER_H: f32 = 60.0;

const STATES: &[(&str, &str)] = &[
    ("01", "Jammu and Kashmir"),
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
    ("26", "Dadra and Nagar Haveli and Daman and Diu"),
    ("27", "Maharashtra"),
    ("29", "Karnataka"),
    ("30", "Goa"),
    ("31", "Lakshadweep"),
    ("32", "Kerala"),
    ("33", "Tamil Nadu"),
    ("34", "Puducherry"),
    ("35", "Andaman and Nicobar Islands"),
    ("36", "Telangana"),
    ("37", "Andhra Pradesh"),
    ("38", "Ladakh"),
];

// -------------------------------------------------------------------- state

#[derive(Clone, Default)]
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
    fn blank() -> Self {
        Self {
            payment_terms_days: "0".into(),
            credit_limit: "0.00".into(),
            opening_balance: "0.00".into(),
            active: true,
            ..Self::default()
        }
    }

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

    fn fingerprint(&self) -> String {
        [
            self.name.trim(),
            self.trade_name.trim(),
            self.contact_person.trim(),
            self.phone.trim(),
            self.email.trim(),
            self.gstin.trim(),
            self.drug_license.trim(),
            self.address_line1.trim(),
            self.address_line2.trim(),
            self.city.trim(),
            self.state.trim(),
            self.state_code.trim(),
            self.pincode.trim(),
            self.payment_terms_days.trim(),
            self.credit_limit.trim(),
            self.opening_balance.trim(),
        ]
        .join("\u{1f}")
    }

    fn to_record(&self) -> Result<SupplierRecord, String> {
        let days = if self.payment_terms_days.trim().is_empty() {
            0
        } else {
            self.payment_terms_days
                .trim()
                .parse::<i64>()
                .map_err(|_| "Payment terms must be a whole number of days.".to_owned())?
        };
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
            phone: digits_only(&self.phone),
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

#[derive(Clone, Copy, PartialEq, Default)]
enum Filter {
    All,
    #[default]
    Active,
    Inactive,
}

#[derive(Clone, Copy, PartialEq, Default)]
enum Tab {
    #[default]
    Details,
    Manufacturers,
    Products,
}

#[derive(Clone, Default)]
struct State {
    loaded: bool,
    year: i32,
    rows: Vec<SupplierRecord>,
    manufacturers: Vec<ManufacturerOption>,
    products: Vec<ProductRecord>,

    form: Form,
    original: String,
    mfr_ids: BTreeSet<i64>,
    prod_ids: BTreeSet<i64>,
    original_mfr: BTreeSet<i64>,
    original_prod: BTreeSet<i64>,
    selected: Option<i64>,

    search: String,
    filter: Filter,
    tab: Tab,
    product_filter: String,
    selected_only: bool,
    new_manufacturer: String,

    submitted: bool,
    pending: Option<Option<i64>>,
    confirm_toggle: bool,
    changed_at: f64,
    focus: Option<Id>,
}

impl State {
    fn dirty(&self) -> bool {
        self.form.fingerprint() != self.original
            || self.mfr_ids != self.original_mfr
            || self.prod_ids != self.original_prod
    }

    fn load(&mut self, form: Form, mfr: BTreeSet<i64>, prod: BTreeSet<i64>, now: f64) {
        self.selected = form.id;
        self.original = form.fingerprint();
        self.form = form;
        self.original_mfr = mfr.clone();
        self.original_prod = prod.clone();
        self.mfr_ids = mfr;
        self.prod_ids = prod;
        self.submitted = false;
        self.pending = None;
        self.confirm_toggle = false;
        self.changed_at = now;
    }

    fn new_supplier(&mut self, now: f64) {
        self.load(Form::blank(), BTreeSet::new(), BTreeSet::new(), now);
        self.tab = Tab::Details;
        self.focus = Some(Id::new("sm_name"));
    }

    /// Opens a supplier row, reading its manufacturer and product links.
    fn open(&mut self, id: i64, app: &mut AsterApp, now: f64) {
        let Some(rec) = self.rows.iter().find(|r| r.id == id).cloned() else {
            self.new_supplier(now);
            return;
        };
        let year = app.selected_year;
        let (mfr, prod) = match app.db.as_ref() {
            Some(db) => {
                let m = db.supplier_manufacturer_ids(year, id);
                let p = db.supplier_product_links(year, id);
                match (m, p) {
                    (Ok(m), Ok(p)) => (
                        m.into_iter().collect::<BTreeSet<i64>>(),
                        p.into_iter().map(|l| l.product_id).collect::<BTreeSet<i64>>(),
                    ),
                    (m, p) => {
                        app.profile_message = m
                            .err()
                            .or_else(|| p.err())
                            .unwrap_or_else(|| "Could not read supplier links.".into());
                        (BTreeSet::new(), BTreeSet::new())
                    }
                }
            }
            None => (BTreeSet::new(), BTreeSet::new()),
        };
        self.load(Form::from_record(&rec), mfr, prod, now);
    }
}

// ------------------------------------------------------------------ helpers

fn anim(ctx: &egui::Context) -> f32 {
    ctx.data(|d| d.get_temp::<f32>(Id::new("aster_anim_time")))
        .unwrap_or(0.14)
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

fn digits_only(v: &str) -> String {
    v.chars().filter(char::is_ascii_digit).collect()
}

fn initials(name: &str) -> String {
    let s: String = name
        .split_whitespace()
        .filter_map(|w| w.chars().find(|c| c.is_alphanumeric()))
        .take(2)
        .collect::<String>()
        .to_uppercase();
    if s.is_empty() {
        "?".into()
    } else {
        s
    }
}

fn paint_text(ui: &egui::Ui, rect: Rect, text: &str, size: f32, color: Color32, strong: bool) {
    if text.is_empty() || rect.width() < 12.0 {
        return;
    }
    let size = if strong { size + 0.5 } else { size };
    let mut job = egui::text::LayoutJob::simple_singleline(
        text.to_owned(),
        FontId::proportional(size),
        color,
    );
    job.wrap.max_width = rect.width();
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let galley = ui.painter().layout_job(job);
    let pos = Pos2::new(rect.left(), rect.center().y - galley.size().y / 2.0);
    ui.painter()
        .with_clip_rect(rect.expand2(Vec2::new(0.0, 4.0)))
        .galley(pos, galley, color);
}

// --------------------------------------------------------------- validation

fn validate_name(v: &str) -> Option<String> {
    let v = v.trim();
    if v.is_empty() {
        Some("Legal name is required.".into())
    } else if v.chars().count() < 2 {
        Some("Use at least 2 characters.".into())
    } else {
        None
    }
}

fn validate_phone(v: &str) -> Option<String> {
    let d = digits_only(v);
    if d.is_empty() {
        return None;
    }
    let d = if d.len() == 12 && d.starts_with("91") {
        d[2..].to_owned()
    } else {
        d
    };
    if d.len() != 10 || !matches!(d.as_bytes()[0], b'6'..=b'9') {
        Some("Enter a 10-digit mobile number (starts with 6–9).".into())
    } else {
        None
    }
}

fn validate_email(v: &str) -> Option<String> {
    let v = v.trim();
    if v.is_empty() {
        return None;
    }
    let ok = v.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
            && !domain.ends_with('.')
            && !v.contains(' ')
    });
    (!ok).then(|| "Enter a valid email address.".into())
}

fn validate_gstin(v: &str) -> Option<String> {
    let v = v.trim().to_uppercase();
    if v.is_empty() {
        return None;
    }
    let b = v.as_bytes();
    let valid = b.len() == 15
        && b[0..2].iter().all(u8::is_ascii_digit)
        && STATES.iter().any(|(c, _)| *c == &v[0..2])
        && b[2..7].iter().all(u8::is_ascii_uppercase)
        && b[7..11].iter().all(u8::is_ascii_digit)
        && b[11].is_ascii_uppercase()
        && (b[12].is_ascii_digit() || b[12].is_ascii_uppercase())
        && b[13] == b'Z'
        && (b[14].is_ascii_digit() || b[14].is_ascii_uppercase());
    (!valid).then(|| "GSTIN looks wrong. Example: 33ABCDE1234F1Z5.".into())
}

fn validate_pincode(v: &str) -> Option<String> {
    let v = v.trim();
    if v.is_empty() {
        None
    } else if v.len() != 6 || !v.chars().all(|c| c.is_ascii_digit()) || v.starts_with('0') {
        Some("PIN code must be 6 digits.".into())
    } else {
        None
    }
}

fn validate_money(v: &str, allow_negative: bool) -> Option<String> {
    let v = v.trim().replace(',', "");
    if v.is_empty() {
        return None;
    }
    match v.parse::<f64>() {
        Ok(n) if n.is_finite() && (allow_negative || n >= 0.0) => None,
        Ok(_) => Some("Amount cannot be negative.".into()),
        Err(_) => Some("Enter a valid amount.".into()),
    }
}

fn validate_days(v: &str) -> Option<String> {
    let v = v.trim();
    if v.is_empty() {
        return None;
    }
    match v.parse::<i64>() {
        Ok(n) if (0..=3650).contains(&n) => None,
        _ => Some("Use whole days, 0 to 3650.".into()),
    }
}

fn state_mismatch(f: &Form) -> Option<String> {
    let g = f.gstin.trim();
    let mismatch = !f.state_code.is_empty() && g.get(0..2).is_some_and(|c| c != f.state_code);
    mismatch.then(|| "GSTIN state code does not match the selected state.".into())
}

struct Errors {
    name: Option<String>,
    phone: Option<String>,
    email: Option<String>,
    gstin: Option<String>,
    pincode: Option<String>,
    days: Option<String>,
    credit: Option<String>,
    opening: Option<String>,
}

impl Errors {
    fn of(f: &Form) -> Self {
        Self {
            name: validate_name(&f.name),
            phone: validate_phone(&f.phone),
            email: validate_email(&f.email),
            gstin: validate_gstin(&f.gstin).or_else(|| state_mismatch(f)),
            pincode: validate_pincode(&f.pincode),
            days: validate_days(&f.payment_terms_days),
            credit: validate_money(&f.credit_limit, false),
            opening: validate_money(&f.opening_balance, true),
        }
    }

    fn count(&self) -> usize {
        [
            &self.name,
            &self.phone,
            &self.email,
            &self.gstin,
            &self.pincode,
            &self.days,
            &self.credit,
            &self.opening,
        ]
        .iter()
        .filter(|e| e.is_some())
        .count()
    }
}

// -------------------------------------------------------------------- page

pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let ctx = ui.ctx().clone();
    let now = ui.input(|i| i.time);
    let key = Id::new("aster_supplier_master_state");
    let mut s = ctx.data_mut(|d| d.get_temp::<State>(key).unwrap_or_default());
    if !s.loaded || s.year != app.selected_year {
        s = State {
            year: app.selected_year,
            ..State::default()
        };
        reload(&mut s, app);
        s.loaded = true;
        s.new_supplier(now - 10.0);
        s.focus = None;
    }

    let (new_key, save_key, find_key) = ctx.input(|i| {
        (
            i.modifiers.command && i.key_pressed(Key::N),
            i.modifiers.command && i.key_pressed(Key::S),
            i.modifiers.command && i.key_pressed(Key::F),
        )
    });
    if new_key {
        s.new_supplier(now);
    }
    if save_key {
        save(&mut s, app, now);
    }
    if find_key {
        ctx.memory_mut(|m| m.request_focus(Id::new("sm_search")));
    }

    let intro = ctx.animate_bool_with_time(Id::new("sm_intro"), true, anim(&ctx));
    let page_top = ui.cursor().top();

    ui.scope(|ui| {
        ui.set_opacity(intro);
        apply_style(ui);

        // ---- header
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("MASTERS  ·  ACCOUNTS PAYABLE")
                        .size(10.5)
                        .strong()
                        .color(PURPLE),
                );
                ui.add_space(2.0);
                ui.label(RichText::new("Supplier Master").size(26.0).strong().color(INK));
                ui.add_space(1.0);
                ui.label(
                    RichText::new(format!(
                        "Distributors, the manufacturers they carry and the medicines they supply  ·  FY {}–{}",
                        app.selected_year,
                        app.selected_year + 1
                    ))
                    .size(12.5)
                    .color(MUTED),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if button(ui, "＋  New supplier", 138.0, 36.0, Kind::Primary, "sm_new")
                    .on_hover_text("Start a new supplier  (Ctrl+N)")
                    .clicked()
                {
                    s.new_supplier(now);
                }
                if button(ui, "↻  Refresh", 100.0, 36.0, Kind::Ghost, "sm_refresh").clicked() {
                    reload(&mut s, app);
                    app.profile_message = "Supplier data refreshed successfully.".into();
                }
            });
        });
        ui.add_space(12.0);

        // ---- stats
        let total = s.rows.len();
        let active = s.rows.iter().filter(|r| r.active).count();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            stat_chip(ui, &total.to_string(), "Suppliers", PURPLE, TINT);
            stat_chip(ui, &active.to_string(), "Active", GREEN, GREEN_TINT);
            stat_chip(
                ui,
                &(total - active).to_string(),
                "Inactive",
                MUTED,
                Color32::from_rgb(240, 238, 245),
            );
            stat_chip(
                ui,
                &s.manufacturers.len().to_string(),
                "Manufacturers",
                AMBER,
                AMBER_TINT,
            );
        });
        ui.add_space(14.0);

        // ---- body, sized to the page so nothing is pushed off-screen
        let free = ui.available_height();
        let body_h = if free.is_finite() && free < 6000.0 {
            free
        } else {
            ctx.content_rect().bottom() - page_top - 130.0
        }
        .max(440.0);
        let total_w = ui.available_width();
        let gap = 16.0;
        let list_w = (total_w * 0.38).clamp(320.0, 460.0);
        let form_w = (total_w - list_w - gap).max(480.0);

        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            list_panel(ui, &mut s, app, list_w, body_h, now);
            editor_panel(ui, &mut s, app, form_w, body_h, now);
        });
    });

    ctx.data_mut(|d| d.insert_temp(key, s));
}

fn apply_style(ui: &mut egui::Ui) {
    let style = ui.style_mut();
    style.spacing.item_spacing = Vec2::new(9.0, 7.0);
    style.spacing.interact_size.y = 34.0;
    style.visuals.extreme_bg_color = Color32::WHITE;
    style.visuals.selection.bg_fill = mix(Color32::WHITE, PURPLE, 0.22);
    style.visuals.selection.stroke = Stroke::new(1.0_f32, PURPLE);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_STRONG);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, mix(BORDER_STRONG, PURPLE, 0.6));
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, PURPLE);
    for w in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
    ] {
        w.corner_radius = 8.0.into();
    }
}

fn card<R>(ui: &mut egui::Ui, width: f32, height: f32, content: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(14.0)
        .inner_margin(Margin::symmetric(18, 16))
        .show(ui, |ui| {
            ui.set_width(width - 38.0);
            ui.set_height(height - 34.0);
            content(ui)
        })
        .inner
}

// -------------------------------------------------------------------- list

fn visible<'a>(rows: &'a [SupplierRecord], search: &str, filter: Filter) -> Vec<&'a SupplierRecord> {
    let needle = search.trim().to_lowercase();
    let mut out: Vec<&SupplierRecord> = rows
        .iter()
        .filter(|r| match filter {
            Filter::All => true,
            Filter::Active => r.active,
            Filter::Inactive => !r.active,
        })
        .filter(|r| {
            needle.is_empty()
                || format!(
                    "{} {} {} {} {} {}",
                    r.name, r.trade_name, r.contact_person, r.phone, r.gstin, r.city
                )
                .to_lowercase()
                .contains(&needle)
        })
        .collect();
    out.sort_by_key(|r| r.name.to_lowercase());
    out
}

fn list_panel(ui: &mut egui::Ui, s: &mut State, app: &mut AsterApp, w: f32, h: f32, now: f64) {
    let mut clicked: Option<i64> = None;
    card(ui, w, h, |ui| {
        let shown = visible(&s.rows, &s.search, s.filter);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Supplier directory").size(16.0).strong().color(INK));
            ui.label(RichText::new(format!("{} shown", shown.len())).size(12.0).color(MUTED));
        });
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::singleline(&mut s.search)
                .id(Id::new("sm_search"))
                .hint_text("Search name, contact, phone, GSTIN, city…  (Ctrl+F)")
                .margin(Margin::symmetric(10, 8))
                .desired_width(ui.available_width()),
        );
        ui.add_space(8.0);
        segmented(
            ui,
            &mut s.filter,
            &[(Filter::Active, "Active"), (Filter::Inactive, "Inactive"), (Filter::All, "All")],
            "sm_filter",
            84.0,
        );
        ui.add_space(8.0);

        if shown.is_empty() {
            let rect = ui.available_rect_before_wrap();
            let (title, detail) = if s.rows.is_empty() {
                ("No suppliers yet", "Fill in the form and press Save supplier.")
            } else {
                ("No matching suppliers", "Try a different search or filter.")
            };
            ui.painter().text(
                rect.center() - Vec2::new(0.0, 28.0),
                egui::Align2::CENTER_CENTER,
                title,
                FontId::proportional(15.0),
                INK,
            );
            ui.painter().text(
                rect.center() - Vec2::new(0.0, 6.0),
                egui::Align2::CENTER_CENTER,
                detail,
                FontId::proportional(12.5),
                MUTED,
            );
            return;
        }

        let selected = s.selected;
        ui.scope(|ui| {
            // must be set before show_rows reads the spacing
            ui.spacing_mut().item_spacing.y = 0.0;
            egui::ScrollArea::vertical()
                .id_salt("sm_directory_scroll")
                .auto_shrink([false, false])
                .show_rows(ui, ROW_H, shown.len(), |ui, range| {
                    for r in &shown[range] {
                        if supplier_row(ui, r, selected == Some(r.id)) {
                            clicked = Some(r.id);
                        }
                    }
                });
        });
    });

    if let Some(id) = clicked {
        if Some(id) != s.selected {
            if s.dirty() {
                s.pending = Some(Some(id));
            } else {
                s.open(id, app, now);
            }
        }
    }
}

fn supplier_row(ui: &mut egui::Ui, r: &SupplierRecord, selected: bool) -> bool {
    let ctx = ui.ctx().clone();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_H), Sense::click());
    let hot = ctx.animate_bool_with_time(response.id, response.hovered(), anim(&ctx));
    let sel = ctx.animate_bool_with_time(response.id.with("sel"), selected, anim(&ctx));
    let inner = rect.shrink2(Vec2::new(0.0, 3.0));
    ui.painter()
        .rect_filled(inner, 10.0, mix(mix(Color32::TRANSPARENT, CANVAS, hot), TINT, sel));
    if sel > 0.0 {
        ui.painter().rect_filled(
            Rect::from_min_size(
                Pos2::new(inner.left(), inner.top() + 10.0),
                Vec2::new(3.0, inner.height() - 20.0),
            ),
            1.5,
            PURPLE.gamma_multiply(sel),
        );
    }

    let center = Pos2::new(inner.left() + 30.0, inner.center().y);
    ui.painter()
        .circle_filled(center, 18.0, if r.active { TINT } else { Color32::from_rgb(238, 236, 242) });
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        initials(&r.name),
        FontId::proportional(12.5),
        if r.active { PURPLE } else { FAINT },
    );

    let right_w = 76.0;
    let text_left = inner.left() + 58.0;
    let text_w = inner.right() - right_w - text_left - 10.0;
    paint_text(
        ui,
        Rect::from_min_size(Pos2::new(text_left, inner.top() + 9.0), Vec2::new(text_w, 18.0)),
        &r.name,
        13.5,
        if r.active { INK } else { MUTED },
        true,
    );
    let lead = if !r.trade_name.is_empty() {
        r.trade_name.as_str()
    } else if !r.phone.is_empty() {
        r.phone.as_str()
    } else {
        "No phone"
    };
    let sub = if r.city.is_empty() {
        lead.to_owned()
    } else {
        format!("{lead}  ·  {}", r.city)
    };
    paint_text(
        ui,
        Rect::from_min_size(Pos2::new(text_left, inner.top() + 30.0), Vec2::new(text_w, 16.0)),
        &sub,
        11.5,
        MUTED,
        false,
    );

    let (label, fg, bg) = if r.active {
        ("ACTIVE", GREEN, GREEN_TINT)
    } else {
        ("INACTIVE", RED, RED_TINT)
    };
    let g = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(9.5), fg);
    let pill = Rect::from_min_size(
        Pos2::new(inner.right() - g.size().x - 18.0, inner.center().y - 9.0),
        Vec2::new(g.size().x + 14.0, 18.0),
    );
    ui.painter().rect_filled(pill, 9.0, bg);
    ui.painter().galley(
        Pos2::new(pill.left() + 7.0, pill.center().y - g.size().y / 2.0),
        g,
        fg,
    );

    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
}

// ------------------------------------------------------------------ editor

fn editor_panel(ui: &mut egui::Ui, s: &mut State, app: &mut AsterApp, w: f32, h: f32, now: f64) {
    let ctx = ui.ctx().clone();
    let fade = if anim(&ctx) == 0.0 {
        1.0
    } else {
        0.4 + 0.6 * ease_out(((now - s.changed_at) / 0.22).clamp(0.0, 1.0) as f32)
    };
    if fade < 1.0 {
        ctx.request_repaint();
    }

    ui.scope(|ui| {
        ui.set_opacity(fade);
        card(ui, w, h, |ui| {
            let editing = s.form.id.is_some();
            let dirty = s.dirty();

            // ---- header
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
                let name = s.form.name.trim();
                ui.painter().circle_filled(rect.center(), 20.0, TINT);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    if name.is_empty() { "+".to_owned() } else { initials(name) },
                    FontId::proportional(14.0),
                    PURPLE,
                );
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    let title_w = (ui.available_width() - 150.0).max(100.0);
                    let (title_rect, _) = ui.allocate_exact_size(Vec2::new(title_w, 22.0), Sense::hover());
                    paint_text(
                        ui,
                        title_rect,
                        if name.is_empty() { "New supplier" } else { name },
                        16.0,
                        INK,
                        true,
                    );
                    let sub = match s.form.id {
                        Some(id) => format!("Supplier no. S{id:04}"),
                        None => "Fields marked * are required".to_owned(),
                    };
                    ui.label(RichText::new(sub).size(12.0).color(MUTED));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let t = ctx.animate_bool_with_time(Id::new("sm_dirty"), dirty, anim(&ctx));
                    if t > 0.0 {
                        pill(ui, "UNSAVED CHANGES", AMBER.gamma_multiply(t), AMBER_TINT.gamma_multiply(t));
                    } else if editing {
                        if s.form.active {
                            pill(ui, "ACTIVE", GREEN, GREEN_TINT);
                        } else {
                            pill(ui, "INACTIVE", RED, RED_TINT);
                        }
                    }
                });
            });

            // ---- discard banner
            if let Some(target) = s.pending {
                ui.add_space(8.0);
                egui::Frame::new()
                    .fill(AMBER_TINT)
                    .corner_radius(9.0)
                    .inner_margin(Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Discard your unsaved changes?")
                                    .size(12.5)
                                    .color(AMBER),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if button(ui, "Keep editing", 104.0, 28.0, Kind::Ghost, "sm_keep").clicked() {
                                    s.pending = None;
                                }
                                if button(ui, "Discard", 84.0, 28.0, Kind::Danger, "sm_discard").clicked() {
                                    match target {
                                        Some(id) => s.open(id, app, now),
                                        None => s.new_supplier(now),
                                    }
                                }
                            });
                        });
                    });
            }

            ui.add_space(10.0);
            let mfr_label = format!("Manufacturers  {}", s.mfr_ids.len());
            let prod_label = format!("Products  {}", s.prod_ids.len());
            segmented(
                ui,
                &mut s.tab,
                &[
                    (Tab::Details, "Details"),
                    (Tab::Manufacturers, mfr_label.as_str()),
                    (Tab::Products, prod_label.as_str()),
                ],
                "sm_tab",
                ((ui.available_width() - 6.0) / 3.0).clamp(110.0, 170.0),
            );
            ui.add_space(8.0);

            // ---- body
            let body_h = (ui.available_height() - FOOTER_H).max(120.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), body_h),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.set_min_height(body_h);
                    match s.tab {
                        Tab::Details => {
                            egui::ScrollArea::vertical()
                                .id_salt("sm_details_scroll")
                                .auto_shrink([false, false])
                                .max_height(body_h)
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.y = 0.0;
                                    details(ui, s);
                                });
                        }
                        Tab::Manufacturers => manufacturers_tab(ui, s, app),
                        Tab::Products => products_tab(ui, s),
                    }
                },
            );

            // ---- footer
            ui.separator();
            ui.add_space(6.0);
            footer(ui, s, app, now);
        });
    });
}

fn pill(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) {
    let g = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(10.0), fg);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(g.size().x + 18.0, 22.0), Sense::hover());
    ui.painter().rect_filled(rect, 11.0, bg);
    ui.painter().galley(
        Pos2::new(rect.left() + 9.0, rect.center().y - g.size().y / 2.0),
        g,
        fg,
    );
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(title).size(10.5).strong().color(PURPLE));
    ui.add_space(6.0);
}

fn details(ui: &mut egui::Ui, s: &mut State) {
    let errors = Errors::of(&s.form);
    let sub = s.submitted;
    let focus = s.focus.take();
    let full = ui.available_width();
    let f = &mut s.form;

    section(ui, "IDENTITY & CONTACT");
    field(ui, Id::new("sm_name"), "LEGAL NAME", true, &mut f.name, full, "Registered business name", &errors.name, sub, focus, false);
    two(ui, [1.0, 1.0], |ui, w| {
        field(ui, Id::new("sm_trade"), "TRADE NAME", false, &mut f.trade_name, w[0], "Name they trade under", &None, sub, focus, false);
        field(ui, Id::new("sm_contact"), "CONTACT PERSON", false, &mut f.contact_person, w[1], "Primary contact", &None, sub, focus, false);
    });
    two(ui, [1.0, 1.15], |ui, w| {
        field(ui, Id::new("sm_phone"), "PHONE", false, &mut f.phone, w[0], "10-digit mobile", &errors.phone, sub, focus, false);
        field(ui, Id::new("sm_email"), "EMAIL", false, &mut f.email, w[1], "name@company.com", &errors.email, sub, focus, false);
    });

    section(ui, "TAX & LICENCE");
    two(ui, [1.25, 1.0], |ui, w| {
        let r = field(ui, Id::new("sm_gstin"), "GSTIN", false, &mut f.gstin, w[0], "33ABCDE1234F1Z5", &errors.gstin, sub, focus, false);
        if r.changed() {
            f.gstin = f.gstin.to_uppercase().replace(' ', "");
            f.gstin.truncate(15);
            if let Some(code) = f.gstin.get(0..2) {
                if let Some((c, n)) = STATES.iter().find(|(c, _)| *c == code) {
                    f.state = (*n).to_owned();
                    f.state_code = (*c).to_owned();
                }
            }
        }
        field(ui, Id::new("sm_dl"), "DRUG LICENCE NO.", false, &mut f.drug_license, w[1], "Licence reference(s)", &None, sub, focus, false);
    });

    section(ui, "ADDRESS");
    field(ui, Id::new("sm_a1"), "ADDRESS LINE 1", false, &mut f.address_line1, full, "Door no., street, area", &None, sub, focus, false);
    field(ui, Id::new("sm_a2"), "ADDRESS LINE 2", false, &mut f.address_line2, full, "Landmark / additional address", &None, sub, focus, false);
    two(ui, [1.3, 1.0], |ui, w| {
        field(ui, Id::new("sm_city"), "CITY", false, &mut f.city, w[0], "City / town", &None, sub, focus, false);
        field(ui, Id::new("sm_pin"), "PIN CODE", false, &mut f.pincode, w[1], "6 digits", &errors.pincode, sub, focus, false);
    });
    state_picker(ui, f);

    section(ui, "CREDIT & PAYMENT TERMS");
    two3(ui, |ui, w| {
        let r = field(ui, Id::new("sm_days"), "TERMS (DAYS)", false, &mut f.payment_terms_days, w, "0", &errors.days, sub, focus, true);
        if r.changed() {
            f.payment_terms_days.retain(|c| c.is_ascii_digit());
        }
    }, |ui, w| {
        let r = field(ui, Id::new("sm_credit"), "CREDIT LIMIT (₹)", false, &mut f.credit_limit, w, "0.00", &errors.credit, sub, focus, true);
        if r.changed() {
            f.credit_limit.retain(|c| c.is_ascii_digit() || c == '.' || c == ',');
        }
    }, |ui, w| {
        let r = field(ui, Id::new("sm_open"), "OPENING BAL. (₹)", false, &mut f.opening_balance, w, "0.00", &errors.opening, sub, focus, true);
        if r.changed() {
            f.opening_balance
                .retain(|c| c.is_ascii_digit() || c == '.' || c == ',' || c == '-');
        }
    });
    ui.label(
        RichText::new("Opening balance: positive means you owe the supplier; negative means an advance paid.")
            .size(11.0)
            .color(FAINT),
    );
    ui.add_space(10.0);
}

fn two(ui: &mut egui::Ui, weights: [f32; 2], content: impl FnOnce(&mut egui::Ui, [f32; 2])) {
    let gap = 12.0;
    let unit = (ui.available_width() - gap) / (weights[0] + weights[1]);
    let w = [weights[0] * unit, weights[1] * unit];
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        content(ui, w);
    });
}

fn two3(
    ui: &mut egui::Ui,
    a: impl FnOnce(&mut egui::Ui, f32),
    b: impl FnOnce(&mut egui::Ui, f32),
    c: impl FnOnce(&mut egui::Ui, f32),
) {
    let gap = 12.0;
    let w = (ui.available_width() - gap * 2.0) / 3.0;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        a(ui, w);
        b(ui, w);
        c(ui, w);
    });
}

#[allow(clippy::too_many_arguments)]
fn field(
    ui: &mut egui::Ui,
    id: Id,
    title: &str,
    required: bool,
    value: &mut String,
    width: f32,
    hint: &str,
    error: &Option<String>,
    submitted: bool,
    focus: Option<Id>,
    right: bool,
) -> egui::Response {
    let ctx = ui.ctx().clone();
    ui.vertical(|ui| {
        ui.set_width(width);
        ui.spacing_mut().item_spacing.y = 3.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            ui.label(RichText::new(title).size(10.0).strong().color(MUTED));
            if required {
                ui.label(RichText::new("*").size(10.0).strong().color(RED));
            }
        });
        if focus == Some(id) {
            ctx.memory_mut(|m| m.request_focus(id));
        }
        let response = ui.add(
            egui::TextEdit::singleline(value)
                .id(id)
                .hint_text(hint)
                .margin(Margin::symmetric(10, 8))
                .horizontal_align(if right { Align::RIGHT } else { Align::LEFT })
                .desired_width(width),
        );
        let touched_id = id.with("touched");
        if response.lost_focus() {
            ui.data_mut(|d| d.insert_temp(touched_id, true));
        }
        let touched: bool = ui.data(|d| d.get_temp(touched_id)).unwrap_or(false);
        let show = error.is_some() && (submitted || touched);
        let t = ctx.animate_bool_with_time(id.with("err"), show, anim(&ctx));
        if t > 0.0 {
            ui.painter().rect_stroke(
                response.rect.expand(1.0),
                8.0,
                Stroke::new(1.5_f32, RED.gamma_multiply(t)),
                egui::StrokeKind::Outside,
            );
        }
        // reserved line: the form never jumps when a message appears
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 16.0), Sense::hover());
        let msg_id = id.with("msg");
        if let Some(m) = error {
            ui.data_mut(|d| d.insert_temp(msg_id, m.clone()));
        }
        if t > 0.0 {
            let text = error
                .clone()
                .or_else(|| ui.data(|d| d.get_temp::<String>(msg_id)))
                .unwrap_or_default();
            paint_text(ui, rect, &text, 11.5, RED.gamma_multiply(t), false);
        }
        response
    })
    .inner
}

fn state_picker(ui: &mut egui::Ui, f: &mut Form) {
    let gap = 12.0;
    let unit = (ui.available_width() - gap) / 2.3;
    let (w0, w1) = (unit * 1.3, unit);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.vertical(|ui| {
            ui.set_width(w0);
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.label(RichText::new("STATE").size(10.0).strong().color(MUTED));
            let selected = if f.state.is_empty() {
                "Select state".to_owned()
            } else {
                f.state.clone()
            };
            egui::ComboBox::from_id_salt("sm_state")
                .width(w0 - 8.0)
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        for (code, name) in STATES {
                            if ui
                                .selectable_label(f.state_code == *code, format!("{code}  ·  {name}"))
                                .clicked()
                            {
                                f.state = (*name).to_owned();
                                f.state_code = (*code).to_owned();
                            }
                        }
                    });
                });
            ui.add_space(16.0);
        });
        ui.vertical(|ui| {
            ui.set_width(w1);
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.label(RichText::new("GST STATE CODE").size(10.0).strong().color(MUTED));
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w1, 34.0), Sense::hover());
            ui.painter().rect_filled(rect, 8.0, CANVAS);
            ui.painter()
                .rect_stroke(rect, 8.0, Stroke::new(1.0_f32, BORDER), egui::StrokeKind::Inside);
            paint_text(
                ui,
                rect.shrink2(Vec2::new(10.0, 0.0)),
                if f.state_code.is_empty() { "Auto" } else { &f.state_code },
                13.0,
                if f.state_code.is_empty() { FAINT } else { INK },
                false,
            );
            ui.add_space(16.0);
        });
    });
}

// -------------------------------------------------------- manufacturers tab

fn manufacturers_tab(ui: &mut egui::Ui, s: &mut State, app: &mut AsterApp) {
    ui.label(
        RichText::new("Tick the manufacturers this supplier carries, or add a new one by name.")
            .size(12.0)
            .color(MUTED),
    );
    ui.add_space(8.0);
    let mut add = false;
    ui.horizontal(|ui| {
        let w = (ui.available_width() - 100.0).max(120.0);
        let r = ui.add(
            egui::TextEdit::singleline(&mut s.new_manufacturer)
                .id(Id::new("sm_new_mfr"))
                .hint_text("New manufacturer, e.g. Cipla Ltd")
                .margin(Margin::symmetric(10, 8))
                .desired_width(w),
        );
        if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            add = true;
        }
        if button(ui, "＋  Add", 88.0, 34.0, Kind::Ghost, "sm_add_mfr").clicked() {
            add = true;
        }
    });
    if add {
        let name = s.new_manufacturer.trim().to_owned();
        if name.is_empty() {
            app.profile_message = "Enter a manufacturer name first.".into();
        } else if s
            .manufacturers
            .iter()
            .any(|m| m.name.trim().eq_ignore_ascii_case(&name))
        {
            app.profile_message = format!("Manufacturer {name} already exists. Tick it below.");
        } else {
            let year = app.selected_year;
            let result = app
                .db
                .as_ref()
                .ok_or_else(|| "Database is not available.".to_owned())
                .and_then(|db| db.create_manufacturer(year, &name));
            match result {
                Ok(id) => {
                    s.new_manufacturer.clear();
                    refresh_lists(s, app);
                    s.mfr_ids.insert(id);
                    app.profile_message = format!("Manufacturer {name} created successfully.");
                }
                Err(e) => app.profile_message = e,
            }
        }
    }
    ui.add_space(8.0);

    if s.manufacturers.is_empty() {
        empty_note(ui, "No manufacturers yet", "Add the first one above.");
        return;
    }
    let State {
        manufacturers,
        mfr_ids,
        ..
    } = s;
    ui.scope(|ui| {
        // must be set before show_rows reads the spacing
        ui.spacing_mut().item_spacing.y = 0.0;
        egui::ScrollArea::vertical()
            .id_salt("sm_mfr_scroll")
            .auto_shrink([false, false])
            .show_rows(ui, CHECK_ROW_H, manufacturers.len(), |ui, range| {
                for m in &manufacturers[range] {
                    let mut on = mfr_ids.contains(&m.id);
                    if check_row(ui, &mut on, &m.name, "", Id::new(("sm_mfr", m.id))) {
                        if on {
                            mfr_ids.insert(m.id);
                        } else {
                            mfr_ids.remove(&m.id);
                        }
                    }
                }
            });
    });
}

// -------------------------------------------------------------- products tab

fn products_tab(ui: &mut egui::Ui, s: &mut State) {
    ui.label(
        RichText::new("Link medicines from Product Master. A product can have several suppliers.")
            .size(12.0)
            .color(MUTED),
    );
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let w = (ui.available_width() - 250.0).max(120.0);
        ui.add(
            egui::TextEdit::singleline(&mut s.product_filter)
                .hint_text("Filter by name, brand or company…")
                .margin(Margin::symmetric(10, 8))
                .desired_width(w),
        );
        let label = if s.selected_only { "Selected only ✓" } else { "Selected only" };
        if button(
            ui,
            label,
            124.0,
            34.0,
            if s.selected_only { Kind::Tinted } else { Kind::Ghost },
            "sm_sel_only",
        )
        .clicked()
        {
            s.selected_only = !s.selected_only;
        }
        if button(ui, "Clear all", 90.0, 34.0, Kind::Ghost, "sm_clear_prod").clicked() {
            s.prod_ids.clear();
        }
    });
    ui.add_space(8.0);

    let needle = s.product_filter.trim().to_lowercase();
    let selected_only = s.selected_only;
    let State { products, prod_ids, .. } = s;
    let mut rows: Vec<&ProductRecord> = products
        .iter()
        .filter(|p| {
            (!selected_only || prod_ids.contains(&p.id))
                && (needle.is_empty()
                    || format!("{} {} {}", p.name, p.brand_name, p.manufacturer_name)
                        .to_lowercase()
                        .contains(&needle))
        })
        .collect();
    rows.sort_by_key(|p| p.name.to_lowercase());

    if rows.is_empty() {
        if products.is_empty() {
            empty_note(ui, "No products yet", "Create products in Product Master first.");
        } else {
            empty_note(ui, "No matching products", "Try a different filter.");
        }
        return;
    }
    ui.scope(|ui| {
        // must be set before show_rows reads the spacing
        ui.spacing_mut().item_spacing.y = 0.0;
        egui::ScrollArea::vertical()
            .id_salt("sm_prod_scroll")
            .auto_shrink([false, false])
            .show_rows(ui, CHECK_ROW_H, rows.len(), |ui, range| {
                for p in &rows[range] {
                    let mut on = prod_ids.contains(&p.id);
                    let sub = if p.manufacturer_name.is_empty() {
                        p.brand_name.clone()
                    } else {
                        p.manufacturer_name.clone()
                    };
                    if check_row(ui, &mut on, &p.name, &sub, Id::new(("sm_prod", p.id))) {
                        if on {
                            prod_ids.insert(p.id);
                        } else {
                            prod_ids.remove(&p.id);
                        }
                    }
                }
            });
    });
}

fn empty_note(ui: &mut egui::Ui, title: &str, detail: &str) {
    let rect = ui.available_rect_before_wrap();
    ui.painter().text(
        rect.center() - Vec2::new(0.0, 14.0),
        egui::Align2::CENTER_CENTER,
        title,
        FontId::proportional(14.5),
        INK,
    );
    ui.painter().text(
        rect.center() + Vec2::new(0.0, 8.0),
        egui::Align2::CENTER_CENTER,
        detail,
        FontId::proportional(12.5),
        MUTED,
    );
}

/// Painted checkbox row. Returns true when toggled.
fn check_row(ui: &mut egui::Ui, on: &mut bool, title: &str, sub: &str, id: Id) -> bool {
    let ctx = ui.ctx().clone();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), CHECK_ROW_H), Sense::hover());
    let response = ui.interact(rect, id, Sense::click());
    let hot = ctx.animate_bool_with_time(id.with("hot"), response.hovered(), anim(&ctx));
    let t = ctx.animate_bool_with_time(id.with("on"), *on, anim(&ctx));
    let inner = rect.shrink2(Vec2::new(0.0, 2.0));
    ui.painter()
        .rect_filled(inner, 8.0, mix(mix(Color32::TRANSPARENT, CANVAS, hot), TINT, t * 0.6));

    let boxr = Rect::from_center_size(Pos2::new(inner.left() + 20.0, inner.center().y), Vec2::splat(18.0));
    ui.painter()
        .rect_filled(boxr, 5.0, mix(Color32::WHITE, PURPLE, t));
    ui.painter().rect_stroke(
        boxr,
        5.0,
        Stroke::new(1.2_f32, mix(BORDER_STRONG, PURPLE, t)),
        egui::StrokeKind::Inside,
    );
    if t > 0.0 {
        let c = boxr.center();
        let s = Stroke::new(1.8_f32, Color32::WHITE.gamma_multiply(t));
        ui.painter()
            .line_segment([c + Vec2::new(-4.0, 0.5), c + Vec2::new(-1.2, 3.5)], s);
        ui.painter()
            .line_segment([c + Vec2::new(-1.2, 3.5), c + Vec2::new(4.5, -3.0)], s);
    }

    let left = inner.left() + 40.0;
    let w = inner.right() - left - 8.0;
    if sub.is_empty() {
        paint_text(
            ui,
            Rect::from_min_size(Pos2::new(left, inner.top()), Vec2::new(w, inner.height())),
            title,
            13.0,
            INK,
            false,
        );
    } else {
        paint_text(
            ui,
            Rect::from_min_size(Pos2::new(left, inner.top() + 5.0), Vec2::new(w, 18.0)),
            title,
            13.0,
            INK,
            false,
        );
        paint_text(
            ui,
            Rect::from_min_size(Pos2::new(left, inner.top() + 23.0), Vec2::new(w, 15.0)),
            sub,
            11.0,
            MUTED,
            false,
        );
    }
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.clicked() {
        *on = !*on;
        true
    } else {
        false
    }
}

// ------------------------------------------------------------------ footer

fn footer(ui: &mut egui::Ui, s: &mut State, app: &mut AsterApp, now: f64) {
    let editing = s.form.id.is_some();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let label = if editing { "Save changes" } else { "Save supplier" };
        if button(ui, label, 150.0, 38.0, Kind::Primary, "sm_save")
            .on_hover_text("Save  (Ctrl+S)")
            .clicked()
        {
            save(s, app, now);
        }
        if s.dirty()
            && button(ui, "Reset", 80.0, 38.0, Kind::Ghost, "sm_reset")
                .on_hover_text("Undo your edits")
                .clicked()
        {
            match s.form.id {
                Some(id) => s.open(id, app, now),
                None => s.new_supplier(now),
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if editing {
                let active = s.form.active;
                let (label, kind) = if s.confirm_toggle {
                    (
                        if active { "Confirm deactivate" } else { "Confirm reactivate" },
                        Kind::Danger,
                    )
                } else if active {
                    ("Deactivate", Kind::Ghost)
                } else {
                    ("Reactivate", Kind::Ghost)
                };
                if button(ui, label, 150.0, 38.0, kind, "sm_toggle").clicked() {
                    if s.confirm_toggle {
                        toggle_active(s, app, now);
                    } else {
                        s.confirm_toggle = true;
                    }
                }
                if s.confirm_toggle
                    && button(ui, "Cancel", 76.0, 38.0, Kind::Ghost, "sm_toggle_cancel").clicked()
                {
                    s.confirm_toggle = false;
                }
            }
        });
    });
}

// ---------------------------------------------------------------- widgets

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Primary,
    Ghost,
    Tinted,
    Danger,
}

fn button(ui: &mut egui::Ui, text: &str, width: f32, height: f32, kind: Kind, salt: &str) -> egui::Response {
    let ctx = ui.ctx().clone();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let response = ui.interact(rect, Id::new(("sm_btn", salt)), Sense::click());
    let t = ctx.animate_bool_with_time(response.id, response.hovered(), anim(&ctx));
    let r = if response.is_pointer_button_down_on() {
        rect.shrink(0.75)
    } else {
        rect
    };
    let (fill, stroke, fg) = match kind {
        Kind::Primary => (mix(PURPLE, PURPLE_DEEP, t), Color32::TRANSPARENT, Color32::WHITE),
        Kind::Ghost => (
            mix(Color32::WHITE, TINT, t),
            mix(BORDER_STRONG, PURPLE, t * 0.6),
            mix(INK, PURPLE, t),
        ),
        Kind::Tinted => (mix(TINT, Color32::from_rgb(234, 228, 245), t), PURPLE.gamma_multiply(0.5), PURPLE),
        Kind::Danger => (
            mix(Color32::from_rgb(252, 240, 240), Color32::from_rgb(248, 226, 226), t),
            RED.gamma_multiply(0.5),
            RED,
        ),
    };
    ui.painter().rect_filled(r, 9.0, fill);
    if stroke != Color32::TRANSPARENT {
        ui.painter()
            .rect_stroke(r, 9.0, Stroke::new(1.0_f32, stroke), egui::StrokeKind::Inside);
    }
    ui.painter().text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        text,
        FontId::proportional(13.0),
        fg,
    );
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn segmented<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    value: &mut T,
    options: &[(T, &str)],
    salt: &str,
    seg_w: f32,
) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(seg_w * options.len() as f32 + 6.0, 36.0),
        Sense::hover(),
    );
    ui.painter().rect_filled(rect, 10.0, Color32::from_rgb(240, 238, 245));
    let ctx = ui.ctx().clone();
    let index = options.iter().position(|o| o.0 == *value).unwrap_or(0) as f32;
    let x = ctx.animate_value_with_time(Id::new(("sm_seg", salt)), index, anim(&ctx) * 1.6);
    let highlight = Rect::from_min_size(
        Pos2::new(rect.left() + 3.0 + x * seg_w, rect.top() + 3.0),
        Vec2::new(seg_w, 30.0),
    );
    ui.painter().rect_filled(highlight, 8.0, Color32::WHITE);
    ui.painter()
        .rect_stroke(highlight, 8.0, Stroke::new(1.0_f32, BORDER), egui::StrokeKind::Inside);
    for (i, (option, text)) in options.iter().enumerate() {
        let seg = Rect::from_min_size(
            Pos2::new(rect.left() + 3.0 + i as f32 * seg_w, rect.top() + 3.0),
            Vec2::new(seg_w, 30.0),
        );
        let response = ui.interact(seg, Id::new(("sm_seg_item", salt, i)), Sense::click());
        ui.painter().text(
            seg.center(),
            egui::Align2::CENTER_CENTER,
            text,
            FontId::proportional(12.5),
            if *option == *value { PURPLE } else { MUTED },
        );
        if response.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            *value = *option;
        }
    }
}

fn stat_chip(ui: &mut egui::Ui, value: &str, label: &str, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(10.0)
        .inner_margin(Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                ui.label(RichText::new(value).size(15.0).strong().color(fg));
                ui.label(RichText::new(label).size(12.5).color(fg));
            });
        });
}

// ------------------------------------------------------------------ actions

fn save(s: &mut State, app: &mut AsterApp, now: f64) {
    s.submitted = true;
    let errors = Errors::of(&s.form);
    if errors.count() > 0 {
        s.tab = Tab::Details;
        app.profile_message = format!(
            "Please correct the highlighted fields ({} to fix).",
            errors.count()
        );
        return;
    }
    let record = match s.form.to_record() {
        Ok(r) => r,
        Err(e) => {
            app.profile_message = e;
            return;
        }
    };

    let name = record.name.to_lowercase();
    for r in &s.rows {
        if Some(r.id) == s.form.id {
            continue;
        }
        if !record.gstin.is_empty() && r.gstin.eq_ignore_ascii_case(&record.gstin) {
            app.profile_message = format!("GSTIN {} already belongs to {}.", record.gstin, r.name);
            return;
        }
        if r.name.trim().to_lowercase() == name {
            app.profile_message = format!("A supplier named {} already exists.", r.name);
            return;
        }
    }

    let links: Vec<SupplierProductLink> = s
        .prod_ids
        .iter()
        .map(|id| SupplierProductLink {
            product_id: *id,
            ..SupplierProductLink::default()
        })
        .collect();
    let mfr: Vec<i64> = s.mfr_ids.iter().copied().collect();
    let year = app.selected_year;
    let id = s.form.id;
    let result = app
        .db
        .as_ref()
        .ok_or_else(|| "Database is not available.".to_owned())
        .and_then(|db| db.save_supplier(year, id, &record, &mfr, &links));
    match result {
        Ok(saved_id) => {
            reload(s, app);
            s.open(saved_id, app, now);
            app.profile_message = format!("Supplier {} saved successfully.", record.name);
        }
        Err(e) => app.profile_message = e,
    }
}

fn toggle_active(s: &mut State, app: &mut AsterApp, now: f64) {
    let Some(id) = s.form.id else { return };
    let target = !s.form.active;
    let year = app.selected_year;
    let result = app
        .db
        .as_ref()
        .ok_or_else(|| "Database is not available.".to_owned())
        .and_then(|db| db.set_supplier_active(year, id, target));
    match result {
        Ok(()) => {
            let name = s.form.name.clone();
            reload(s, app);
            s.open(id, app, now);
            app.profile_message = format!(
                "Supplier {name} {} successfully.",
                if target { "reactivated" } else { "deactivated" }
            );
        }
        Err(e) => {
            s.confirm_toggle = false;
            app.profile_message = e;
        }
    }
}

fn refresh_lists(s: &mut State, app: &mut AsterApp) {
    let year = app.selected_year;
    if let Some(db) = app.db.as_ref() {
        match db.manufacturers(year) {
            Ok(m) => s.manufacturers = m,
            Err(e) => app.profile_message = e,
        }
        match db.products(year, "") {
            Ok(p) => s.products = p,
            Err(e) => app.profile_message = e,
        }
    }
}

fn reload(s: &mut State, app: &mut AsterApp) {
    let year = app.selected_year;
    match app.db.as_ref() {
        Some(db) => match db.suppliers(year, "") {
            Ok(rows) => s.rows = rows,
            Err(e) => {
                s.rows.clear();
                app.profile_message = e;
            }
        },
        None => s.rows.clear(),
    }
    refresh_lists(s, app);
}