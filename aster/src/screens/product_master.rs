use eframe::egui::{
    self, Align, Color32, FontId, Id, Key, Margin, Pos2, Rect, RichText, Sense, Stroke, Vec2,
};

use crate::{
    app::AsterApp,
    database::{ManufacturerOption, ProductRecord, UnitOption},
};

// ------------------------------------------------------------------ palette

const INK: Color32 = Color32::from_rgb(43, 39, 52);
const MUTED: Color32 = Color32::from_rgb(100, 94, 111);
const SOFT: Color32 = Color32::from_rgb(127, 120, 139);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const PURPLE_DEEP: Color32 = Color32::from_rgb(74, 50, 120);
const PURPLE_TINT: Color32 = Color32::from_rgb(241, 237, 248);
const BORDER: Color32 = Color32::from_rgb(226, 223, 232);
const PANE: Color32 = Color32::from_rgb(249, 248, 252);
const DANGER: Color32 = Color32::from_rgb(176, 58, 58);
const AMBER: Color32 = Color32::from_rgb(200, 133, 30);
const GREEN: Color32 = Color32::from_rgb(38, 110, 72);
const GREEN_TINT: Color32 = Color32::from_rgb(232, 245, 238);

const ROW_H: f32 = 54.0;
const HEADER_H: f32 = 42.0;
const FOOTER_H: f32 = 32.0;
const DRAWER_W: f32 = 408.0;

const SCHEDULES: &[(&str, &str)] = &[
    ("", "None / OTC"),
    ("G", "Schedule G"),
    ("H", "Schedule H"),
    ("H1", "Schedule H1"),
    ("X", "Schedule X"),
    ("NDPS", "Narcotic (NDPS)"),
];

// -------------------------------------------------------------------- state

/// Lightweight copy of a product row so the list can be cached between frames.
#[derive(Clone)]
struct Row {
    id: i64,
    name: String,
    brand: String,
    generic: String,
    composition: String,
    manufacturer_id: Option<i64>,
    manufacturer: String,
    unit_id: Option<i64>,
    unit: String,
    hsn: String,
    gst: String,
    gst_num: f64,
    schedule: String,
    barcode: String,
    reorder: String,
    reorder_num: f64,
    active: bool,
}

impl Row {
    fn from_record(p: &ProductRecord) -> Self {
        let gst = p.gst_percent.to_string();
        let reorder = p.reorder_level.to_string();
        Self {
            id: p.id,
            name: p.name.clone(),
            brand: p.brand_name.clone(),
            generic: p.generic_name.clone(),
            composition: p.composition.clone(),
            manufacturer_id: p.manufacturer_id,
            manufacturer: p.manufacturer_name.clone(),
            unit_id: p.unit_id,
            unit: p.unit_name.clone(),
            hsn: p.hsn_code.clone(),
            gst_num: gst.parse().unwrap_or(0.0),
            gst,
            schedule: p.schedule.clone(),
            barcode: p.barcode.clone(),
            reorder_num: reorder.parse().unwrap_or(0.0),
            reorder,
            active: p.active,
        }
    }
}

#[derive(Clone)]
struct Cache {
    year: i32,
    search: String,
    rows: Vec<Row>,
    manufacturers: Vec<(i64, String)>,
    units: Vec<(i64, String)>,
    error: String,
}

#[derive(Clone, Copy, PartialEq, Default)]
enum Status {
    #[default]
    All,
    Active,
    Inactive,
}

#[derive(Clone, Copy, PartialEq, Default)]
enum SortKey {
    #[default]
    Name,
    Manufacturer,
    Gst,
    Reorder,
}

#[derive(Clone, Default)]
struct View {
    search: String,
    status: Status,
    manufacturer_filter: Option<i64>,
    sort: SortKey,
    descending: bool,
    open: bool,
    reload: bool,
    selected: Option<i64>,
}

#[derive(Clone)]
struct ProductForm {
    year: i32,
    id: Option<i64>,
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
    active: bool,
    original: String,
}

impl ProductForm {
    fn new(year: i32) -> Self {
        let mut form = Self {
            year,
            id: None,
            name: String::new(),
            brand_name: String::new(),
            generic_name: String::new(),
            composition: String::new(),
            manufacturer_id: None,
            unit_id: None,
            hsn_code: String::new(),
            gst_percent: "5".to_owned(),
            schedule: String::new(),
            barcode: String::new(),
            reorder_level: "0".to_owned(),
            active: true,
            original: String::new(),
        };
        form.original = form.fingerprint();
        form
    }

    fn from_row(year: i32, r: &Row) -> Self {
        let mut form = Self {
            year,
            id: Some(r.id),
            name: r.name.clone(),
            brand_name: r.brand.clone(),
            generic_name: r.generic.clone(),
            composition: r.composition.clone(),
            manufacturer_id: r.manufacturer_id,
            unit_id: r.unit_id,
            hsn_code: r.hsn.clone(),
            gst_percent: r.gst.clone(),
            schedule: r.schedule.clone(),
            barcode: r.barcode.clone(),
            reorder_level: r.reorder.clone(),
            active: r.active,
            original: String::new(),
        };
        form.original = form.fingerprint();
        form
    }

    fn fingerprint(&self) -> String {
        format!(
            "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{:?}\u{1f}{:?}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
            self.name,
            self.brand_name,
            self.generic_name,
            self.composition,
            self.manufacturer_id,
            self.unit_id,
            self.hsn_code,
            self.gst_percent,
            self.schedule,
            self.barcode,
            self.reorder_level,
            self.active
        )
    }

    fn is_dirty(&self) -> bool {
        self.fingerprint() != self.original
    }

    /// Every problem with the form, used to block saving.
    fn errors(&self) -> Vec<String> {
        let mut errors = vec![
            validate_name(&self.name),
            validate_hsn(&self.hsn_code),
            validate_gst(&self.gst_percent),
            validate_reorder(&self.reorder_level),
            validate_barcode(&self.barcode),
        ];
        if self.manufacturer_id.is_none() {
            errors.push(Some("Choose a manufacturer.".into()));
        }
        if self.unit_id.is_none() {
            errors.push(Some("Choose a unit.".into()));
        }
        errors.into_iter().flatten().collect()
    }

    fn to_record(&self) -> Result<ProductRecord, String> {
        Ok(ProductRecord {
            id: self.id.unwrap_or_default(),
            name: self.name.trim().to_owned(),
            brand_name: self.brand_name.trim().to_owned(),
            generic_name: self.generic_name.trim().to_owned(),
            composition: self.composition.trim().to_owned(),
            manufacturer_id: self.manufacturer_id,
            manufacturer_name: String::new(),
            unit_id: self.unit_id,
            unit_name: String::new(),
            hsn_code: self.hsn_code.trim().to_owned(),
            gst_percent: self
                .gst_percent
                .trim()
                .parse()
                .map_err(|_| "Enter a valid GST percentage.".to_owned())?,
            schedule: self.schedule.trim().to_owned(),
            barcode: self.barcode.trim().to_owned(),
            reorder_level: self
                .reorder_level
                .trim()
                .parse()
                .map_err(|_| "Enter a whole number for the reorder level.".to_owned())?,
            active: self.active,
        })
    }
}

