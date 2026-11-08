use crate::{
    app::AsterApp,
    database::{
        ManufacturerOption, NewPurchaseProductDraft, ProductRecord, PurchaseDraft,
        PurchaseLineDraft, PurchaseSummary, SupplierRecord, UnitOption,
    },
};
use eframe::egui::{
    self, Align, Color32, FontId, Id, Key, Layout, Margin, Modifiers, Pos2, Rect, RichText, Sense,
    Stroke, Vec2,
};

// ------------------------------------------------------------------ palette

const INK: Color32 = Color32::from_rgb(43, 39, 52);
const MUTED: Color32 = Color32::from_rgb(100, 94, 111);
const FAINT: Color32 = Color32::from_rgb(150, 145, 162);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const PURPLE_DEEP: Color32 = Color32::from_rgb(74, 50, 120);
const BORDER: Color32 = Color32::from_rgb(226, 223, 232);
const BORDER_STRONG: Color32 = Color32::from_rgb(208, 201, 221);
const GREEN: Color32 = Color32::from_rgb(38, 110, 72);
const RED: Color32 = Color32::from_rgb(176, 58, 58);
const AMBER: Color32 = Color32::from_rgb(170, 111, 22);
const CANVAS: Color32 = Color32::from_rgb(248, 247, 251);
const TINT: Color32 = Color32::from_rgb(243, 239, 249);

const RECENT_W: f32 = 336.0;
const WIDE_BREAKPOINT: f32 = 1120.0;
const FOOTER_H: f32 = 100.0;
const RECENT_ROW_H: f32 = 64.0;
const COLUMN_GAP: f32 = 16.0;

// -------------------------------------------------------------------- state

#[derive(Clone, Debug)]
struct LineForm {
    uid: u64,
    born: f64,
    product_id: i64,
    product_query: String,
    /// A product typed in through "New product". It is created when the
    /// purchase is posted; `product_id` is a negative placeholder until then.
    new_product: Option<NewPurchaseProductDraft>,
    batch_no: String,
    /// Typed as MM/YY (also accepts MM/YYYY and YYYY-MM-DD).
    expiry: String,
    qty: String,
    free_qty: String,
    rate: String,
    discount: String,
    mrp: String,
    sale_rate: String,
}

impl LineForm {
    fn new(uid: u64, born: f64) -> Self {
        Self {
            uid,
            born,
            product_id: 0,
            product_query: String::new(),
            new_product: None,
            batch_no: String::new(),
            expiry: String::new(),
            qty: "1".into(),
            free_qty: "0".into(),
            rate: "0.00".into(),
            discount: "0".into(),
            mrp: "0.00".into(),
            sale_rate: "0.00".into(),
        }
    }

    fn is_blank(&self) -> bool {
        self.product_id == 0
            && self.new_product.is_none()
            && self.product_query.trim().is_empty()
            && self.batch_no.trim().is_empty()
            && self.expiry.trim().is_empty()
    }
}

/// What a line currently points at: an existing product or a pending new one.
struct Resolved<'a> {
    name: &'a str,
    gst_percent: f64,
    hsn: &'a str,
    maker: &'a str,
    is_new: bool,
}

fn resolve<'a>(line: &'a LineForm, products: &'a [ProductRecord]) -> Option<Resolved<'a>> {
    if let Some(n) = &line.new_product {
        return Some(Resolved {
            name: n.name.as_str(),
            gst_percent: n.gst_percent,
            hsn: n.hsn_code.as_str(),
            maker: "",
            is_new: true,
        });
    }
    if line.product_id <= 0 {
        return None;
    }
    products
        .iter()
        .find(|p| p.id == line.product_id)
        .map(|p| Resolved {
            name: p.name.as_str(),
            gst_percent: p.gst_percent,
            hsn: p.hsn_code.as_str(),
            maker: p.manufacturer_name.as_str(),
            is_new: false,
        })
}

#[derive(Clone, Copy, PartialEq, Default)]
enum Tab {
    #[default]
    Entry,
    Recent,
}

#[derive(Clone, Debug, Default)]
struct NewProductForm {
    target_uid: u64,
    born: f64,

    name: String,
    brand_name: String,
    generic_name: String,
    composition: String,

    manufacturer_id: Option<i64>,
    unit_id: Option<i64>,

    hsn_code: String,
    gst_percent: String,
    schedule: String,
    barcode: String,
    reorder_level: String,

    error: String,
}

impl NewProductForm {
    fn blank(target_uid: u64, name: String, now: f64) -> Self {
        Self {
            target_uid,
            born: now,
            name,
            reorder_level: "0".into(),
            ..Default::default()
        }
    }

    fn from_draft(target_uid: u64, d: &NewPurchaseProductDraft, now: f64) -> Self {
        Self {
            target_uid,
            born: now,
            name: d.name.clone(),
            brand_name: d.brand_name.clone(),
            generic_name: d.generic_name.clone(),
            composition: d.composition.clone(),
            manufacturer_id: d.manufacturer_id,
            unit_id: d.unit_id,
            hsn_code: d.hsn_code.clone(),
            gst_percent: format!("{}", d.gst_percent),
            schedule: d.schedule.clone(),
            barcode: d.barcode.clone(),
            reorder_level: format!("{}", d.reorder_level),
            error: String::new(),
        }
    }

    fn validate(
        &self,
        manufacturers: &[ManufacturerOption],
        units: &[UnitOption],
    ) -> Result<NewPurchaseProductDraft, String> {
        let name = self.name.trim();

        if name.is_empty() {
            return Err("Product name is required.".into());
        }

        let hsn_code = self.hsn_code.trim();

        if !matches!(hsn_code.len(), 4 | 6 | 8) || !hsn_code.chars().all(|c| c.is_ascii_digit()) {
            return Err("Enter a valid 4, 6 or 8 digit HSN code.".into());
        }

        let gst_percent: f64 = self
            .gst_percent
            .trim()
            .parse()
            .map_err(|_| "Enter a valid GST percentage.")?;

        if !gst_percent.is_finite() || !(0.0..=100.0).contains(&gst_percent) {
            return Err("GST percentage must be between 0 and 100.".into());
        }

        let reorder_text = self.reorder_level.trim();
        let reorder_level: f64 = if reorder_text.is_empty() {
            0.0
        } else {
            reorder_text
                .parse()
                .map_err(|_| "Enter a valid reorder level.")?
        };

        if !reorder_level.is_finite() || reorder_level < 0.0 {
            return Err("Reorder level cannot be negative.".into());
        }

        let manufacturer_id = self
            .manufacturer_id
            .filter(|id| manufacturers.iter().any(|m| m.id == *id))
            .ok_or("Select a valid manufacturer.")?;

        let unit_id = self
            .unit_id
            .filter(|id| units.iter().any(|u| u.id == *id))
            .ok_or("Select a valid unit.")?;

        Ok(NewPurchaseProductDraft {
            name: name.to_string(),
            brand_name: self.brand_name.trim().to_string(),
            generic_name: self.generic_name.trim().to_string(),
            composition: self.composition.trim().to_string(),
            manufacturer_id: Some(manufacturer_id),
            unit_id: Some(unit_id),
            hsn_code: hsn_code.to_string(),
            gst_percent,
            schedule: self.schedule.trim().to_string(),
            barcode: self.barcode.trim().to_string(),
            reorder_level,
        })
    }
}

#[derive(Clone, Default)]
struct State {
    loaded: bool,
    suppliers: Vec<SupplierRecord>,
    products: Vec<ProductRecord>,
    recent: Vec<PurchaseSummary>,
    manufacturers: Vec<ManufacturerOption>,
    units: Vec<UnitOption>,
    new_product_form: Option<NewProductForm>,
    supplier_id: i64,
    supplier_invoice_no: String,
    /// DD/MM/YYYY, the same format as the working date.
    invoice_date: String,
    notes: String,
    lines: Vec<LineForm>,
    next_uid: u64,
    message: String,
    error: bool,
    submitted: bool,
    confirm_clear: bool,
    tab: Tab,
    focus: Option<Id>,
    scroll_to_last: bool,
}

impl State {
    fn new_line(&mut self, now: f64) -> LineForm {
        self.next_uid += 1;
        LineForm::new(self.next_uid, now)
    }

    fn reset(&mut self, working_date: &str, now: f64) {
        self.supplier_id = 0;
        self.supplier_invoice_no.clear();
        self.invoice_date = working_date_dmy(working_date);
        self.notes.clear();
        let line = self.new_line(now);
        self.focus = Some(prod_id(line.uid));
        self.lines = vec![line];
        self.submitted = false;
        self.confirm_clear = false;
        self.new_product_form = None;
    }

    fn add_line(&mut self, now: f64) {
        let line = self.new_line(now);
        self.focus = Some(prod_id(line.uid));
        self.lines.push(line);
        self.scroll_to_last = true;
        self.confirm_clear = false;
    }
}

fn prod_id(uid: u64) -> Id {
    Id::new(("pe_prod", uid))
}

fn field_id(uid: u64, k: usize) -> Id {
    Id::new(("pe_field", uid, k))
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

/// Eases a displayed money value towards its target so totals count up and
/// down smoothly instead of jumping.
fn tween_money(ctx: &egui::Context, key: &str, target: i64, now: f64) -> i64 {
    let id = Id::new(("pe_tween", key));
    let (mut from, mut to, mut start): (i64, i64, f64) = ctx
        .data(|d| d.get_temp(id))
        .unwrap_or((target, target, now));
    let duration = (anim(ctx) * 2.0) as f64;
    let shown = |from: i64, to: i64, start: f64| -> (i64, bool) {
        if duration <= 0.0 {
            return (to, false);
        }
        let t = ((now - start) / duration).clamp(0.0, 1.0);
        let e = 1.0 - (1.0 - t).powi(3);
        (from + ((to - from) as f64 * e).round() as i64, t < 1.0)
    };
    if to != target {
        from = shown(from, to, start).0;
        to = target;
        start = now;
    }
    let (value, running) = shown(from, to, start);
    if running {
        ctx.request_repaint();
    }
    ctx.data_mut(|d| d.insert_temp(id, (from, to, start)));
    value
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

/// Indian digit grouping: ₹12,34,567.89
fn money(paise: i64) -> String {
    let negative = paise < 0;
    let p = paise.unsigned_abs();
    let rupees = (p / 100).to_string();
    let frac = p % 100;
    let grouped = if rupees.len() <= 3 {
        rupees
    } else {
        let (head, tail) = rupees.split_at(rupees.len() - 3);
        let mut parts: Vec<String> = Vec::new();
        let chars: Vec<char> = head.chars().collect();
        let mut end = chars.len();
        while end > 0 {
            let start = end.saturating_sub(2);
            parts.push(chars[start..end].iter().collect());
            end = start;
        }
        parts.reverse();
        format!("{},{}", parts.join(","), tail)
    };
    format!(
        "{}₹{}.{:02}",
        if negative { "-" } else { "" },
        grouped,
        frac
    )
}

fn qty_text(milli: i64) -> String {
    if milli % 1000 == 0 {
        (milli / 1000).to_string()
    } else {
        format!("{:.3}", milli as f64 / 1000.0)
    }
}

/// A small right-aligned text link. Returns true when clicked.
fn link(ui: &mut egui::Ui, rect: Rect, text: &str, id: Id) -> bool {
    let ctx = ui.ctx().clone();
    let response = ui.interact(rect, id, Sense::click());
    let hot = ctx.animate_bool_with_time(id.with("hot"), response.hovered(), anim(&ctx));
    let color = mix(PURPLE, PURPLE_DEEP, hot);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(11.5), color);
    let pos = Pos2::new(
        rect.right() - galley.size().x,
        rect.center().y - galley.size().y / 2.0,
    );
    let underline_y = pos.y + galley.size().y;
    let width = galley.size().x;
    ui.painter().galley(pos, galley, color);
    if hot > 0.0 {
        ui.painter().line_segment(
            [
                Pos2::new(pos.x, underline_y),
                Pos2::new(pos.x + width, underline_y),
            ],
            Stroke::new(1.0_f32, color.gamma_multiply(hot)),
        );
    }
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
}

// -------------------------------------------------------------------- dates

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        _ => 30,
    }
}

fn month_abbr(month: u32) -> &'static str {
    [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .get(month.saturating_sub(1) as usize)
    .copied()
    .unwrap_or("")
}

/// DD/MM/YYYY or YYYY-MM-DD → (year, month, day).
fn parse_date(value: &str) -> Option<(i32, u32, u32)> {
    let v = value.trim();
    let (year, month, day) = if v.contains('/') {
        let mut p = v.split('/');
        let d = p.next()?.trim().parse::<u32>().ok()?;
        let m = p.next()?.trim().parse::<u32>().ok()?;
        let y = p.next()?.trim().parse::<i32>().ok()?;
        if p.next().is_some() {
            return None;
        }
        (y, m, d)
    } else {
        let mut p = v.split('-');
        let y = p.next()?.parse::<i32>().ok()?;
        let m = p.next()?.parse::<u32>().ok()?;
        let d = p.next()?.parse::<u32>().ok()?;
        if p.next().is_some() {
            return None;
        }
        (y, m, d)
    };
    if !(1900..=2200).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=days_in_month(year, month)).contains(&day)
    {
        return None;
    }
    Some((year, month, day))
}

fn working_date_dmy(date_text: &str) -> String {
    if let Some((y, m, d)) = parse_date(date_text) {
        return format!("{d:02}/{m:02}/{y:04}");
    }
    let (y, m, d) = crate::date::local_date();
    format!("{d:02}/{m:02}/{y:04}")
}

/// MM/YY, MM/YYYY, YYYY-MM or YYYY-MM-DD → (year, month).
fn parse_expiry(value: &str) -> Option<(i32, u32)> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }
    if v.contains('/') {
        let mut p = v.split('/');
        let m = p.next()?.trim().parse::<u32>().ok()?;
        let ys = p.next()?.trim();
        if p.next().is_some() || !(1..=12).contains(&m) {
            return None;
        }
        let y = ys.parse::<i32>().ok()?;
        let year = match ys.len() {
            2 => 2000 + y,
            4 => y,
            _ => return None,
        };
        return (2000..=2200).contains(&year).then_some((year, m));
    }
    let mut p = v.split('-');
    let y = p.next()?.parse::<i32>().ok()?;
    let m = p.next()?.parse::<u32>().ok()?;
    if !(1..=12).contains(&m) || !(2000..=2200).contains(&y) {
        return None;
    }
    if let Some(d) = p.next() {
        let d = d.parse::<u32>().ok()?;
        if !(1..=days_in_month(y, m)).contains(&d) {
            return None;
        }
    }
    Some((y, m))
}

fn expiry_iso(ym: (i32, u32)) -> String {
    format!("{:04}-{:02}-{:02}", ym.0, ym.1, days_in_month(ym.0, ym.1))
}

fn months_between(from: (i32, u32), to: (i32, u32)) -> i32 {
    (to.0 - from.0) * 12 + to.1 as i32 - from.1 as i32
}

/// Inserts the slash while typing: `0827` → `08/27`, `01052026` → `01/05/2026`.
/// Returns true when the text was rewritten (the caret must then be moved).
fn auto_slash(value: &mut String, groups: &[usize]) -> bool {
    if value.contains('/') || value.contains('-') {
        return false;
    }
    let digits: String = value.chars().filter(char::is_ascii_digit).collect();
    let mut out = String::new();
    let mut taken = 0;
    for (i, size) in groups.iter().enumerate() {
        if taken >= digits.len() {
            break;
        }
        if i > 0 {
            out.push('/');
        }
        let end = (taken + size).min(digits.len());
        out.push_str(&digits[taken..end]);
        taken = end;
    }
    if (digits.len() >= groups[0] || digits.len() < value.len()) && out != *value {
        *value = out;
        true
    } else {
        false
    }
}

/// After rewriting a text box programmatically, keep the caret at the end so
/// the next typed digit lands after the inserted slash.
fn cursor_to_end(ctx: &egui::Context, id: Id, text: &str) {
    if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
        let end = egui::text::CCursor::new(text.chars().count());
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(end)));
        egui::TextEdit::store_state(ctx, id, state);
    }
}

// ------------------------------------------------------------------ parsing

fn parse_money(v: &str) -> Result<i64, String> {
    let n = if v.trim().is_empty() {
        0.0
    } else {
        v.trim()
            .replace(',', "")
            .parse::<f64>()
            .map_err(|_| format!("Invalid money amount: {v}"))?
    };
    if !n.is_finite() || n < 0.0 || n > (i64::MAX / 100) as f64 {
        return Err("Money amount is invalid or out of range.".into());
    }
    Ok((n * 100.0).round() as i64)
}

fn parse_qty(v: &str, label: &str, allow_zero: bool) -> Result<i64, String> {
    let n = if v.trim().is_empty() {
        0.0
    } else {
        v.trim()
            .parse::<f64>()
            .map_err(|_| format!("{label} must be a number."))?
    };
    if !n.is_finite() || n < 0.0 || (!allow_zero && n <= 0.0) || n > 9_000_000_000.0 {
        return Err(format!("{label} is outside the allowed range."));
    }
    let milli = (n * 1000.0).round();
    if (n * 1000.0 - milli).abs() > 0.000001 {
        return Err(format!("{label} supports at most three decimal places."));
    }
    Ok(milli as i64)
}

fn parse_discount(v: &str) -> Result<i64, String> {
    let n = if v.trim().is_empty() {
        0.0
    } else {
        v.trim()
            .parse::<f64>()
            .map_err(|_| "Discount must be a number.".to_owned())?
    };
    if !n.is_finite() || !(0.0..=100.0).contains(&n) {
        return Err("Discount must be between 0 and 100%.".into());
    }
    Ok((n * 100.0).round() as i64)
}

// --------------------------------------------------------------- line maths

#[derive(Default, Clone, Copy)]
struct Calc {
    taxable: i64,
    tax: i64,
    net_unit: f64,
    margin: Option<f64>,
}

fn line_calc(line: &LineForm, products: &[ProductRecord]) -> Option<Calc> {
    let (Ok(q), Ok(rate), Ok(discount)) = (
        parse_qty(&line.qty, "Quantity", false),
        parse_money(&line.rate),
        parse_discount(&line.discount),
    ) else {
        return None;
    };
    let gst_percent = resolve(line, products)?.gst_percent;
    let gross = (q as i128 * rate as i128 / 1000) as i64;
    let base = ((gross as i128 * (10000 - discount) as i128 + 5000) / 10000) as i64;
    let gst = (gst_percent * 100.0).round() as i64;
    let tax = ((base as i128 * gst as i128 + 5000) / 10000) as i64;
    let net_unit = rate as f64 * (10000 - discount) as f64 / 10000.0;
    let sale = parse_money(&line.sale_rate).unwrap_or(0);
    let margin = (sale > 0 && net_unit > 0.0).then(|| (sale as f64 - net_unit) / net_unit * 100.0);
    Some(Calc {
        taxable: base,
        tax,
        net_unit,
        margin,
    })
}

#[derive(Default, Clone)]
struct LineErr {
    product: bool,
    batch: bool,
    expiry: bool,
    qty: bool,
    free: bool,
    rate: bool,
    disc: bool,
    mrp: bool,
    sale: bool,
    first: Option<String>,
}

impl LineErr {
    fn any(&self) -> bool {
        self.first.is_some()
    }
}

fn mark(flag: &mut bool, first: &mut Option<String>, message: &str) {
    *flag = true;
    if first.is_none() {
        *first = Some(message.to_owned());
    }
}