// --------------------------------------------------------------- validation

fn validate_name(v: &str) -> Option<String> {
    let v = v.trim();
    if v.is_empty() {
        Some("Product name is required.".into())
    } else if v.chars().count() < 2 {
        Some("Use at least 2 characters.".into())
    } else {
        None
    }
}

fn validate_hsn(v: &str) -> Option<String> {
    let v = v.trim();
    if v.is_empty() {
        Some("HSN code is required.".into())
    } else if !v.chars().all(|c| c.is_ascii_digit()) || ![4, 6, 8].contains(&v.len()) {
        Some("HSN must be 4, 6 or 8 digits (medicines are usually 3004).".into())
    } else {
        None
    }
}

fn validate_gst(v: &str) -> Option<String> {
    let v = v.trim();
    if v.is_empty() {
        return Some("GST % is required.".into());
    }
    match v.parse::<f64>() {
        Ok(n) if (0.0..=100.0).contains(&n) => None,
        _ => Some("Enter a GST rate between 0 and 100.".into()),
    }
}

fn validate_reorder(v: &str) -> Option<String> {
    let v = v.trim();
    if v.is_empty() {
        return Some("Enter 0 or more.".into());
    }
    match v.parse::<f64>() {
        Ok(n) if n >= 0.0 => None,
        _ => Some("Reorder level must be 0 or more.".into()),
    }
}

fn validate_barcode(v: &str) -> Option<String> {
    let v = v.trim();
    if v.is_empty() {
        None
    } else if v.chars().any(char::is_whitespace) {
        Some("Barcode cannot contain spaces.".into())
    } else if !(4..=32).contains(&v.chars().count()) {
        Some("Barcode should be 4 to 32 characters.".into())
    } else {
        None
    }
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

fn flag(ui: &egui::Ui, id: Id) -> bool {
    ui.data(|d| d.get_temp::<bool>(id).unwrap_or(false))
}

fn set_flag(ui: &egui::Ui, id: Id, value: bool) {
    ui.data_mut(|d| d.insert_temp(id, value));
}

fn load(app: &AsterApp, year: i32, search: &str) -> Cache {
    let mut cache = Cache {
        year,
        search: search.to_owned(),
        rows: Vec::new(),
        manufacturers: Vec::new(),
        units: Vec::new(),
        error: String::new(),
    };
    let Some(db) = app.db.as_ref() else {
        cache.error = "Database is not available.".into();
        return cache;
    };
    let products = db.products(year, search);
    let manufacturers = db.manufacturers(year);
    let units = db.units(year);
    match (products, manufacturers, units) {
        (Ok(p), Ok(m), Ok(u)) => {
            cache.rows = p.iter().map(Row::from_record).collect();
            cache.manufacturers = m
                .iter()
                .map(|m: &ManufacturerOption| (m.id, m.name.clone()))
                .collect();
            cache.units = u
                .iter()
                .map(|u: &UnitOption| (u.id, format!("{} ({})", u.name, u.short_name)))
                .collect();
        }
        (p, m, u) => {
            cache.error = p
                .err()
                .or_else(|| m.err())
                .or_else(|| u.err())
                .unwrap_or_default();
        }
    }
    cache
}

fn paint_text(
    ui: &egui::Ui,
    rect: Rect,
    text: &str,
    size: f32,
    color: Color32,
    dy: f32,
    strong: bool,
) {
    if text.is_empty() || rect.width() < 12.0 {
        return;
    }
    let mut font = FontId::proportional(size);
    if strong {
        font = FontId::proportional(size + 0.5);
    }
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap.max_width = rect.width();
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let galley = ui.painter().layout_job(job);
    let pos = Pos2::new(rect.left(), rect.center().y - galley.size().y / 2.0 + dy);
    ui.painter()
        .with_clip_rect(rect.expand2(Vec2::new(0.0, 4.0)))
        .galley(pos, galley, color);
}

/// Pill with its left edge at `left` and vertically centred on `y`.
fn paint_pill(ui: &egui::Ui, left: f32, y: f32, text: &str, fg: Color32, bg: Color32) -> f32 {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(11.5), fg);
    let w = galley.size().x + 16.0;
    let rect = Rect::from_min_size(Pos2::new(left, y - 10.0), Vec2::new(w, 20.0));
    ui.painter().rect_filled(rect, 10.0, bg);
    ui.painter().galley(
        Pos2::new(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0),
        galley,
        fg,
    );
    w
}

fn schedule_colors(schedule: &str) -> (Color32, Color32) {
    match schedule {
        "X" | "NDPS" => (
            Color32::from_rgb(160, 40, 50),
            Color32::from_rgb(252, 232, 232),
        ),
        "H" | "H1" => (
            Color32::from_rgb(160, 100, 20),
            Color32::from_rgb(253, 242, 222),
        ),
        _ => (PURPLE, PURPLE_TINT),
    }
}

// ------------------------------------------------------------------- screen

pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let ctx = ui.ctx().clone();
    let year = app.selected_year;
    let view_id = Id::new("aster_pm_view");
    let form_id = Id::new("aster_product_master_form");
    let cache_id = Id::new("aster_pm_cache");
    let search_id = Id::new("aster_pm_search");

    let mut view: View = ctx.data(|d| d.get_temp(view_id)).unwrap_or_default();
    let mut form: ProductForm = ctx
        .data(|d| d.get_temp::<ProductForm>(form_id))
        .filter(|f| f.year == year)
        .unwrap_or_else(|| ProductForm::new(year));
    let mut cache: Option<Cache> = ctx.data(|d| d.get_temp(cache_id));
    if view.reload || cache.as_ref().map_or(true, |c| c.year != year) {
        cache = Some(load(app, year, &view.search));
        view.reload = false;
    }
    let mut cache = cache.expect("cache loaded");

    // ---- keyboard shortcuts
    let (new_key, find_key, save_key, esc_key) = ctx.input(|i| {
        (
            i.modifiers.command && i.key_pressed(Key::N),
            i.modifiers.command && i.key_pressed(Key::F),
            i.modifiers.command && i.key_pressed(Key::S),
            i.key_pressed(Key::Escape),
        )
    });
    let mut save_requested = save_key && view.open;
    if new_key {
        form = ProductForm::new(year);
        view.open = true;
        view.selected = None;
        reset_touched(ui);
    }
    if find_key {
        ctx.memory_mut(|m| m.request_focus(search_id));
    }
    if esc_key && view.open && ctx.memory(|m| m.focused().is_none()) {
        view.open = false;
        view.selected = None;
    }

    // ---- page header
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("MASTERS").size(11.0).strong().color(SOFT));
            ui.add_space(4.0);
            ui.label(
                RichText::new("Products / medicines")
                    .size(28.0)
                    .strong()
                    .color(INK),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(format!(
                    "Product master for financial year {year}–{}",
                    year + 1
                ))
                .size(14.5)
                .color(MUTED),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            if button(ui, "+  New product", 140.0, true, true).clicked() {
                form = ProductForm::new(year);
                view.open = true;
                view.selected = None;
                reset_touched(ui);
            }
        });
    });
    ui.add_space(18.0);

    // ---- stats
    let total = cache.rows.len();
    let active = cache.rows.iter().filter(|r| r.active).count();
    let scheduled = cache.rows.iter().filter(|r| !r.schedule.is_empty()).count();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        stat_chip(ui, "Products", total, PURPLE, PURPLE_TINT);
        stat_chip(ui, "Active", active, GREEN, GREEN_TINT);
        stat_chip(
            ui,
            "Inactive",
            total - active,
            MUTED,
            Color32::from_rgb(240, 238, 245),
        );
        stat_chip(
            ui,
            "Scheduled drugs",
            scheduled,
            Color32::from_rgb(160, 100, 20),
            Color32::from_rgb(253, 242, 222),
        );
    });
    ui.add_space(14.0);

    // ---- toolbar
    toolbar(ui, &mut view, &cache, search_id);
    ui.add_space(14.0);

    // search changed → reload from the database
    if view.search != cache.search {
        cache = load(app, year, &view.search);
    }

    // ---- filtered + sorted rows
    let mut shown: Vec<&Row> = cache
        .rows
        .iter()
        .filter(|r| match view.status {
            Status::All => true,
            Status::Active => r.active,
            Status::Inactive => !r.active,
        })
        .filter(|r| {
            view.manufacturer_filter
                .map_or(true, |id| r.manufacturer_id == Some(id))
        })
        .collect();
    shown.sort_by(|a, b| {
        let order = match view.sort {
            SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortKey::Manufacturer => a
                .manufacturer
                .to_lowercase()
                .cmp(&b.manufacturer.to_lowercase()),
            SortKey::Gst => a.gst_num.total_cmp(&b.gst_num),
            SortKey::Reorder => a.reorder_num.total_cmp(&b.reorder_num),
        };
        if view.descending {
            order.reverse()
        } else {
            order
        }
    });

    // ---- table + drawer
    let total_w = ui.available_width();
    let drawer_t =
        ctx.animate_bool_with_time(Id::new("aster_pm_drawer"), view.open, anim(&ctx) * 2.0);
    let drawer_full = DRAWER_W.min(total_w * 0.46);
    let drawer_w = drawer_full * (1.0 - (1.0 - drawer_t).powi(3));
    let gap = 16.0 * drawer_t;
    let table_w = (total_w - drawer_w - gap).max(320.0);
    let avail_h = (ui.clip_rect().bottom() - ui.cursor().top() - 24.0).max(380.0);

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        table(
            ui, app, year, &mut view, &mut form, &cache, &shown, table_w, avail_h,
        );
        if drawer_t > 0.01 {
            ui.add_space(gap);
            drawer(
                ui,
                app,
                year,
                &mut view,
                &mut form,
                &cache,
                drawer_w,
                drawer_full,
                avail_h,
                drawer_t,
                &mut save_requested,
            );
        }
    });

    ctx.data_mut(|d| {
        d.insert_temp(view_id, view);
        d.insert_temp(form_id, form);
        d.insert_temp(cache_id, cache);
    });
}

fn reset_touched(ui: &egui::Ui) {
    set_flag(ui, Id::new("aster_pm_submitted"), false);
    for label in [
        "Product name",
        "HSN code",
        "GST %",
        "Barcode",
        "Reorder level",
        "Manufacturer",
        "Unit",
    ] {
        set_flag(ui, Id::new(("aster_pm_touched", label)), false);
    }
}

// ------------------------------------------------------------------ toolbar

fn toolbar(ui: &mut egui::Ui, view: &mut View, cache: &Cache, search_id: Id) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;

        // search
        let width = (ui.available_width() * 0.36).clamp(220.0, 380.0);
        let response = ui.add(
            egui::TextEdit::singleline(&mut view.search)
                .id(search_id)
                .desired_width(width)
                .margin(Margin::symmetric(12, 8))
                .hint_text("Search name, brand, composition, barcode…  (Ctrl+F)"),
        );
        let _ = response;
        if !view.search.is_empty() && small_icon_button(ui, "×", "Clear search").clicked() {
            view.search.clear();
        }

        // status segmented control
        segmented(
            ui,
            &mut view.status,
            &[
                (Status::All, "All"),
                (Status::Active, "Active"),
                (Status::Inactive, "Inactive"),
            ],
        );

        // manufacturer filter
        let selected = view
            .manufacturer_filter
            .and_then(|id| cache.manufacturers.iter().find(|m| m.0 == id))
            .map(|m| m.1.clone())
            .unwrap_or_else(|| "All manufacturers".to_owned());
        egui::ComboBox::from_id_salt("aster_pm_man_filter")
            .width(190.0)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut view.manufacturer_filter, None, "All manufacturers");
                for (id, name) in &cache.manufacturers {
                    ui.selectable_value(&mut view.manufacturer_filter, Some(*id), name);
                }
            });

        if (view.status != Status::All || view.manufacturer_filter.is_some())
            && link(ui, "Reset filters").clicked()
        {
            view.status = Status::All;
            view.manufacturer_filter = None;
        }
    });
}

fn segmented<T: Copy + PartialEq>(ui: &mut egui::Ui, value: &mut T, options: &[(T, &str)]) {
    let seg_w = 78.0;
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(seg_w * options.len() as f32 + 6.0, 36.0),
        Sense::hover(),
    );
    ui.painter()
        .rect_filled(rect, 10.0, Color32::from_rgb(240, 238, 245));
    let ctx = ui.ctx().clone();
    let index = options.iter().position(|o| o.0 == *value).unwrap_or(0) as f32;
    let x = ctx.animate_value_with_time(
        Id::new(("aster_seg", options.len(), options[0].1)),
        index,
        anim(&ctx) * 1.6,
    );
    let highlight = Rect::from_min_size(
        Pos2::new(rect.left() + 3.0 + x * seg_w, rect.top() + 3.0),
        Vec2::new(seg_w, 30.0),
    );
    ui.painter().rect_filled(highlight, 8.0, Color32::WHITE);
    ui.painter().rect_stroke(
        highlight,
        8.0,
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );
    for (i, (option, label)) in options.iter().enumerate() {
        let seg = Rect::from_min_size(
            Pos2::new(rect.left() + 3.0 + i as f32 * seg_w, rect.top() + 3.0),
            Vec2::new(seg_w, 30.0),
        );
        let response = ui.interact(seg, Id::new(("aster_seg_item", *label)), Sense::click());
        let selected = *option == *value;
        ui.painter().text(
            seg.center(),
            egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(13.0),
            if selected { PURPLE } else { MUTED },
        );
        if response.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            *value = *option;
        }
    }
}