fn line_errors(line: &LineForm, products: &[ProductRecord], invoice_ym: (i32, u32)) -> LineErr {
    let mut e = LineErr::default();
    let mut first = None;
    if resolve(line, products).is_none() {
        mark(
            &mut e.product,
            &mut first,
            "select a product from the list or create a new one.",
        );
    }
    if line.batch_no.trim().is_empty() {
        mark(&mut e.batch, &mut first, "enter the batch number.");
    }
    match parse_expiry(&line.expiry) {
        None => mark(
            &mut e.expiry,
            &mut first,
            "enter a valid expiry such as 08/27.",
        ),
        Some(ym) if months_between(invoice_ym, ym) < 0 => {
            mark(&mut e.expiry, &mut first, "this batch is already expired.")
        }
        _ => {}
    }
    if let Err(m) = parse_qty(&line.qty, "Quantity", false) {
        mark(&mut e.qty, &mut first, &m.to_lowercase());
    }
    if let Err(m) = parse_qty(&line.free_qty, "Free quantity", true) {
        mark(&mut e.free, &mut first, &m.to_lowercase());
    }
    if parse_money(&line.rate).is_err() {
        mark(&mut e.rate, &mut first, "enter a valid purchase rate.");
    }
    if let Err(m) = parse_discount(&line.discount) {
        mark(&mut e.disc, &mut first, &m.to_lowercase());
    }
    let mrp = parse_money(&line.mrp);
    let sale = parse_money(&line.sale_rate);
    if mrp.is_err() {
        mark(&mut e.mrp, &mut first, "enter a valid MRP.");
    }
    if sale.is_err() {
        mark(&mut e.sale, &mut first, "enter a valid sale rate.");
    }
    if let (Ok(m), Ok(s)) = (mrp, sale) {
        if m > 0 && s > m {
            mark(
                &mut e.sale,
                &mut first,
                "sale rate cannot be higher than MRP.",
            );
        }
    }
    e.first = first;
    e
}

struct Totals {
    lines: usize,
    qty_milli: i64,
    taxable: i64,
    tax: i64,
}

fn totals(s: &State) -> Totals {
    let mut t = Totals {
        lines: 0,
        qty_milli: 0,
        taxable: 0,
        tax: 0,
    };
    for l in &s.lines {
        if let Some(c) = line_calc(l, &s.products) {
            t.lines += 1;
            t.qty_milli = t
                .qty_milli
                .saturating_add(parse_qty(&l.qty, "Quantity", false).unwrap_or(0));
            t.taxable = t.taxable.saturating_add(c.taxable);
            t.tax = t.tax.saturating_add(c.tax);
        }
    }
    t
}

fn invoice_ym(s: &State, app: &AsterApp) -> (i32, u32) {
    parse_date(&s.invoice_date)
        .or_else(|| parse_date(&app.date_text))
        .map(|(y, m, _)| (y, m))
        .unwrap_or_else(|| {
            let (y, m, _) = crate::date::local_date();
            (y, m)
        })
}

/// The left column: a scrolling form with the totals bar pinned underneath,
/// so Post is always on screen however many items are added.
fn entry_column(ui: &mut egui::Ui, s: &mut State, app: &mut AsterApp, w: f32, h: f32, now: f64) {
    ui.set_width(w);
    let scroll_h = (h - FOOTER_H - 12.0).max(220.0);

    egui::ScrollArea::vertical()
        .id_salt("pe_entry_scroll")
        .auto_shrink([false, false])
        .min_scrolled_height(scroll_h)
        .max_height(scroll_h)
        .show(ui, |ui| {
            ui.set_width(w);
            ui.spacing_mut().item_spacing.y = 12.0;
            invoice_card(ui, s, app, w, now);
            items_card(ui, s, app, w, now);
            ui.add_space(4.0);
        });

    ui.add_space(12.0);
    footer_card(ui, s, app, w, now);
}

// -------------------------------------------------------------------- page

pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let ctx = ui.ctx().clone();
    let now = ui.input(|i| i.time);
    let key = Id::new(("purchase-entry", app.selected_year));
    let mut s = ctx.data_mut(|d| d.get_temp::<State>(key).unwrap_or_default());

    if !s.loaded {
        s.loaded = true;
        s.reset(&app.date_text, now - 10.0);
        reload(&mut s, app);
    }

    // Shortcuts are ignored while the create-product dialog is open.
    let dialog_open = s.new_product_form.is_some();
    let (post_key, add_key) = ctx.input(|i| {
        (
            i.modifiers.command && i.key_pressed(Key::Enter),
            i.modifiers.command && i.key_pressed(Key::N),
        )
    });
    if add_key && !dialog_open {
        s.add_line(now);
    }
    if post_key && !dialog_open {
        post(&mut s, app, now);
    }

    let intro = ctx.animate_bool_with_time(Id::new("pe_intro"), true, anim(&ctx));
    let page_top = ui.cursor().top();

    ui.scope(|ui| {
        ui.set_opacity(intro);
        apply_style(ui);

        // ---- header
        let total_w = ui.available_width();
        let wide = total_w >= WIDE_BREAKPOINT;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("TRANSACTIONS  ·  INVENTORY INWARD")
                        .size(10.5)
                        .strong()
                        .color(PURPLE),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new("Purchase Entry")
                        .size(26.0)
                        .strong()
                        .color(INK),
                );
                ui.add_space(1.0);
                ui.label(
                    RichText::new(format!(
                        "Financial year {}–{}   ·   Working date {}",
                        app.selected_year,
                        app.selected_year + 1,
                        working_date_dmy(&app.date_text)
                    ))
                    .size(12.5)
                    .color(MUTED),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if button(
                    ui,
                    "↻  Refresh",
                    104.0,
                    34.0,
                    Style::Ghost,
                    true,
                    "pe_refresh",
                )
                .clicked()
                {
                    reload(&mut s, app);
                    s.message = "Suppliers, products and recent purchases refreshed.".into();
                    s.error = false;
                }
                if !wide {
                    ui.add_space(4.0);
                    segmented(
                        ui,
                        &mut s.tab,
                        &[(Tab::Entry, "New purchase"), (Tab::Recent, "Recent")],
                    );
                }
            });
        });
        ui.add_space(14.0);

        // ---- body
        let free = ui.available_height();
        let body_h = if free.is_finite() && free < 6000.0 {
            free
        } else {
            ctx.content_rect().bottom() - page_top - 96.0
        }
        .max(420.0);

        if wide {
            let entry_w = total_w - RECENT_W - COLUMN_GAP;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = COLUMN_GAP;
                ui.vertical(|ui| {
                    entry_column(ui, &mut s, app, entry_w, body_h, now);
                });
                recent_panel(ui, &s, RECENT_W, body_h);
            });
        } else if s.tab == Tab::Entry {
            ui.vertical(|ui| {
                entry_column(ui, &mut s, app, total_w, body_h, now);
            });
        } else {
            recent_panel(ui, &s, total_w, body_h);
        }

        render_new_product_dialog(&ctx, &mut s, now);
    });

    ctx.data_mut(|d| d.insert_temp(key, s));
}

/// Page-local look. Scoped to this page; nothing leaks into other screens.
fn apply_style(ui: &mut egui::Ui) {
    let style = ui.style_mut();
    style.spacing.item_spacing = Vec2::new(9.0, 7.0);
    style.spacing.button_padding = Vec2::new(13.0, 7.0);
    style.spacing.interact_size.y = 32.0;
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

// ------------------------------------------------------------- entry panel

fn card<R>(
    ui: &mut egui::Ui,
    width: f32,
    height: Option<f32>,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(12.0)
        .inner_margin(Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.set_width(width - 34.0);
            if let Some(h) = height {
                ui.set_height(h - 30.0);
            }
            content(ui)
        })
        .inner
}

fn label(ui: &mut egui::Ui, text: &str, required: bool) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.label(RichText::new(text).size(10.0).strong().color(MUTED));
        if required {
            ui.label(RichText::new("*").size(10.0).strong().color(RED));
        }
    });
}

fn outline_error(ui: &egui::Ui, rect: Rect, show: bool, id: Id) {
    let ctx = ui.ctx();
    let t = ctx.animate_bool_with_time(id, show, anim(ctx));
    if t > 0.0 {
        ui.painter().rect_stroke(
            rect.expand(1.0),
            8.0,
            Stroke::new(1.5_f32, RED.gamma_multiply(t)),
            egui::StrokeKind::Outside,
        );
    }
}

fn invoice_card(ui: &mut egui::Ui, s: &mut State, app: &AsterApp, w: f32, _now: f64) {
    card(ui, w, None, |ui| {
        let avail = ui.available_width();
        let gap = 12.0;
        let weights = [2.3_f32, 1.5, 1.2, 1.7];
        let unit = (avail - gap * 3.0) / weights.iter().sum::<f32>();
        let widths: Vec<f32> = weights.iter().map(|x| x * unit).collect();
        let submitted = s.submitted;

        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;

            // supplier
            ui.vertical(|ui| {
                ui.set_width(widths[0]);
                label(ui, "SUPPLIER", true);
                let missing = submitted && s.supplier_id == 0;
                let rect = supplier_combo(ui, &s.suppliers, &mut s.supplier_id, widths[0]);
                outline_error(ui, rect, missing, Id::new("pe_err_supplier"));
            });

            // invoice number
            ui.vertical(|ui| {
                ui.set_width(widths[1]);
                label(ui, "SUPPLIER INVOICE NO.", true);
                let r = ui.add(
                    egui::TextEdit::singleline(&mut s.supplier_invoice_no)
                        .id(Id::new("pe_inv_no"))
                        .hint_text("As printed on invoice")
                        .margin(Margin::symmetric(10, 7))
                        .desired_width(widths[1]),
                );
                outline_error(
                    ui,
                    r.rect,
                    submitted && s.supplier_invoice_no.trim().is_empty(),
                    Id::new("pe_err_invno"),
                );
            });

            // date
            ui.vertical(|ui| {
                ui.set_width(widths[2]);
                label(ui, "INVOICE DATE", true);
                let date_id = Id::new("pe_inv_date");
                let r = ui.add(
                    egui::TextEdit::singleline(&mut s.invoice_date)
                        .id(date_id)
                        .hint_text("DD/MM/YYYY")
                        .margin(Margin::symmetric(10, 7))
                        .desired_width(widths[2]),
                );
                if r.changed() && auto_slash(&mut s.invoice_date, &[2, 2, 4]) {
                    cursor_to_end(ui.ctx(), date_id, &s.invoice_date);
                }
                outline_error(
                    ui,
                    r.rect,
                    submitted && parse_date(&s.invoice_date).is_none(),
                    Id::new("pe_err_date"),
                );
            });

            // notes
            ui.vertical(|ui| {
                ui.set_width(widths[3]);
                label(ui, "NOTES", false);
                ui.add(
                    egui::TextEdit::singleline(&mut s.notes)
                        .hint_text("Optional remark")
                        .margin(Margin::symmetric(10, 7))
                        .desired_width(widths[3]),
                );
            });
        });

        // one reserved status line, so the layout never jumps
        ui.add_space(6.0);
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 16.0), Sense::hover());
        let (text, color) = invoice_note(s, app);
        paint_text(ui, rect, &text, 11.5, color, false);
    });
}

fn invoice_note(s: &State, app: &AsterApp) -> (String, Color32) {
    if s.suppliers.is_empty() {
        return (
            "No active suppliers found. Create or reactivate a supplier first.".into(),
            RED,
        );
    }
    let supplier = s.suppliers.iter().find(|x| x.id == s.supplier_id);
    if let Some(sup) = supplier {
        let no = s.supplier_invoice_no.trim();
        if !no.is_empty()
            && s.recent.iter().any(|r| {
                r.supplier_name == sup.name && r.supplier_invoice_no.trim().eq_ignore_ascii_case(no)
            })
        {
            return (
                format!(
                    "Invoice {no} from {} was already posted. Check before posting again.",
                    sup.name
                ),
                AMBER,
            );
        }
    }
    if let (Some((y, m, d)), Some((wy, wm, wd))) =
        (parse_date(&s.invoice_date), parse_date(&app.date_text))
    {
        if (y, m, d) > (wy, wm, wd) {
            return ("Invoice date is later than the working date.".into(), AMBER);
        }
    }
    match supplier {
        Some(sup) if !sup.gstin.trim().is_empty() => (format!("GSTIN {}", sup.gstin), MUTED),
        Some(_) => ("This supplier has no GSTIN on file.".into(), AMBER),
        None => (
            "Posting adds stock batches and tax records to this financial year in one step.".into(),
            FAINT,
        ),
    }
}

fn supplier_combo(
    ui: &mut egui::Ui,
    suppliers: &[SupplierRecord],
    value: &mut i64,
    width: f32,
) -> Rect {
    let selected = suppliers
        .iter()
        .find(|x| x.id == *value)
        .map(|x| x.name.clone())
        .unwrap_or_else(|| "Select supplier".into());
    let filter_id = Id::new("pe_supplier_filter");
    let open_id = Id::new("pe_supplier_open");
    let was_open: bool = ui.data(|d| d.get_temp(open_id)).unwrap_or(false);
    let result = egui::ComboBox::from_id_salt("purchase-supplier")
        .selected_text(selected)
        .width(width - 8.0)
        .show_ui(ui, |ui| {
            let mut filter: String = ui.data(|d| d.get_temp(filter_id)).unwrap_or_default();
            let te = ui.add(
                egui::TextEdit::singleline(&mut filter)
                    .hint_text("Type to filter…")
                    .desired_width(width - 40.0),
            );
            if !was_open {
                te.request_focus();
            }
            let needle = filter.to_lowercase();
            egui::ScrollArea::vertical()
                .max_height(240.0)
                .show(ui, |ui| {
                    for x in suppliers {
                        let text = if x.gstin.is_empty() {
                            x.name.clone()
                        } else {
                            format!("{}  ·  {}", x.name, x.gstin)
                        };
                        if !needle.is_empty() && !text.to_lowercase().contains(&needle) {
                            continue;
                        }
                        if ui.selectable_label(*value == x.id, text).clicked() {
                            *value = x.id;
                        }
                    }
                });
            ui.data_mut(|d| d.insert_temp(filter_id, filter));
        });
    let open_now = result.inner.is_some();
    ui.data_mut(|d| d.insert_temp(open_id, open_now));
    if !open_now {
        ui.data_mut(|d| d.remove::<String>(filter_id));
    }
    result.response.rect
}

// -------------------------------------------------------------- items card

#[derive(Default)]
struct LineEvent {
    remove: bool,
    enter_last: bool,
    /// Open the "create product" dialog, prefilled with this name.
    new_product: Option<String>,
    /// Re-open the dialog for the product already created on this line.
    edit_product: bool,
}

fn items_card(ui: &mut egui::Ui, s: &mut State, app: &AsterApp, w: f32, now: f64) {
    let ym = invoice_ym(s, app);
    card(ui, w, None, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("ITEMS & TAXATION")
                        .size(11.0)
                        .strong()
                        .color(PURPLE),
                );
                ui.label(
                    RichText::new(
                        "Search a medicine, or create it on the spot, then enter batch, expiry, quantity and rates.",
                    )
                    .size(12.0)
                    .color(MUTED),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if button(
                    ui,
                    "＋  Add item",
                    118.0,
                    34.0,
                    Style::Primary,
                    true,
                    "pe_add_item",
                )
                .on_hover_text("Add another line  (Ctrl+N)")
                .clicked()
                {
                    s.add_line(now);
                }
                let n = s.lines.len();
                ui.label(
                    RichText::new(format!("{n} item{}", if n == 1 { "" } else { "s" }))
                        .size(11.5)
                        .color(MUTED),
                );
            });
        });
        ui.add_space(10.0);

        let scroll_last = std::mem::take(&mut s.scroll_to_last);
        let mut remove = None;
        let mut enter_last = false;
        let mut open_new: Option<(usize, String)> = None;
        let mut edit_idx: Option<usize> = None;
        let State {
            lines,
            products,
            submitted,
            focus,
            ..
        } = s;

        // duplicate product + batch detection
        let dup: Vec<bool> = lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                l.product_id != 0
                    && !l.batch_no.trim().is_empty()
                    && lines.iter().enumerate().any(|(j, o)| {
                        j != i
                            && o.product_id == l.product_id
                            && o.batch_no.trim().eq_ignore_ascii_case(l.batch_no.trim())
                    })
            })
            .collect();
        let count = lines.len();

        ui.spacing_mut().item_spacing.y = 10.0;
        for (i, line) in lines.iter_mut().enumerate() {
            let ev = line_card(
                ui,
                line,
                i,
                products,
                *submitted,
                focus,
                dup[i],
                now,
                i + 1 == count,
                ym,
            );
            if scroll_last && i + 1 == count {
                ui.scroll_to_cursor(Some(Align::BOTTOM));
            }
            if ev.remove {
                remove = Some(i);
            }
            if ev.enter_last {
                enter_last = true;
            }
            if let Some(q) = ev.new_product {
                open_new = Some((i, q));
            }
            if ev.edit_product {
                edit_idx = Some(i);
            }
        }
        if products.is_empty() {
            ui.label(
                RichText::new(
                    "No active products found. Use “New product” on a line to create one here.",
                )
                .size(12.0)
                .color(AMBER),
            );
        }

        if let Some((i, query)) = open_new {
            if let Some(l) = s.lines.get(i) {
                s.new_product_form = Some(NewProductForm::blank(l.uid, query, now));
            }
        } else if let Some(i) = edit_idx {
            if let Some(l) = s.lines.get(i) {
                if let Some(d) = &l.new_product {
                    s.new_product_form = Some(NewProductForm::from_draft(l.uid, d, now));
                }
            }
        }

        if let Some(i) = remove {
            if s.lines.len() > 1 {
                s.lines.remove(i);
            } else {
                let line = s.new_line(now);
                s.focus = Some(prod_id(line.uid));
                s.lines[0] = line;
            }
        }
        if enter_last {
            s.add_line(now);
        }
    });
}

// ------------------------------------------------------ new product dialog