// -------------------------------------------------------------------- table

#[derive(Clone, Copy, PartialEq)]
enum Col {
    Product,
    Composition,
    Manufacturer,
    Unit,
    Hsn,
    Gst,
    Reorder,
    Status,
    Actions,
}

/// (column, min width, weight, priority - lower is kept first when space is short)
const COLUMNS: [(Col, f32, f32, u8); 9] = [
    (Col::Product, 220.0, 3.0, 0),
    (Col::Composition, 170.0, 2.2, 7),
    (Col::Manufacturer, 140.0, 1.8, 3),
    (Col::Unit, 78.0, 0.5, 4),
    (Col::Hsn, 78.0, 0.5, 5),
    (Col::Gst, 74.0, 0.4, 1),
    (Col::Reorder, 84.0, 0.5, 6),
    (Col::Status, 96.0, 0.5, 2),
    (Col::Actions, 84.0, 0.0, 0),
];

fn choose_columns(width: f32) -> Vec<(Col, f32)> {
    let mut order: Vec<usize> = (0..COLUMNS.len()).collect();
    order.sort_by_key(|i| COLUMNS[*i].3);
    let mut keep = vec![false; COLUMNS.len()];
    let mut used = 0.0;
    for i in order {
        let (_, min, _, priority) = COLUMNS[i];
        if priority == 0 || used + min <= width {
            keep[i] = true;
            used += min;
        }
    }
    let weights: f32 = COLUMNS
        .iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, c)| c.2)
        .sum();
    let extra = (width - used).max(0.0);
    COLUMNS
        .iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, c)| {
            (
                c.0,
                c.1 + if weights > 0.0 {
                    extra * c.2 / weights
                } else {
                    0.0
                },
            )
        })
        .collect()
}

fn column_label(col: Col) -> &'static str {
    match col {
        Col::Product => "PRODUCT",
        Col::Composition => "COMPOSITION",
        Col::Manufacturer => "MANUFACTURER",
        Col::Unit => "UNIT",
        Col::Hsn => "HSN",
        Col::Gst => "GST",
        Col::Reorder => "REORDER",
        Col::Status => "STATUS",
        Col::Actions => "",
    }
}