fn render_new_product_dialog(ctx: &egui::Context, s: &mut State, now: f64) {
    let Some(mut form) = s.new_product_form.take() else {
        return;
    };

    let mut open = true;
    let mut save_clicked = false;
    let mut cancel_clicked = ctx.input(|i| i.key_pressed(Key::Escape));

    let opacity = if anim(ctx) == 0.0 {
        1.0
    } else {
        ease_out(((now - form.born) / 0.18).clamp(0.0, 1.0) as f32)
    };
    if opacity < 1.0 {
        ctx.request_repaint();
    }

    // dimmed backdrop that also blocks clicks on the page behind the dialog
    let screen = ctx.content_rect();
    egui::Area::new(Id::new("pe_new_product_backdrop"))
        .order(egui::Order::Middle)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(screen.size(), Sense::click_and_drag());
            ui.painter().rect_filled(
                rect,
                0.0,
                Color32::from_black_alpha((110.0 * opacity) as u8),
            );
        });

    let can_save = !s.manufacturers.is_empty() && !s.units.is_empty();

    egui::Window::new("Create product for this purchase")
        .id(Id::new("pe_new_product_dialog"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .collapsible(false)
        .resizable(false)
        .default_width(560.0)
        .min_width(560.0)
        .max_width(560.0)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.scope(|ui| {
                ui.set_opacity(opacity);
                ui.set_min_width(520.0);
                ui.spacing_mut().item_spacing.y = 8.0;

                ui.label(
                    RichText::new("Register the medicine once, then receive its batch below.")
                        .size(12.0)
                        .color(MUTED),
                );
                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .id_salt("pe_new_product_form_scroll")
                    .max_height(430.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        egui::Grid::new("pe_new_product_form_grid")
                            .num_columns(2)
                            .spacing(Vec2::new(14.0, 12.0))
                            .show(ui, |ui| {
                                modal_text_field(
                                    ui,
                                    "PRODUCT NAME",
                                    true,
                                    &mut form.name,
                                    "e.g. Paracetamol 500",
                                );
                                modal_text_field(
                                    ui,
                                    "BRAND NAME",
                                    false,
                                    &mut form.brand_name,
                                    "Brand / trade name",
                                );
                                ui.end_row();

                                modal_text_field(
                                    ui,
                                    "GENERIC NAME",
                                    false,
                                    &mut form.generic_name,
                                    "Generic medicine name",
                                );
                                modal_text_field(
                                    ui,
                                    "COMPOSITION",
                                    false,
                                    &mut form.composition,
                                    "Composition / strength",
                                );
                                ui.end_row();

                                ui.vertical(|ui| {
                                    ui.set_width(245.0);
                                    label(ui, "MANUFACTURER", true);
                                    let selected = form
                                        .manufacturer_id
                                        .and_then(|id| s.manufacturers.iter().find(|m| m.id == id))
                                        .map(|m| m.name.as_str())
                                        .unwrap_or("Select manufacturer");
                                    egui::ComboBox::from_id_salt("pe_new_product_manufacturer")
                                        .width(225.0)
                                        .selected_text(selected)
                                        .show_ui(ui, |ui| {
                                            for m in &s.manufacturers {
                                                ui.selectable_value(
                                                    &mut form.manufacturer_id,
                                                    Some(m.id),
                                                    &m.name,
                                                );
                                            }
                                        });
                                });
                                ui.vertical(|ui| {
                                    ui.set_width(245.0);
                                    label(ui, "UNIT", true);
                                    let selected = form
                                        .unit_id
                                        .and_then(|id| s.units.iter().find(|u| u.id == id))
                                        .map(|u| u.name.as_str())
                                        .unwrap_or("Select unit");
                                    egui::ComboBox::from_id_salt("pe_new_product_unit")
                                        .width(225.0)
                                        .selected_text(selected)
                                        .show_ui(ui, |ui| {
                                            for unit in &s.units {
                                                ui.selectable_value(
                                                    &mut form.unit_id,
                                                    Some(unit.id),
                                                    &unit.name,
                                                );
                                            }
                                        });
                                });
                                ui.end_row();

                                modal_text_field(
                                    ui,
                                    "HSN CODE",
                                    true,
                                    &mut form.hsn_code,
                                    "4, 6 or 8 digits",
                                );
                                modal_text_field(
                                    ui,
                                    "GST %",
                                    true,
                                    &mut form.gst_percent,
                                    "e.g. 5",
                                );
                                ui.end_row();

                                modal_text_field(
                                    ui,
                                    "SCHEDULE",
                                    false,
                                    &mut form.schedule,
                                    "e.g. H / H1 / OTC",
                                );
                                modal_text_field(
                                    ui,
                                    "BARCODE",
                                    false,
                                    &mut form.barcode,
                                    "Optional barcode",
                                );
                                ui.end_row();

                                modal_text_field(
                                    ui,
                                    "REORDER LEVEL",
                                    false,
                                    &mut form.reorder_level,
                                    "Minimum stock alert",
                                );
                                ui.vertical(|ui| {
                                    ui.set_width(245.0);
                                    label(ui, "STOCK HANDLING", false);
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new(
                                            "Created and stocked only when this purchase is posted.",
                                        )
                                        .size(11.0)
                                        .color(GREEN),
                                    );
                                });
                                ui.end_row();
                            });
                    });

                if !can_save {
                    ui.label(
                        RichText::new(
                            "A manufacturer and unit are required. Add the missing master data first, then reopen this form.",
                        )
                        .size(11.5)
                        .color(AMBER),
                    );
                }
                if !form.error.is_empty() {
                    ui.label(RichText::new(&form.error).size(11.5).color(RED));
                }

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if button(
                        ui,
                        "Add product to purchase",
                        210.0,
                        38.0,
                        Style::Primary,
                        can_save,
                        "pe_new_product_save",
                    )
                    .clicked()
                    {
                        save_clicked = true;
                    }
                    if button(
                        ui,
                        "Cancel",
                        104.0,
                        38.0,
                        Style::Ghost,
                        true,
                        "pe_new_product_cancel",
                    )
                    .clicked()
                    {
                        cancel_clicked = true;
                    }
                });
            });
        });

    if save_clicked {
        match form.validate(&s.manufacturers, &s.units) {
            Ok(draft) => {
                let same = |a: &str, b: &str| a.trim().eq_ignore_ascii_case(b.trim());
                let duplicate_existing = s.products.iter().any(|p| {
                    same(&p.name, &draft.name)
                        && same(&p.brand_name, &draft.brand_name)
                        && same(&p.composition, &draft.composition)
                });
                let duplicate_pending = s.lines.iter().any(|l| {
                    l.uid != form.target_uid
                        && l.new_product.as_ref().map_or(false, |n| {
                            same(&n.name, &draft.name)
                                && same(&n.brand_name, &draft.brand_name)
                                && same(&n.composition, &draft.composition)
                        })
                });

                if duplicate_existing {
                    form.error =
                        "This product already exists. Cancel and select it from the product search."
                            .into();
                    s.new_product_form = Some(form);
                    return;
                }
                if duplicate_pending {
                    form.error =
                        "This product is already being created on another line. Select it from the search there."
                            .into();
                    s.new_product_form = Some(form);
                    return;
                }

                if let Some(line) = s.lines.iter_mut().find(|line| line.uid == form.target_uid) {
                    // Negative ID = temporary product, not a database ID.
                    line.product_id = -(line.uid as i64);
                    line.product_query = draft.name.clone();
                    line.new_product = Some(draft);
                    s.focus = Some(field_id(line.uid, 0));
                    s.message =
                        "Product added to this purchase draft. It will be saved with the stock when you post."
                            .into();
                    s.error = false;
                    s.confirm_clear = false;
                }
            }
            Err(error) => {
                form.error = error;
                s.new_product_form = Some(form);
            }
        }
    } else if !cancel_clicked && open {
        s.new_product_form = Some(form);
    }
}

fn modal_text_field(
    ui: &mut egui::Ui,
    title: &str,
    required: bool,
    value: &mut String,
    hint: &str,
) {
    ui.vertical(|ui| {
        ui.set_width(245.0);
        label(ui, title, required);
        ui.add_sized(
            Vec2::new(235.0, 32.0),
            egui::TextEdit::singleline(value)
                .hint_text(hint)
                .margin(Margin::symmetric(9, 6)),
        );
    });
}

#[allow(clippy::too_many_arguments)]
fn line_card(
    ui: &mut egui::Ui,
    line: &mut LineForm,
    index: usize,
    products: &[ProductRecord],
    submitted: bool,
    focus: &mut Option<Id>,
    duplicate: bool,
    now: f64,
    is_last: bool,
    invoice_ym: (i32, u32),
) -> LineEvent {
    let ctx = ui.ctx().clone();
    let mut ev = LineEvent::default();
    let t = if anim(&ctx) == 0.0 {
        1.0
    } else {
        (((now - line.born) / 0.26).clamp(0.0, 1.0)) as f32
    };
    if t < 1.0 {
        ctx.request_repaint();
    }
    let errors = line_errors(line, products, invoice_ym);
    let show_err = submitted && errors.any();

    ui.scope(|ui| {
        ui.set_opacity(ease_out(t));
        let frame = egui::Frame::new()
            .fill(CANVAS)
            .stroke(Stroke::new(
                1.0_f32,
                if show_err {
                    RED.gamma_multiply(0.55)
                } else {
                    BORDER
                },
            ))
            .corner_radius(11.0)
            .inner_margin(Margin::same(12));
        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            let inner_right = ui.max_rect().right();
            let top = ui.cursor().top();

            // ---- row A: badge + product search + remove
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let (badge, _) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::hover());
                ui.painter().rect_filled(badge, 7.0, TINT);
                ui.painter().text(
                    badge.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{:02}", index + 1),
                    FontId::proportional(11.0),
                    PURPLE,
                );
                let mid_w = (inner_right - ui.cursor().left() - 38.0).max(200.0);
                ui.vertical(|ui| {
                    ui.set_width(mid_w);
                    product_search(
                        ui,
                        line,
                        products,
                        mid_w,
                        submitted && errors.product,
                        focus,
                        &mut ev.new_product,
                    );
                    product_meta(ui, line, products, submitted && errors.product, &mut ev);
                });
            });

            // remove button, drawn at a fixed spot so it never moves the layout
            let rm = Rect::from_min_size(Pos2::new(inner_right - 28.0, top), Vec2::splat(28.0));
            let rr = ui.interact(rm, Id::new(("pe_rm", line.uid)), Sense::click());
            let hot = ctx.animate_bool_with_time(rr.id, rr.hovered(), anim(&ctx));
            ui.painter().rect_filled(
                rm,
                8.0,
                mix(Color32::TRANSPARENT, Color32::from_rgb(250, 232, 232), hot),
            );
            ui.painter().text(
                rm.center(),
                egui::Align2::CENTER_CENTER,
                "✕",
                FontId::proportional(13.0),
                mix(FAINT, RED, hot),
            );
            if rr.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if rr.on_hover_text("Remove this line").clicked() {
                ev.remove = true;
            }

            ui.add_space(8.0);

            // ---- row B: numeric fields on a fixed grid
            let avail = ui.available_width().max(320.0);
            let per_row: usize = if avail >= 640.0 { 8 } else { 4 };
            let weights = [1.5_f32, 1.05, 0.9, 0.8, 1.1, 0.9, 1.1, 1.1];
            let gap = 8.0;
            for chunk in 0..(8 / per_row) {
                let ws = &weights[chunk * per_row..(chunk + 1) * per_row];
                let unit = (avail - gap * (per_row as f32 - 1.0)) / ws.iter().sum::<f32>();
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (k, weight) in ws.iter().enumerate() {
                        let idx = chunk * per_row + k;
                        let width = weight * unit;
                        let id = field_id(line.uid, idx);
                        let (title, hint, numeric, bad) = match idx {
                            0 => ("BATCH NO. *", "Batch", false, errors.batch),
                            1 => ("EXPIRY *", "MM/YY", false, errors.expiry),
                            2 => ("QTY *", "0", true, errors.qty),
                            3 => ("FREE QTY", "0", true, errors.free),
                            4 => ("RATE (₹) *", "0.00", true, errors.rate),
                            5 => ("DISC. %", "0", true, errors.disc),
                            6 => ("MRP (₹)", "0.00", true, errors.mrp),
                            _ => ("SALE RATE (₹)", "0.00", true, errors.sale),
                        };
                        let value = match idx {
                            0 => &mut line.batch_no,
                            1 => &mut line.expiry,
                            2 => &mut line.qty,
                            3 => &mut line.free_qty,
                            4 => &mut line.rate,
                            5 => &mut line.discount,
                            6 => &mut line.mrp,
                            _ => &mut line.sale_rate,
                        };
                        let r = line_field(
                            ui,
                            id,
                            title,
                            value,
                            width,
                            hint,
                            numeric,
                            submitted && bad,
                        );
                        if idx == 1 && r.changed() && auto_slash(&mut line.expiry, &[2, 4]) {
                            cursor_to_end(ui.ctx(), id, &line.expiry);
                        }
                        if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            if idx == 7 {
                                if is_last {
                                    ev.enter_last = true;
                                }
                            } else {
                                *focus = Some(field_id(line.uid, idx + 1));
                            }
                        }
                    }
                });
                if chunk + 1 < 8 / per_row {
                    ui.add_space(6.0);
                }
            }

            ui.add_space(10.0);
            line_summary(ui, line, products, duplicate, invoice_ym);

            if show_err {
                ui.add_space(6.0);
                if let Some(message) = &errors.first {
                    ui.label(
                        RichText::new(format!("Line {}: {message}", index + 1))
                            .size(11.5)
                            .color(RED),
                    );
                }
            }
        });
    });
    ev
}