fn column_sort(col: Col) -> Option<SortKey> {
    match col {
        Col::Product => Some(SortKey::Name),
        Col::Manufacturer => Some(SortKey::Manufacturer),
        Col::Gst => Some(SortKey::Gst),
        Col::Reorder => Some(SortKey::Reorder),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn table(
    ui: &mut egui::Ui,
    app: &mut AsterApp,
    year: i32,
    view: &mut View,
    form: &mut ProductForm,
    cache: &Cache,
    shown: &[&Row],
    width: f32,
    height: f32,
) {
    let ctx = ui.ctx().clone();
    let (card, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    ui.painter().rect_filled(card, 12.0, Color32::WHITE);
    ui.painter().rect_stroke(
        card,
        12.0,
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );

    let inner_w = width - 14.0;
    let columns = choose_columns(inner_w);

    // header
    let header = Rect::from_min_size(card.min, Vec2::new(width, HEADER_H));
    ui.painter().rect_filled(
        header.shrink(1.0),
        egui::CornerRadius {
            nw: 11,
            ne: 11,
            sw: 0,
            se: 0,
        },
        PANE,
    );
    ui.painter().hline(
        header.x_range(),
        header.bottom(),
        Stroke::new(1.0_f32, BORDER),
    );
    let mut x = header.left() + 18.0;
    let last = columns.len().saturating_sub(1);
    for (i, (col, w)) in columns.iter().enumerate() {
        let w = if i == 0 { *w - 18.0 } else { *w };
        let cell = Rect::from_min_size(Pos2::new(x, header.top()), Vec2::new(w, HEADER_H));
        let label = column_label(*col);
        if let Some(key) = column_sort(*col) {
            let response = ui.interact(cell, Id::new(("aster_pm_sort", label)), Sense::click());
            let active = view.sort == key;
            let color = if active { PURPLE } else { SOFT };
            let galley =
                ui.painter()
                    .layout_no_wrap(label.to_owned(), FontId::proportional(11.0), color);
            let gw = galley.size().x;
            ui.painter().galley(
                Pos2::new(cell.left(), cell.center().y - galley.size().y / 2.0),
                galley,
                color,
            );
            if active {
                let c = Pos2::new(cell.left() + gw + 10.0, cell.center().y);
                let dir = if view.descending { 1.0 } else { -1.0 };
                let stroke = Stroke::new(1.5_f32, PURPLE);
                ui.painter().line_segment(
                    [
                        c + Vec2::new(-3.5, -1.5 * dir),
                        c + Vec2::new(0.0, 1.5 * dir),
                    ],
                    stroke,
                );
                ui.painter().line_segment(
                    [
                        c + Vec2::new(0.0, 1.5 * dir),
                        c + Vec2::new(3.5, -1.5 * dir),
                    ],
                    stroke,
                );
            }
            if response.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                if view.sort == key {
                    view.descending = !view.descending;
                } else {
                    view.sort = key;
                    view.descending = false;
                }
            }
        } else if i != last {
            ui.painter().text(
                Pos2::new(cell.left(), cell.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                FontId::proportional(11.0),
                SOFT,
            );
        }
        x += w;
    }

    // footer
    let footer = Rect::from_min_max(Pos2::new(card.left(), card.bottom() - FOOTER_H), card.max);
    ui.painter()
        .hline(footer.x_range(), footer.top(), Stroke::new(1.0_f32, BORDER));
    ui.painter().text(
        Pos2::new(footer.left() + 18.0, footer.center().y),
        egui::Align2::LEFT_CENTER,
        format!("Showing {} of {} products", shown.len(), cache.rows.len()),
        FontId::proportional(12.0),
        MUTED,
    );
    if width > 560.0 {
        ui.painter().text(
            Pos2::new(footer.right() - 18.0, footer.center().y),
            egui::Align2::RIGHT_CENTER,
            "Click a row to edit  ·  Ctrl+N new product",
            FontId::proportional(12.0),
            SOFT,
        );
    }

    // body
    let body = Rect::from_min_max(
        Pos2::new(card.left() + 1.0, header.bottom() + 1.0),
        Pos2::new(card.right() - 1.0, footer.top() - 1.0),
    );

    if !cache.error.is_empty() {
        empty_state(ui, body, "Could not load products", &cache.error, true);
        return;
    }
    if shown.is_empty() {
        let (title, detail) = if cache.rows.is_empty() && view.search.is_empty() {
            (
                "No products yet",
                "Add your first medicine with the New product button.",
            )
        } else {
            (
                "No products match",
                "Try a different search or reset the filters.",
            )
        };
        empty_state(ui, body, title, detail, false);
        return;
    }

    let mut body_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(body)
            .layout(egui::Layout::top_down(Align::Min)),
    );
    let mut toggle: Option<(i64, bool)> = None;
    let mut open_row: Option<&Row> = None;

    egui::ScrollArea::vertical()
        .id_salt("aster_pm_rows")
        .auto_shrink([false, false])
        .show_rows(&mut body_ui, ROW_H, shown.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for index in range {
                let row = shown[index];
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_H), Sense::click());
                let selected = view.selected == Some(row.id) && view.open;
                let hover = ctx.animate_bool_with_time(
                    Id::new(("aster_pm_row", row.id)),
                    response.hovered(),
                    anim(&ctx),
                );
                let fill = if selected {
                    PURPLE_TINT
                } else {
                    mix(Color32::WHITE, Color32::from_rgb(249, 247, 253), hover)
                };
                ui.painter().rect_filled(rect, 0.0, fill);
                if selected {
                    ui.painter().rect_filled(
                        Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height())),
                        0.0,
                        PURPLE,
                    );
                }
                ui.painter().hline(
                    rect.x_range(),
                    rect.bottom() - 0.5,
                    Stroke::new(1.0_f32, Color32::from_rgb(240, 238, 245)),
                );

                let dim = if row.active { 1.0 } else { 0.55 };
                let mut x = rect.left() + 18.0;
                let mut action_hit = false;
                for (i, (col, w)) in columns.iter().enumerate() {
                    let w = if i == 0 { *w - 18.0 } else { *w };
                    let pad = 10.0;
                    let cell = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(w, ROW_H));
                    let text_rect =
                        Rect::from_min_max(cell.min, Pos2::new(cell.right() - pad, cell.bottom()));
                    match col {
                        Col::Product => {
                            paint_text(
                                ui,
                                text_rect,
                                &row.name,
                                14.0,
                                INK.gamma_multiply(dim),
                                -8.0,
                                true,
                            );
                            let sub = match (row.brand.is_empty(), row.generic.is_empty()) {
                                (false, _) => row.brand.clone(),
                                (true, false) => row.generic.clone(),
                                _ => String::new(),
                            };
                            let mut sub_rect = text_rect;
                            if !row.schedule.is_empty() {
                                let (fg, bg) = schedule_colors(&row.schedule);
                                let pill_w = paint_pill(
                                    ui,
                                    text_rect.left(),
                                    text_rect.center().y + 10.0,
                                    &format!("Sch {}", row.schedule),
                                    fg,
                                    bg,
                                );
                                sub_rect.min.x += pill_w + 8.0;
                            }
                            paint_text(ui, sub_rect, &sub, 12.0, SOFT, 10.0, false);
                        }
                        Col::Composition => {
                            let text = if row.composition.is_empty() {
                                &row.generic
                            } else {
                                &row.composition
                            };
                            paint_text(
                                ui,
                                text_rect,
                                text,
                                12.5,
                                MUTED.gamma_multiply(dim),
                                0.0,
                                false,
                            );
                        }
                        Col::Manufacturer => {
                            paint_text(
                                ui,
                                text_rect,
                                &row.manufacturer,
                                13.0,
                                MUTED.gamma_multiply(dim),
                                0.0,
                                false,
                            );
                        }
                        Col::Unit => {
                            paint_text(
                                ui,
                                text_rect,
                                &row.unit,
                                13.0,
                                MUTED.gamma_multiply(dim),
                                0.0,
                                false,
                            );
                        }
                        Col::Hsn => {
                            paint_text(
                                ui,
                                text_rect,
                                &row.hsn,
                                13.0,
                                MUTED.gamma_multiply(dim),
                                0.0,
                                false,
                            );
                        }
                        Col::Gst => {
                            paint_pill(
                                ui,
                                cell.left(),
                                cell.center().y,
                                &format!("{}%", row.gst),
                                PURPLE,
                                PURPLE_TINT,
                            );
                        }
                        Col::Reorder => {
                            paint_text(
                                ui,
                                text_rect,
                                &row.reorder,
                                13.0,
                                MUTED.gamma_multiply(dim),
                                0.0,
                                false,
                            );
                        }
                        Col::Status => {
                            let (text, fg, bg) = if row.active {
                                ("Active", GREEN, GREEN_TINT)
                            } else {
                                ("Inactive", MUTED, Color32::from_rgb(240, 238, 245))
                            };
                            paint_pill(ui, cell.left(), cell.center().y, text, fg, bg);
                        }
                        Col::Actions => {
                            let edit_rect = Rect::from_center_size(
                                Pos2::new(cell.left() + 24.0, cell.center().y),
                                Vec2::splat(30.0),
                            );
                            let toggle_rect = Rect::from_center_size(
                                Pos2::new(cell.left() + 58.0, cell.center().y),
                                Vec2::splat(30.0),
                            );
                            let edit = ui.interact(
                                edit_rect,
                                Id::new(("aster_pm_edit", row.id)),
                                Sense::click(),
                            );
                            paint_action_icon(ui, edit_rect, ActionIcon::Edit, edit.hovered());
                            if edit.clicked() {
                                action_hit = true;
                                open_row = Some(row);
                            }
                            edit.on_hover_text("Edit product");

                            let tog = ui.interact(
                                toggle_rect,
                                Id::new(("aster_pm_toggle", row.id)),
                                Sense::click(),
                            );
                            paint_action_icon(
                                ui,
                                toggle_rect,
                                if row.active {
                                    ActionIcon::Deactivate
                                } else {
                                    ActionIcon::Activate
                                },
                                tog.hovered(),
                            );
                            if tog.clicked() {
                                action_hit = true;
                                toggle = Some((row.id, !row.active));
                            }
                            tog.on_hover_text(if row.active { "Deactivate" } else { "Activate" });
                        }
                    }
                    x += w;
                }
                if response.hovered() {
                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if response.clicked() && !action_hit {
                    open_row = Some(row);
                }
            }
        });

    if let Some(row) = open_row {
        *form = ProductForm::from_row(year, row);
        view.open = true;
        view.selected = Some(row.id);
        reset_touched(ui);
    }
    if let Some((id, active)) = toggle {
        let result = match app.db.as_ref() {
            Some(db) => db.set_product_active(year, id, active),
            None => Err("Database is not available.".to_owned()),
        };
        match result {
            Ok(()) => {
                app.profile_message = if active {
                    "Product activated successfully."
                } else {
                    "Product deactivated successfully."
                }
                .into();
                view.reload = true;
                if form.id == Some(id) {
                    form.active = active;
                    form.original = form.fingerprint();
                }
            }
            Err(error) => app.profile_message = error,
        }
    }
}

fn empty_state(ui: &mut egui::Ui, body: Rect, title: &str, detail: &str, error: bool) {
    let c = body.center();
    let color = if error { DANGER } else { PURPLE };
    ui.painter().circle_filled(
        c - Vec2::new(0.0, 36.0),
        28.0,
        if error {
            Color32::from_rgb(252, 238, 238)
        } else {
            PURPLE_TINT
        },
    );
    let stroke = Stroke::new(1.8_f32, color);
    let b = c - Vec2::new(0.0, 36.0);
    ui.painter().rect_stroke(
        Rect::from_center_size(b, Vec2::new(18.0, 14.0)),
        3.0,
        stroke,
        egui::StrokeKind::Inside,
    );
    ui.painter().line_segment(
        [b + Vec2::new(-9.0, -2.0), b + Vec2::new(9.0, -2.0)],
        stroke,
    );
    ui.painter().text(
        c + Vec2::new(0.0, 8.0),
        egui::Align2::CENTER_CENTER,
        title,
        FontId::proportional(16.0),
        INK,
    );
    ui.painter().text(
        c + Vec2::new(0.0, 32.0),
        egui::Align2::CENTER_CENTER,
        detail,
        FontId::proportional(13.0),
        MUTED,
    );
}

#[derive(Clone, Copy)]
enum ActionIcon {
    Edit,
    Deactivate,
    Activate,
}

fn paint_action_icon(ui: &egui::Ui, rect: Rect, icon: ActionIcon, hovered: bool) {
    if hovered {
        ui.painter()
            .rect_filled(rect, 8.0, Color32::from_rgb(238, 233, 247));
    }
    let color = if hovered { PURPLE } else { SOFT };
    let stroke = Stroke::new(1.5_f32, color);
    let c = rect.center();
    match icon {
        ActionIcon::Edit => {
            ui.painter().line_segment(
                [c + Vec2::new(-5.0, 5.0), c + Vec2::new(4.0, -4.0)],
                Stroke::new(2.4_f32, color),
            );
            ui.painter()
                .line_segment([c + Vec2::new(-6.0, 6.5), c + Vec2::new(-5.0, 3.5)], stroke);
            ui.painter()
                .line_segment([c + Vec2::new(-6.0, 6.5), c + Vec2::new(-3.0, 5.5)], stroke);
            ui.painter()
                .line_segment([c + Vec2::new(1.0, 6.5), c + Vec2::new(6.0, 6.5)], stroke);
        }
        ActionIcon::Deactivate => {
            ui.painter().circle_stroke(c, 6.5, stroke);
            ui.painter()
                .line_segment([c + Vec2::new(-4.5, 4.5), c + Vec2::new(4.5, -4.5)], stroke);
        }
        ActionIcon::Activate => {
            let green = if hovered { GREEN } else { SOFT };
            let s = Stroke::new(1.5_f32, green);
            ui.painter().circle_stroke(c, 6.5, s);
            ui.painter()
                .line_segment([c + Vec2::new(-3.0, 0.0), c + Vec2::new(-1.0, 2.5)], s);
            ui.painter()
                .line_segment([c + Vec2::new(-1.0, 2.5), c + Vec2::new(3.5, -2.5)], s);
        }
    }
}

// ------------------------------------------------------------------- drawer