#[allow(clippy::too_many_arguments)]
fn line_field(
    ui: &mut egui::Ui,
    id: Id,
    title: &str,
    value: &mut String,
    width: f32,
    hint: &str,
    numeric: bool,
    error: bool,
) -> egui::Response {
    ui.vertical(|ui| {
        ui.set_width(width);
        ui.label(RichText::new(title).size(9.5).strong().color(MUTED));
        let before = value.clone();
        let response = ui.add(
            egui::TextEdit::singleline(value)
                .id(id)
                .hint_text(hint)
                .margin(Margin::symmetric(8, 6))
                .desired_width(width),
        );
        if numeric && *value != before {
            value.retain(|c| c.is_ascii_digit() || c == '.' || c == ',');
        }
        if numeric && response.gained_focus() {
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
                let end = egui::text::CCursor::new(value.chars().count());
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        end,
                    )));
                egui::TextEdit::store_state(ui.ctx(), id, state);
            }
        }
        outline_error(ui, response.rect, error, id.with("err"));
        response
    })
    .inner
}

fn product_search(
    ui: &mut egui::Ui,
    line: &mut LineForm,
    products: &[ProductRecord],
    width: f32,
    error: bool,
    focus: &mut Option<Id>,
    create: &mut Option<String>,
) {
    let ctx = ui.ctx().clone();
    let pid = prod_id(line.uid);
    let open_id = Id::new(("pe_pop_open", line.uid));
    let rect_id = Id::new(("pe_pop_rect", line.uid));
    let hl_id = Id::new(("pe_pop_hl", line.uid));

    ui.label(
        RichText::new("PRODUCT / MEDICINE *")
            .size(9.5)
            .strong()
            .color(MUTED),
    );

    if *focus == Some(pid) {
        ctx.memory_mut(|m| m.request_focus(pid));
        *focus = None;
    }

    let was_open: bool = ctx.data(|d| d.get_temp(open_id)).unwrap_or(false);
    let mut highlight: usize = ctx.data(|d| d.get_temp(hl_id)).unwrap_or(0);
    let had_focus = ctx.memory(|m| m.has_focus(pid));
    let (mut down, mut up, mut enter) = (false, false, false);
    if was_open && had_focus {
        ctx.input_mut(|i| {
            down = i.consume_key(Modifiers::NONE, Key::ArrowDown);
            up = i.consume_key(Modifiers::NONE, Key::ArrowUp);
            enter = i.consume_key(Modifiers::NONE, Key::Enter);
        });
    }

    let response = ui.add(
        egui::TextEdit::singleline(&mut line.product_query)
            .id(pid)
            .hint_text("Type medicine name, brand or generic…")
            .margin(Margin::symmetric(10, 7))
            .desired_width(width),
    );
    outline_error(ui, response.rect, error, pid.with("err"));

    if response.changed() {
        // Editing the text of a chosen product un-chooses it.
        let chosen = resolve(line, products).map(|r| r.name.trim().to_lowercase());
        if let Some(name) = chosen {
            if line.product_query.trim().to_lowercase() != name {
                line.product_id = 0;
                line.new_product = None;
            }
        }
        highlight = 0;
    }

    // ---- results
    let raw_query = line.product_query.trim().to_owned();
    let query = raw_query.to_lowercase();
    let last_rect: Option<Rect> = ctx.data(|d| d.get_temp(rect_id));
    let pointer_inside = last_rect
        .zip(ctx.input(|i| i.pointer.hover_pos()))
        .map_or(false, |(r, p)| r.expand(4.0).contains(p));
    let show = line.product_id == 0 && (response.has_focus() || (was_open && pointer_inside));

    let mut matches: Vec<&ProductRecord> = products
        .iter()
        .filter(|p| {
            query.is_empty()
                || format!(
                    "{} {} {} {} {}",
                    p.name, p.brand_name, p.generic_name, p.manufacturer_name, p.barcode
                )
                .to_lowercase()
                .contains(&query)
        })
        .collect();
    matches.sort_by_key(|p| {
        let name = p.name.to_lowercase();
        if name == query {
            0
        } else if name.starts_with(&query) {
            1
        } else if p.brand_name.to_lowercase().starts_with(&query)
            || p.generic_name.to_lowercase().starts_with(&query)
        {
            2
        } else {
            3
        }
    });
    matches.truncate(7);

    // index == matches.len() is the "create new product" row
    let create_index = matches.len();
    if down {
        highlight += 1;
    }
    if up {
        highlight = highlight.saturating_sub(1);
    }
    highlight = highlight.min(create_index);

    let mut picked: Option<&ProductRecord> = None;
    let mut create_requested = false;
    if show {
        let area = egui::Area::new(Id::new(("pe_pop", line.uid)))
            .order(egui::Order::Foreground)
            .fixed_pos(response.rect.left_bottom() + Vec2::new(0.0, 4.0))
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(Color32::WHITE)
                    .inner_margin(Margin::same(4))
                    .show(ui, |ui| {
                        let w = (width - 8.0).max(260.0);
                        ui.set_width(w);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        if matches.is_empty() {
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("No matching active products.")
                                    .size(12.0)
                                    .color(AMBER),
                            );
                            ui.add_space(4.0);
                        }
                        for (n, p) in matches.iter().enumerate() {
                            let (r, rr) =
                                ui.allocate_exact_size(Vec2::new(w, 42.0), Sense::click());
                            let hot = rr.hovered() || n == highlight;
                            if hot {
                                ui.painter().rect_filled(r, 7.0, TINT);
                            }
                            paint_text(
                                ui,
                                Rect::from_min_size(
                                    r.min + Vec2::new(10.0, 4.0),
                                    Vec2::new(w - 20.0, 17.0),
                                ),
                                &p.name,
                                13.0,
                                INK,
                                true,
                            );
                            let meta = [
                                p.brand_name.trim(),
                                p.generic_name.trim(),
                                p.manufacturer_name.trim(),
                            ]
                            .into_iter()
                            .filter(|x| !x.is_empty())
                            .collect::<Vec<_>>()
                            .join("  ·  ");
                            let meta = if meta.is_empty() {
                                format!("GST {:.0}%", p.gst_percent)
                            } else {
                                format!("{meta}  ·  GST {:.0}%", p.gst_percent)
                            };
                            paint_text(
                                ui,
                                Rect::from_min_size(
                                    r.min + Vec2::new(10.0, 22.0),
                                    Vec2::new(w - 20.0, 15.0),
                                ),
                                &meta,
                                11.0,
                                MUTED,
                                false,
                            );
                            if rr.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                highlight = n;
                            }
                            // select on press so the choice lands before the text box loses focus
                            if rr.hovered() && ui.input(|i| i.pointer.primary_pressed())
                                || rr.clicked()
                            {
                                picked = Some(*p);
                            }
                        }

                        // ---- create-new row, always last
                        ui.add_space(2.0);
                        let (sep, _) = ui.allocate_exact_size(Vec2::new(w, 1.0), Sense::hover());
                        ui.painter().rect_filled(sep, 0.0, BORDER);
                        let (r, rr) = ui.allocate_exact_size(Vec2::new(w, 38.0), Sense::click());
                        let hot = rr.hovered() || highlight == create_index;
                        if hot {
                            ui.painter().rect_filled(r, 7.0, TINT);
                        }
                        let text = if raw_query.is_empty() {
                            "＋  Create a new product".to_owned()
                        } else {
                            format!("＋  Create “{raw_query}” as a new product")
                        };
                        paint_text(
                            ui,
                            Rect::from_min_size(
                                r.min + Vec2::new(10.0, 0.0),
                                Vec2::new(w - 20.0, 38.0),
                            ),
                            &text,
                            12.5,
                            PURPLE,
                            true,
                        );
                        if rr.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            highlight = create_index;
                        }
                        if rr.hovered() && ui.input(|i| i.pointer.primary_pressed()) || rr.clicked()
                        {
                            create_requested = true;
                        }
                    });
            });
        ctx.data_mut(|d| d.insert_temp(rect_id, area.response.rect));
    } else {
        ctx.data_mut(|d| d.remove::<Rect>(rect_id));
    }

    if enter && show {
        if highlight < matches.len() {
            picked = matches.get(highlight).copied();
        } else if !raw_query.is_empty() {
            create_requested = true;
        }
    }
    if let Some(p) = picked {
        line.product_id = p.id;
        line.new_product = None;
        line.product_query = p.name.clone();
        *focus = Some(field_id(line.uid, 0));
    } else if create_requested {
        *create = Some(raw_query);
    }
    ctx.data_mut(|d| {
        d.insert_temp(open_id, show && picked.is_none() && !create_requested);
        d.insert_temp(hl_id, highlight);
    });
}