#[allow(clippy::too_many_arguments)]
fn drawer(
    ui: &mut egui::Ui,
    app: &mut AsterApp,
    year: i32,
    view: &mut View,
    form: &mut ProductForm,
    cache: &Cache,
    width: f32,
    full_width: f32,
    height: f32,
    t: f32,
    save_requested: &mut bool,
) {
    let (outer, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_size(
                outer.min,
                Vec2::new(full_width, height),
            ))
            .layout(egui::Layout::top_down(Align::Min)),
    );
    child.set_clip_rect(outer.intersect(ui.clip_rect()));
    child.set_opacity(t);
    let ui = &mut child;
    let card = Rect::from_min_size(outer.min, Vec2::new(full_width, height));
    ui.painter().rect_filled(card, 12.0, Color32::WHITE);
    ui.painter().rect_stroke(
        card,
        12.0,
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );

    let submitted_id = Id::new("aster_pm_submitted");
    let submitted = flag(ui, submitted_id);
    let editing = form.id.is_some();
    let dirty = form.is_dirty();

    // header
    let head = Rect::from_min_size(card.min, Vec2::new(full_width, 64.0));
    ui.painter().text(
        Pos2::new(head.left() + 22.0, head.top() + 22.0),
        egui::Align2::LEFT_CENTER,
        if editing {
            "Edit product"
        } else {
            "New product"
        },
        FontId::proportional(17.0),
        INK,
    );
    let sub = if editing {
        if form.name.trim().is_empty() {
            "Editing existing product".to_owned()
        } else {
            form.name.trim().to_owned()
        }
    } else {
        "Fields marked * are required".to_owned()
    };
    paint_text(
        ui,
        Rect::from_min_size(
            Pos2::new(head.left() + 22.0, head.top() + 36.0),
            Vec2::new(full_width - 90.0, 20.0),
        ),
        &sub,
        12.5,
        MUTED,
        0.0,
        false,
    );
    let close_rect = Rect::from_center_size(
        Pos2::new(head.right() - 28.0, head.top() + 30.0),
        Vec2::splat(30.0),
    );
    let close = ui.interact(close_rect, Id::new("aster_pm_close"), Sense::click());
    if close.hovered() {
        ui.painter()
            .rect_filled(close_rect, 8.0, Color32::from_rgb(243, 240, 249));
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let cc = close_rect.center();
    let xs = Stroke::new(1.6_f32, if close.hovered() { PURPLE } else { SOFT });
    ui.painter()
        .line_segment([cc + Vec2::new(-4.5, -4.5), cc + Vec2::new(4.5, 4.5)], xs);
    ui.painter()
        .line_segment([cc + Vec2::new(-4.5, 4.5), cc + Vec2::new(4.5, -4.5)], xs);
    if close.on_hover_text("Close  (Esc)").clicked() {
        view.open = false;
        view.selected = None;
    }
    ui.painter()
        .hline(card.x_range(), head.bottom(), Stroke::new(1.0_f32, BORDER));

    // footer geometry
    let foot_h = 76.0;
    let foot = Rect::from_min_max(Pos2::new(card.left(), card.bottom() - foot_h), card.max);
    let body = Rect::from_min_max(
        Pos2::new(card.left() + 22.0, head.bottom() + 14.0),
        Pos2::new(card.right() - 22.0, foot.top() - 6.0),
    );

    // body (scrolls)
    let mut body_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(body)
            .layout(egui::Layout::top_down(Align::Min)),
    );
    egui::ScrollArea::vertical()
        .id_salt("aster_pm_form_scroll")
        .auto_shrink([false, false])
        .show(&mut body_ui, |ui| {
            ui.set_width(body.width() - 10.0);
            section(ui, "Identity");
            field(
                ui,
                "Product name",
                true,
                "e.g. Paracetamol 500 mg",
                &mut form.name,
                submitted,
                &validate_name,
            );
            field(
                ui,
                "Brand name",
                false,
                "Brand / trade name",
                &mut form.brand_name,
                submitted,
                &|_: &str| None,
            );
            field(
                ui,
                "Generic name",
                false,
                "Salt / generic name",
                &mut form.generic_name,
                submitted,
                &|_: &str| None,
            );
            field(
                ui,
                "Composition",
                false,
                "e.g. Paracetamol IP 500 mg",
                &mut form.composition,
                submitted,
                &|_: &str| None,
            );

            section(ui, "Classification");
            picker(
                ui,
                "Manufacturer",
                "Select manufacturer",
                &cache.manufacturers,
                &mut form.manufacturer_id,
                submitted,
            );
            picker(
                ui,
                "Unit",
                "Select unit",
                &cache.units,
                &mut form.unit_id,
                submitted,
            );
            schedule_picker(ui, &mut form.schedule);

            section(ui, "Tax & stock");
            field(
                ui,
                "HSN code",
                true,
                "4, 6 or 8 digits",
                &mut form.hsn_code,
                submitted,
                &validate_hsn,
            );
            let gst = field(
                ui,
                "GST %",
                true,
                "e.g. 5",
                &mut form.gst_percent,
                submitted,
                &validate_gst,
            );
            let _ = gst;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(RichText::new("Quick pick").size(11.5).color(SOFT));
                for rate in ["0", "5", "18"] {
                    let selected = form
                        .gst_percent
                        .trim()
                        .parse::<f64>()
                        .ok()
                        .zip(rate.parse::<f64>().ok())
                        .map_or(false, |(a, b)| (a - b).abs() < f64::EPSILON);
                    if chip(ui, &format!("{rate}%"), selected).clicked() {
                        form.gst_percent = rate.to_owned();
                    }
                }
            });
            ui.add_space(10.0);
            field(
                ui,
                "Barcode",
                false,
                "Scan or type",
                &mut form.barcode,
                submitted,
                &validate_barcode,
            );
            field(
                ui,
                "Reorder level",
                true,
                "Minimum stock before reordering",
                &mut form.reorder_level,
                submitted,
                &validate_reorder,
            );

            section(ui, "Availability");
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Active").size(13.5).strong().color(INK));
                    ui.label(
                        RichText::new("Inactive products are hidden from billing")
                            .size(12.0)
                            .color(MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    toggle_switch(ui, &mut form.active);
                });
            });
            ui.add_space(12.0);
        });

    // footer
    ui.painter()
        .hline(card.x_range(), foot.top(), Stroke::new(1.0_f32, BORDER));
    let save_rect = Rect::from_min_size(
        Pos2::new(foot.left() + 22.0, foot.top() + 14.0),
        Vec2::new(full_width - 44.0 - 108.0, 40.0),
    );
    let clear_rect = Rect::from_min_size(
        Pos2::new(save_rect.right() + 10.0, save_rect.top()),
        Vec2::new(98.0, 40.0),
    );
    let save_label = if editing {
        "Save changes"
    } else {
        "Save product"
    };
    let save = button_at(ui, save_rect, save_label, true, true, "aster_pm_save");
    let clear = button_at(
        ui,
        clear_rect,
        if editing { "Cancel" } else { "Clear" },
        true,
        false,
        "aster_pm_clear",
    );
    if dirty {
        ui.painter().circle_filled(
            Pos2::new(foot.left() + 28.0, foot.bottom() - 11.0),
            3.0,
            AMBER,
        );
        ui.painter().text(
            Pos2::new(foot.left() + 38.0, foot.bottom() - 11.0),
            egui::Align2::LEFT_CENTER,
            "Unsaved changes",
            FontId::proportional(11.5),
            AMBER,
        );
    }

    if clear.clicked() {
        if editing {
            view.open = false;
            view.selected = None;
        }
        *form = ProductForm::new(year);
        reset_touched(ui);
    }
    if save.clicked() || *save_requested {
        *save_requested = false;
        set_flag(ui, submitted_id, true);
        match save_product(app, form, year) {
            Ok(()) => {
                app.profile_message = "Product saved successfully.".into();
                view.reload = true;
                if editing {
                    view.open = false;
                    view.selected = None;
                }
                // Stay open with a blank form after adding so several products can be entered in a row.
                *form = ProductForm::new(year);
                reset_touched(ui);
            }
            Err(error) => app.profile_message = error,
        }
    }
}

fn save_product(app: &AsterApp, form: &ProductForm, year: i32) -> Result<(), String> {
    let errors = form.errors();
    if !errors.is_empty() {
        return Err(format!(
            "Please correct the highlighted fields ({} to fix).",
            errors.len()
        ));
    }
    let record = form.to_record()?;
    let db = app
        .db
        .as_ref()
        .ok_or_else(|| "Database is not available.".to_owned())?;

    // Duplicate checks against the full product list.
    let existing = db.products(year, "")?;
    let this_id = form.id.unwrap_or(-1);
    let name = form.name.trim().to_lowercase();
    let barcode = form.barcode.trim();
    for p in &existing {
        if p.id == this_id {
            continue;
        }
        if p.name.trim().to_lowercase() == name && p.manufacturer_id == form.manufacturer_id {
            return Err("A product with this name and manufacturer already exists.".into());
        }
        if !barcode.is_empty() && p.barcode.trim() == barcode {
            return Err(format!("Barcode {barcode} is already used by {}.", p.name));
        }
    }
    db.save_product(year, form.id, &record)
}

// ------------------------------------------------------------------ widgets

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .size(10.5)
            .strong()
            .color(SOFT),
    );
    ui.add_space(8.0);
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
    let message_id = Id::new(("aster_pm_msg", label));
    if let Some(message) = error {
        ui.data_mut(|d| d.insert_temp(message_id, message.clone()));
    }
    let t = ctx.animate_bool_with_time(Id::new(("aster_pm_err", label)), show, anim(&ctx));
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