fn product_meta(
    ui: &mut egui::Ui,
    line: &LineForm,
    products: &[ProductRecord],
    error: bool,
    ev: &mut LineEvent,
) {
    ui.add_space(3.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 16.0), Sense::hover());
    let link_w = 116.0;
    let text_rect =
        Rect::from_min_max(rect.min, Pos2::new(rect.right() - link_w - 8.0, rect.max.y));
    let link_rect = Rect::from_min_max(Pos2::new(rect.right() - link_w, rect.min.y), rect.max);
    let link_id = Id::new(("pe_meta_link", line.uid));

    match resolve(line, products) {
        Some(r) if r.is_new => {
            let hsn = if r.hsn.trim().is_empty() {
                "HSN not set".to_owned()
            } else {
                format!("HSN {}", r.hsn)
            };
            paint_text(
                ui,
                text_rect,
                &format!(
                    "NEW PRODUCT   ·   {hsn}   ·   GST {:.2}%   ·   saved when you post",
                    r.gst_percent
                ),
                11.0,
                GREEN,
                false,
            );
            if link(ui, link_rect, "Edit details", link_id) {
                ev.edit_product = true;
            }
        }
        Some(r) => {
            let mut parts = vec![if r.hsn.trim().is_empty() {
                "HSN not set".to_owned()
            } else {
                format!("HSN {}", r.hsn)
            }];
            parts.push(format!("GST {:.2}%", r.gst_percent));
            if !r.maker.trim().is_empty() {
                parts.push(r.maker.to_owned());
            }
            paint_text(ui, text_rect, &parts.join("   ·   "), 11.0, MUTED, false);
        }
        None => {
            let (text, color) = if error {
                ("Choose a medicine from the suggestions, or create it.", RED)
            } else {
                ("No medicine selected yet.", FAINT)
            };
            paint_text(ui, text_rect, text, 11.0, color, false);
            if link(ui, link_rect, "＋ New product", link_id) {
                ev.new_product = Some(line.product_query.trim().to_owned());
            }
        }
    }
}

fn line_summary(
    ui: &mut egui::Ui,
    line: &LineForm,
    products: &[ProductRecord],
    duplicate: bool,
    invoice_ym: (i32, u32),
) {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(8.0)
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let calc = line_calc(line, products);
            let gst = resolve(line, products)
                .map(|r| r.gst_percent)
                .unwrap_or(0.0);
            let c = calc.unwrap_or_default();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(18.0, 4.0);
                kv(ui, "Taxable", &money(c.taxable), INK);
                kv(ui, &format!("GST {gst:.0}%"), &money(c.tax), PURPLE);
                kv(ui, "Line total", &money(c.taxable + c.tax), PURPLE_DEEP);
                if c.net_unit > 0.0 {
                    kv(
                        ui,
                        "Net cost / unit",
                        &money(c.net_unit.round() as i64),
                        INK,
                    );
                }
                if let Some(m) = c.margin {
                    kv(
                        ui,
                        "Margin",
                        &format!("{m:.1}%"),
                        if m < 0.0 { RED } else { GREEN },
                    );
                }
            });

            let mut notes: Vec<(String, Color32)> = Vec::new();
            if let Some(ym) = parse_expiry(&line.expiry) {
                let months = months_between(invoice_ym, ym);
                let text = format!(
                    "Expires {} {} · {} month{} of shelf life",
                    month_abbr(ym.1),
                    ym.0,
                    months.max(0),
                    if months == 1 { "" } else { "s" }
                );
                if months < 0 {
                    notes.push((format!("Expired in {} {}", month_abbr(ym.1), ym.0), RED));
                } else if months < 6 {
                    notes.push((format!("Short expiry: {text}"), AMBER));
                } else {
                    notes.push((text, MUTED));
                }
            }
            if duplicate {
                notes.push((
                    "Same product and batch appears on another line.".into(),
                    AMBER,
                ));
            }
            let rate = parse_money(&line.rate).unwrap_or(0);
            let mrp = parse_money(&line.mrp).unwrap_or(0);
            let sale = parse_money(&line.sale_rate).unwrap_or(0);
            if mrp > 0 && rate > mrp {
                notes.push(("Purchase rate is higher than MRP.".into(), AMBER));
            }
            if sale > 0 && c.net_unit > 0.0 && (sale as f64) < c.net_unit {
                notes.push(("Sale rate is below cost.".into(), AMBER));
            }
            if !notes.is_empty() {
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(16.0, 2.0);
                    for (text, color) in notes {
                        ui.label(RichText::new(text).size(11.5).color(color));
                    }
                });
            }
        });
}

fn kv(ui: &mut egui::Ui, key: &str, value: &str, color: Color32) {
    ui.label(RichText::new(key).size(11.5).color(MUTED));
    ui.label(RichText::new(value).size(13.0).strong().color(color));
}

// ------------------------------------------------------------------ footer

fn footer_card(ui: &mut egui::Ui, s: &mut State, app: &mut AsterApp, w: f32, now: f64) {
    let ctx = ui.ctx().clone();
    let t = totals(s);
    let shown_taxable = tween_money(&ctx, "taxable", t.taxable, now);
    let shown_tax = tween_money(&ctx, "tax", t.tax, now);
    let shown_total = tween_money(&ctx, "total", t.taxable + t.tax, now);

    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(12.0)
        .inner_margin(Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_width(w - 34.0);
            ui.set_height(FOOTER_H - 26.0);
            ui.spacing_mut().item_spacing.y = 6.0;

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 26.0;
                stat(ui, "ITEMS", &t.lines.to_string());
                stat(ui, "TOTAL QTY", &qty_text(t.qty_milli));
                stat(ui, "TAXABLE VALUE", &money(shown_taxable));
                stat(ui, "TOTAL GST", &money(shown_tax));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("INVOICE TOTAL (ESTIMATE)")
                                .size(10.0)
                                .strong()
                                .color(MUTED),
                        );
                        ui.label(
                            RichText::new(money(shown_total))
                                .size(24.0)
                                .strong()
                                .color(PURPLE),
                        );
                    });
                });
            });

            ui.horizontal(|ui| {
                // status / message, fading in when it changes
                let msg_t = ctx.animate_bool_with_time(
                    Id::new("pe_msg"),
                    !s.message.is_empty(),
                    anim(&ctx),
                );
                let width = (ui.available_width() - 360.0).max(120.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 38.0), Sense::hover());
                if !s.message.is_empty() {
                    let color = if s.error { RED } else { GREEN };
                    paint_text(
                        ui,
                        rect,
                        &s.message,
                        12.0,
                        color.gamma_multiply(msg_t),
                        false,
                    );
                } else {
                    paint_text(
                        ui,
                        rect,
                        "Same state: CGST + SGST  ·  Other states: IGST. Tax is split automatically when posted.",
                        11.0,
                        FAINT,
                        false,
                    );
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if button(
                        ui,
                        "Post purchase & update stock",
                        232.0,
                        38.0,
                        Style::Primary,
                        !s.suppliers.is_empty(),
                        "pe_post",
                    )
                    .on_hover_text("Post and update stock  (Ctrl+Enter)")
                    .clicked()
                    {
                        post(s, app, now);
                    }
                    let (label, style) = if s.confirm_clear {
                        ("Click again to clear", Style::Danger)
                    } else {
                        ("Clear all", Style::Ghost)
                    };
                    if button(ui, label, 136.0, 38.0, style, true, "pe_clear").clicked() {
                        if s.confirm_clear
                            || (s.lines.iter().all(LineForm::is_blank)
                                && s.supplier_invoice_no.is_empty())
                        {
                            s.reset(&app.date_text, now);
                            s.message.clear();
                        } else {
                            s.confirm_clear = true;
                        }
                    }
                });
            });
        });
}

fn stat(ui: &mut egui::Ui, title: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(RichText::new(title).size(10.0).strong().color(MUTED));
        ui.label(RichText::new(value).size(15.0).strong().color(INK));
    });
}

// ------------------------------------------------------------ recent panel

fn recent_panel(ui: &mut egui::Ui, s: &State, w: f32, h: f32) {
    ui.allocate_ui_with_layout(Vec2::new(w, h), Layout::top_down(Align::Min), |ui| {
        ui.set_min_size(Vec2::new(w, h));
        card(ui, w, Some(h), |ui| {
            let sum: i64 = s.recent.iter().map(|r| r.total_paise).sum();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("RECENT PURCHASES")
                            .size(11.0)
                            .strong()
                            .color(PURPLE),
                    );
                    ui.label(
                        RichText::new("Latest invoices in this financial year")
                            .size(12.0)
                            .color(MUTED),
                    );
                });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 20.0;
                stat(ui, "LISTED", &s.recent.len().to_string());
                stat(ui, "VALUE", &money(sum));
            });
            ui.add_space(8.0);
            ui.separator();

            if s.recent.is_empty() {
                let rect = ui.available_rect_before_wrap();
                ui.painter().text(
                    rect.center() - Vec2::new(0.0, 8.0),
                    egui::Align2::CENTER_CENTER,
                    "No posted purchases yet",
                    FontId::proportional(14.0),
                    INK,
                );
                ui.painter().text(
                    rect.center() + Vec2::new(0.0, 12.0),
                    egui::Align2::CENTER_CENTER,
                    "Posted invoices will appear here.",
                    FontId::proportional(12.0),
                    MUTED,
                );
                return;
            }

            ui.scope(|ui| {
                // must be set before show_rows reads the spacing
                ui.spacing_mut().item_spacing.y = 0.0;
                egui::ScrollArea::vertical()
                    .id_salt("pe_recent_scroll")
                    .auto_shrink([false, false])
                    .show_rows(ui, RECENT_ROW_H, s.recent.len(), |ui, range| {
                        for r in &s.recent[range] {
                            recent_row(ui, r);
                        }
                    });
            });
        });
    });
}

fn recent_row(ui: &mut egui::Ui, r: &PurchaseSummary) {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), RECENT_ROW_H),
        Sense::hover(),
    );
    let ctx = ui.ctx().clone();
    let hot = ctx.animate_bool_with_time(response.id, response.hovered(), anim(&ctx));
    if hot > 0.0 {
        ui.painter().rect_filled(
            rect.shrink2(Vec2::new(-4.0, 3.0)),
            8.0,
            TINT.gamma_multiply(hot),
        );
    }
    ui.painter().line_segment(
        [
            Pos2::new(rect.left(), rect.bottom() - 0.5),
            Pos2::new(rect.right(), rect.bottom() - 0.5),
        ],
        Stroke::new(1.0_f32, BORDER.gamma_multiply(1.0 - hot)),
    );
    let total = money(r.total_paise);
    let total_w = 104.0;
    let left_w = rect.width() - total_w - 8.0;
    let line = |y: f32, h: f32| {
        Rect::from_min_size(Pos2::new(rect.left(), rect.top() + y), Vec2::new(left_w, h))
    };
    paint_text(ui, line(8.0, 17.0), &r.purchase_no, 13.0, INK, true);
    paint_text(ui, line(26.0, 16.0), &r.supplier_name, 12.0, MUTED, false);
    paint_text(
        ui,
        line(43.0, 15.0),
        &format!("{}  ·  {}", r.invoice_date, r.supplier_invoice_no),
        11.0,
        FAINT,
        false,
    );
    paint_text(
        ui,
        Rect::from_min_size(
            Pos2::new(rect.right() - total_w, rect.top() + 8.0),
            Vec2::new(total_w, 17.0),
        ),
        &total,
        13.0,
        PURPLE_DEEP,
        true,
    );
    let posted = r.status.eq_ignore_ascii_case("posted");
    let (fg, bg) = if posted {
        (GREEN, Color32::from_rgb(232, 245, 238))
    } else {
        (AMBER, Color32::from_rgb(253, 242, 222))
    };
    let galley =
        ui.painter()
            .layout_no_wrap(r.status.to_uppercase(), FontId::proportional(9.5), fg);
    let pill = Rect::from_min_size(
        Pos2::new(rect.right() - galley.size().x - 14.0, rect.top() + 32.0),
        Vec2::new(galley.size().x + 14.0, 17.0),
    );
    ui.painter().rect_filled(pill, 8.5, bg);
    ui.painter().galley(
        Pos2::new(pill.left() + 7.0, pill.center().y - galley.size().y / 2.0),
        galley,
        fg,
    );
    response.on_hover_text(format!("Purchase record ID: {}", r.id));
}

// ----------------------------------------------------------------- widgets

#[derive(Clone, Copy, PartialEq)]
enum Style {
    Primary,
    Ghost,
    Danger,
}

fn button(
    ui: &mut egui::Ui,
    text: &str,
    width: f32,
    height: f32,
    style: Style,
    enabled: bool,
    salt: &str,
) -> egui::Response {
    let ctx = ui.ctx().clone();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let response = ui.interact(
        rect,
        Id::new(("pe_btn", salt)),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let t = ctx.animate_bool_with_time(response.id, enabled && response.hovered(), anim(&ctx));
    let r = if enabled && response.is_pointer_button_down_on() {
        rect.shrink(0.75)
    } else {
        rect
    };
    let (fill, stroke, fg) = match style {
        Style::Primary => (
            if enabled {
                mix(PURPLE, PURPLE_DEEP, t)
            } else {
                Color32::from_rgb(208, 201, 222)
            },
            Color32::TRANSPARENT,
            Color32::WHITE,
        ),
        Style::Ghost => (
            mix(Color32::WHITE, TINT, t),
            mix(BORDER_STRONG, PURPLE, t * 0.6),
            if enabled { mix(INK, PURPLE, t) } else { FAINT },
        ),
        Style::Danger => (
            mix(
                Color32::from_rgb(252, 240, 240),
                Color32::from_rgb(248, 226, 226),
                t,
            ),
            RED.gamma_multiply(0.5),
            RED,
        ),
    };
    ui.painter().rect_filled(r, 9.0, fill);
    if stroke != Color32::TRANSPARENT {
        ui.painter().rect_stroke(
            r,
            9.0,
            Stroke::new(1.0_f32, stroke),
            egui::StrokeKind::Inside,
        );
    }
    ui.painter().text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        text,
        FontId::proportional(13.0),
        fg,
    );
    if enabled && response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn segmented<T: Copy + PartialEq>(ui: &mut egui::Ui, value: &mut T, options: &[(T, &str)]) {
    let seg_w = 104.0;
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(seg_w * options.len() as f32 + 6.0, 34.0),
        Sense::hover(),
    );
    ui.painter()
        .rect_filled(rect, 10.0, Color32::from_rgb(240, 238, 245));
    let ctx = ui.ctx().clone();
    let index = options.iter().position(|o| o.0 == *value).unwrap_or(0) as f32;
    let x = ctx.animate_value_with_time(Id::new("pe_seg"), index, anim(&ctx) * 1.6);
    let highlight = Rect::from_min_size(
        Pos2::new(rect.left() + 3.0 + x * seg_w, rect.top() + 3.0),
        Vec2::new(seg_w, 28.0),
    );
    ui.painter().rect_filled(highlight, 8.0, Color32::WHITE);
    ui.painter().rect_stroke(
        highlight,
        8.0,
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );
    for (i, (option, text)) in options.iter().enumerate() {
        let seg = Rect::from_min_size(
            Pos2::new(rect.left() + 3.0 + i as f32 * seg_w, rect.top() + 3.0),
            Vec2::new(seg_w, 28.0),
        );
        let response = ui.interact(seg, Id::new(("pe_seg_item", i)), Sense::click());
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

// --------------------------------------------------------------------- post

fn post(s: &mut State, app: &mut AsterApp, now: f64) {
    s.submitted = true;
    s.confirm_clear = false;
    let ym = invoice_ym(s, app);

    if s.supplier_id == 0 {
        s.message = "Select a supplier.".into();
        s.error = true;
        return;
    }
    if s.supplier_invoice_no.trim().is_empty() {
        s.message = "Enter the supplier invoice number.".into();
        s.error = true;
        return;
    }
    let Some((y, m, d)) = parse_date(&s.invoice_date) else {
        s.message = "Enter a valid invoice date as DD/MM/YYYY.".into();
        s.error = true;
        return;
    };

    let mut lines = Vec::new();
    for (i, l) in s.lines.iter().enumerate() {
        let e = line_errors(l, &s.products, ym);
        if let Some(message) = e.first {
            s.message = format!("Line {}: {message}", i + 1);
            s.error = true;
            return;
        }

        // A product created inline may have been added to Product Master
        // meanwhile (e.g. after a refresh); never create it twice.
        if let Some(n) = &l.new_product {
            let exists = s.products.iter().any(|p| {
                p.name.trim().eq_ignore_ascii_case(n.name.trim())
                    && p.brand_name
                        .trim()
                        .eq_ignore_ascii_case(n.brand_name.trim())
                    && p.composition
                        .trim()
                        .eq_ignore_ascii_case(n.composition.trim())
            });
            if exists {
                s.message = format!(
                    "Line {}: “{}” already exists in Product Master. Select it from the search instead.",
                    i + 1,
                    n.name
                );
                s.error = true;
                return;
            }
        }

        let built = (|| -> Result<PurchaseLineDraft, String> {
            Ok(PurchaseLineDraft {
                // The new product is created together with the purchase; the
                // negative placeholder id must never reach the database.
                new_product: l.new_product.clone(),
                product_id: if l.new_product.is_some() {
                    0
                } else {
                    l.product_id
                },
                batch_no: l.batch_no.trim().into(),
                expiry_date: expiry_iso(parse_expiry(&l.expiry).ok_or("Invalid expiry.")?),
                quantity_milli: parse_qty(&l.qty, "Quantity", false)?,
                free_quantity_milli: parse_qty(&l.free_qty, "Free quantity", true)?,
                rate_paise: parse_money(&l.rate)?,
                discount_basis_points: parse_discount(&l.discount)?,
                mrp_paise: parse_money(&l.mrp)?,
                sale_rate_paise: parse_money(&l.sale_rate)?,
            })
        })();
        match built {
            Ok(line) => lines.push(line),
            Err(message) => {
                s.message = format!("Line {}: {message}", i + 1);
                s.error = true;
                return;
            }
        }
    }

    let draft = PurchaseDraft {
        supplier_id: s.supplier_id,
        supplier_invoice_no: s.supplier_invoice_no.trim().into(),
        invoice_date: format!("{y:04}-{m:02}-{d:02}"),
        notes: s.notes.trim().into(),
        lines,
    };

    let result = app
        .db
        .as_ref()
        .ok_or_else(|| "Database is not open.".to_owned())
        .and_then(|db| db.post_purchase(app.selected_year, &draft));

    match result {
        Ok(no) => {
            s.reset(&app.date_text, now);
            reload(s, app);
            s.message =
                format!("Purchase {no} posted. Batch stock and stock movements were updated.");
            s.error = false;
            app.profile_message = format!("Purchase {no} posted successfully.");
        }
        Err(e) => {
            s.message = e;
            s.error = true;
        }
    }
}

fn reload(s: &mut State, app: &AsterApp) {
    if let Some(db) = app.db.as_ref() {
        s.suppliers = db
            .suppliers(app.selected_year, "")
            .unwrap_or_default()
            .into_iter()
            .filter(|x| x.active)
            .collect();
        s.products = db
            .products(app.selected_year, "")
            .unwrap_or_default()
            .into_iter()
            .filter(|x| x.active)
            .collect();
        s.recent = db
            .recent_purchases(app.selected_year, 40)
            .unwrap_or_default();
        s.manufacturers = db.manufacturers(app.selected_year).unwrap_or_default();
        s.units = db.units(app.selected_year).unwrap_or_default();
    } else {
        s.suppliers.clear();
        s.products.clear();
        s.recent.clear();
        s.manufacturers.clear();
        s.units.clear();
    }
}