fn field(
    ui: &mut egui::Ui,
    label: &str,
    required: bool,
    hint: &str,
    value: &mut String,
    submitted: bool,
    validate: &dyn Fn(&str) -> Option<String>,
) -> egui::Response {
    label_row(ui, label, required);
    let error = if required || !value.trim().is_empty() {
        validate(value)
    } else {
        None
    };
    let touched_id = Id::new(("aster_pm_touched", label));
    let show_error = error.is_some() && (submitted || flag(ui, touched_id));
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

/// Searchable drop-down for manufacturer / unit.
fn picker(
    ui: &mut egui::Ui,
    label: &str,
    placeholder: &str,
    options: &[(i64, String)],
    value: &mut Option<i64>,
    submitted: bool,
) {
    label_row(ui, label, true);
    let error = if value.is_none() {
        Some(format!("Choose a {}.", label.to_lowercase()))
    } else {
        None
    };
    let touched_id = Id::new(("aster_pm_touched", label));
    let show_error = error.is_some() && (submitted || flag(ui, touched_id));
    let selected = value
        .and_then(|id| options.iter().find(|o| o.0 == id))
        .map(|o| o.1.clone())
        .unwrap_or_else(|| placeholder.to_owned());
    let filter_id = Id::new(("aster_pm_filter", label));
    let open_id = Id::new(("aster_pm_open", label));
    let was_open = flag(ui, open_id);

    let start = ui.cursor().min;
    let width = ui.available_width();
    let result = egui::ComboBox::from_id_salt(("aster_pm_picker", label))
        .width(width - 8.0)
        .selected_text(selected)
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
                .max_height(220.0)
                .show(ui, |ui| {
                    if options.is_empty() {
                        ui.label(RichText::new("No options available").color(SOFT));
                    }
                    for (id, name) in options {
                        if !needle.is_empty() && !name.to_lowercase().contains(&needle) {
                            continue;
                        }
                        if ui.selectable_label(*value == Some(*id), name).clicked() {
                            *value = Some(*id);
                            set_flag(ui, touched_id, true);
                        }
                    }
                });
            ui.data_mut(|d| d.insert_temp(filter_id, filter));
        });
    let open_now = result.inner.is_some();
    set_flag(ui, open_id, open_now);
    if !open_now {
        ui.data_mut(|d| d.remove::<String>(filter_id));
    }
    if show_error {
        let rect =
            Rect::from_min_size(start, Vec2::new(width - 8.0, result.response.rect.height()));
        ui.painter().rect_stroke(
            rect.expand(1.0),
            6.0,
            Stroke::new(1.5_f32, DANGER),
            egui::StrokeKind::Outside,
        );
    }
    error_line(ui, label, &error, show_error);
    ui.add_space(10.0);
}

fn schedule_picker(ui: &mut egui::Ui, schedule: &mut String) {
    label_row(ui, "Drug schedule", false);
    let known = SCHEDULES
        .iter()
        .find(|(code, _)| *code == schedule.as_str());
    let selected = match known {
        Some((_, name)) => (*name).to_owned(),
        None => schedule.clone(),
    };
    egui::ComboBox::from_id_salt("aster_pm_schedule")
        .width(ui.available_width() - 8.0)
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for (code, name) in SCHEDULES {
                if ui
                    .selectable_label(schedule.as_str() == *code, *name)
                    .clicked()
                {
                    *schedule = (*code).to_owned();
                }
            }
        });
    if matches!(schedule.as_str(), "H" | "H1" | "X" | "NDPS") {
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Sold only against a valid prescription. Keep records as required by your licence.",
            )
            .size(11.5)
            .color(Color32::from_rgb(160, 100, 20)),
        );
    }
    ui.add_space(10.0);
}

fn chip(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(12.0), INK);
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(galley.size().x + 20.0, 24.0), Sense::click());
    let ctx = ui.ctx().clone();
    let t = ctx.animate_bool_with_time(response.id, selected, anim(&ctx));
    let hover = ctx.animate_bool_with_time(response.id.with("h"), response.hovered(), anim(&ctx));
    ui.painter().rect_filled(
        rect,
        12.0,
        mix(
            mix(
                Color32::from_rgb(243, 240, 249),
                Color32::from_rgb(234, 228, 245),
                hover,
            ),
            PURPLE,
            t,
        ),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::proportional(12.0),
        mix(MUTED, Color32::WHITE, t),
    );
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(42.0, 24.0), Sense::click());
    if response.clicked() {
        *on = !*on;
    }
    let ctx = ui.ctx().clone();
    let t = ctx.animate_bool_with_time(response.id, *on, anim(&ctx) * 1.3);
    ui.painter()
        .rect_filled(rect, 12.0, mix(Color32::from_rgb(214, 210, 224), PURPLE, t));
    let knob_x = egui::lerp((rect.left() + 12.0)..=(rect.right() - 12.0), t);
    ui.painter()
        .circle_filled(Pos2::new(knob_x, rect.center().y), 9.0, Color32::WHITE);
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn stat_chip(ui: &mut egui::Ui, label: &str, count: usize, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(10.0)
        .inner_margin(Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                ui.label(
                    RichText::new(count.to_string())
                        .size(15.0)
                        .strong()
                        .color(fg),
                );
                ui.label(RichText::new(label).size(12.5).color(fg));
            });
        });
}

fn link(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let response = ui
        .add(egui::Label::new(RichText::new(label).size(12.5).color(PURPLE)).sense(Sense::click()));
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn small_icon_button(ui: &mut egui::Ui, glyph: &str, tip: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 8.0, Color32::from_rgb(243, 240, 249));
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(18.0),
        MUTED,
    );
    response.on_hover_text(tip)
}

fn button(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
    enabled: bool,
    primary: bool,
) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 38.0), Sense::hover());
    button_at(ui, rect, label, enabled, primary, label)
}

fn button_at(
    ui: &mut egui::Ui,
    rect: Rect,
    label: &str,
    enabled: bool,
    primary: bool,
    salt: &str,
) -> egui::Response {
    let ctx = ui.ctx().clone();
    let response = ui.interact(
        rect,
        Id::new(("aster_pm_btn", salt)),
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
    if primary {
        let fill = if enabled {
            mix(PURPLE, PURPLE_DEEP, t)
        } else {
            Color32::from_rgb(208, 201, 222)
        };
        ui.painter().rect_filled(r, 10.0, fill);
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
            10.0,
            mix(Color32::WHITE, Color32::from_rgb(243, 240, 249), t),
        );
        ui.painter().rect_stroke(
            r,
            10.0,
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
