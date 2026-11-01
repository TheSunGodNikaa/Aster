use crate::app::AsterApp;
use crate::utils::currency::format_inr;
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Layout, Margin, Pos2, Rect,
    Response, RichText, ScrollArea, Sense, Shape, Stroke, TextStyle, Ui, Vec2,
};
use std::f32::consts::{PI, TAU};

// ───────────────────────── Palette ─────────────────────────
const TEXT: Color32 = Color32::from_rgb(28, 25, 38);
const MUTED: Color32 = Color32::from_rgb(92, 86, 108);
const PURPLE: Color32 = Color32::from_rgb(99, 68, 151);
const PURPLE_BG: Color32 = Color32::from_rgb(240, 235, 249);
const CARD: Color32 = Color32::WHITE;
const BORDER: Color32 = Color32::from_rgb(219, 214, 229);
const ROW_ALT: Color32 = Color32::from_rgb(249, 247, 253);
const HEADER_BG: Color32 = Color32::from_rgb(242, 239, 249);
const GREEN: Color32 = Color32::from_rgb(28, 124, 82);
const GREEN_BG: Color32 = Color32::from_rgb(225, 244, 234);
const AMBER: Color32 = Color32::from_rgb(190, 104, 8);
const AMBER_BG: Color32 = Color32::from_rgb(253, 240, 221);
const YELLOW: Color32 = Color32::from_rgb(170, 130, 10);
const YELLOW_BG: Color32 = Color32::from_rgb(251, 244, 211);
const RED: Color32 = Color32::from_rgb(190, 50, 50);
const RED_BG: Color32 = Color32::from_rgb(250, 229, 229);
const GRAY: Color32 = Color32::from_rgb(120, 114, 136);
const GRAY_BG: Color32 = Color32::from_rgb(236, 234, 241);
const SOFT_GREEN: Color32 = Color32::from_rgb(98, 166, 108);

const STATE_ID: &str = "aster-stock-screen-state-v2";
const EXPIRY_WINDOW_DAYS: i64 = 90;

// ───────────────────────── Data ─────────────────────────
#[derive(Clone, Default)]
struct Batch {
    batch: String,
    expiry: String,
    qty: f64,
    free_qty: f64,
    mrp: i64,
    sale_rate: i64,
    cost: i64,
}

impl Batch {
    fn has_stock(&self) -> bool {
        self.qty + self.free_qty > 0.0
    }

    fn days_left(&self, today: i64) -> Option<i64> {
        parse_date(&self.expiry).map(|d| d - today)
    }

    fn value(&self) -> i64 {
        (self.qty * self.cost as f64).round() as i64
    }
}

#[derive(Clone, Default)]
struct ProductLite {
    id: i64,
    name: String,
    brand: String,
    unit: String,
    reorder: f64,
}

#[derive(Clone, Default)]
struct StockRow {
    product: ProductLite,
    qty: f64,
    free_qty: f64,
    stock_value: i64,
    batches: Vec<Batch>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Health {
    Healthy,
    Low,
    Expiring,
    Expired,
    OutOfStock,
}

impl Health {
    fn label(self) -> &'static str {
        match self {
            Health::Healthy => "Healthy",
            Health::Low => "Low stock",
            Health::Expiring => "Expiring",
            Health::Expired => "Expired",
            Health::OutOfStock => "Out of stock",
        }
    }

    fn colors(self) -> (Color32, Color32) {
        match self {
            Health::Healthy => (GREEN, GREEN_BG),
            Health::Low => (AMBER, AMBER_BG),
            Health::Expiring => (YELLOW, YELLOW_BG),
            Health::Expired => (RED, RED_BG),
            Health::OutOfStock => (GRAY, GRAY_BG),
        }
    }
}

impl StockRow {
    fn total_qty(&self) -> f64 {
        self.qty + self.free_qty
    }

    fn is_low(&self) -> bool {
        self.product.reorder > 0.0 && self.qty <= self.product.reorder
    }

    fn has_expired(&self, today: i64) -> bool {
        self.batches
            .iter()
            .any(|b| b.has_stock() && b.days_left(today).is_some_and(|d| d < 0))
    }

    fn has_expiring(&self, today: i64) -> bool {
        self.batches.iter().any(|b| {
            b.has_stock()
                && b.days_left(today)
                    .is_some_and(|d| (0..=EXPIRY_WINDOW_DAYS).contains(&d))
        })
    }

    fn next_expiry(&self, today: i64) -> Option<(&str, i64)> {
        self.batches
            .iter()
            .filter(|b| b.has_stock())
            .filter_map(|b| b.days_left(today).map(|d| (b.expiry.as_str(), d)))
            .min_by_key(|(_, d)| *d)
    }

    fn health(&self, today: i64) -> Health {
        if self.total_qty() <= 0.0 {
            Health::OutOfStock
        } else if self.has_expired(today) {
            Health::Expired
        } else if self.has_expiring(today) {
            Health::Expiring
        } else if self.is_low() {
            Health::Low
        } else {
            Health::Healthy
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum StockFilter {
    #[default]
    All,
    Healthy,
    Low,
    Expiring,
    Expired,
    OutOfStock,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum SortBy {
    #[default]
    Name,
    Value,
    QtyLow,
    Expiry,
}

fn sort_label(sort: SortBy) -> &'static str {
    match sort {
        SortBy::Name => "Name  (A–Z)",
        SortBy::Value => "Stock value  (high to low)",
        SortBy::QtyLow => "Quantity  (lowest first)",
        SortBy::Expiry => "Expiry  (soonest first)",
    }
}

fn row_matches(row: &StockRow, filter: StockFilter, today: i64) -> bool {
    match filter {
        StockFilter::All => true,
        StockFilter::Healthy => row.health(today) == Health::Healthy,
        StockFilter::Low => row.is_low(),
        StockFilter::Expiring => row.has_expiring(today),
        StockFilter::Expired => row.has_expired(today),
        StockFilter::OutOfStock => row.total_qty() <= 0.0,
    }
}

#[derive(Clone, Default)]
struct State {
    year: i32,
    search: String,
    loaded_search: String,
    rows: Vec<StockRow>,
    selected: Option<i64>,
    error: String,
    loaded: bool,
    filter: StockFilter,
    sort: SortBy,
}

// ───────────────────────── Entry point ─────────────────────────
pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let state_id = Id::new(STATE_ID);

    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<State>(state_id).unwrap_or_default());

    if !state.loaded || state.year != app.selected_year || state.loaded_search != state.search {
        state.year = app.selected_year;
        state.loaded_search = state.search.clone();
        state.loaded = true;
        reload(app, &mut state);
    }

    let today = today_days();

    ScrollArea::vertical()
        .id_salt("aster_stock_full_page")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            apply_style(ui);
            ui.add_space(6.0);

            draw_header(ui, app, &mut state);
            ui.add_space(18.0);

            if !state.error.is_empty() {
                egui::Frame::new()
                    .fill(RED_BG)
                    .stroke(Stroke::new(1.0_f32, RED))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.label(RichText::new(&state.error).size(13.5).strong().color(RED));
                    });
                ui.add_space(12.0);
            }

            draw_alert(ui, &mut state, today);
            draw_metrics(ui, &state, today);
            ui.add_space(14.0);

            ui.columns(2, |cols| {
                health_card(&mut cols[0], &mut state, today);
                top_value_card(&mut cols[1], &mut state);
            });

            ui.add_space(14.0);
            expiry_card(ui, &state, today);

            ui.add_space(14.0);
            draw_workspace(ui, &mut state, today);

            ui.add_space(22.0);
        });

    ui.ctx().data_mut(|d| d.insert_temp(state_id, state));
}

fn reload(app: &AsterApp, state: &mut State) {
    state.rows.clear();
    state.error.clear();

    let Some(db) = app.db.as_ref() else {
        state.error = "Database is not available.".into();
        return;
    };

    let records = match db.stock_batches(app.selected_year, &state.search) {
        Ok(records) => records,
        Err(error) => {
            state.error = error;
            return;
        }
    };

    for record in records {
        let product_id = record.product_id;

        let index = match state
            .rows
            .iter()
            .position(|row| row.product.id == product_id)
        {
            Some(index) => index,
            None => {
                state.rows.push(StockRow {
                    product: ProductLite {
                        id: record.product_id,
                        name: record.product_name.clone(),
                        brand: record.brand_name.clone(),
                        unit: record.unit_name.clone(),
                        reorder: record.reorder_level,
                    },
                    ..Default::default()
                });

                state.rows.len() - 1
            }
        };

        let row = &mut state.rows[index];

        if record.batch_id.is_some() {
            let qty = record.quantity_milli as f64 / 1000.0;
            let free_qty = record.free_quantity_milli as f64 / 1000.0;

            row.qty += qty;
            row.free_qty += free_qty;

            row.stock_value += (qty * record.purchase_rate_paise as f64).round() as i64;

            row.batches.push(Batch {
                batch: record.batch_no.clone(),
                expiry: record.expiry_date.clone(),
                qty,
                free_qty,
                mrp: record.mrp_paise,
                sale_rate: record.sale_rate_paise,
                cost: record.purchase_rate_paise as i64,
            });
        }
    }
}

// ───────────────────────── Page sections ─────────────────────────
fn draw_header(ui: &mut Ui, app: &AsterApp, state: &mut State) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("INVENTORY CONTROL")
                    .size(11.0)
                    .strong()
                    .color(PURPLE),
            );
            ui.add_space(2.0);
            ui.label(RichText::new("Stock Overview").size(28.0).strong().color(TEXT));
            ui.add_space(2.0);
            ui.label(
                RichText::new("On-hand quantities, batch expiry and valuation")
                    .size(13.0)
                    .color(MUTED),
            );
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("↻  Refresh").size(13.5).strong())
                        .min_size(Vec2::new(0.0, 34.0)),
                )
                .clicked()
            {
                state.year = app.selected_year;
                state.loaded_search = state.search.clone();
                state.loaded = true;
                reload(app, state);
            }
            egui::Frame::new()
                .fill(PURPLE_BG)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(12, 7))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("FY {}–{}", state.year, state.year + 1))
                            .size(13.0)
                            .strong()
                            .color(PURPLE),
                    );
                });
        });
    });
}

fn draw_alert(ui: &mut Ui, state: &mut State, today: i64) {
    let (batches, value) = expired_exposure(&state.rows, today);
    if batches == 0 {
        return;
    }

    egui::Frame::new()
        .fill(RED_BG)
        .stroke(Stroke::new(1.0_f32, RED))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "⚠  {batches} expired batch{} still on the shelf  ·  {} at cost",
                        if batches == 1 { "" } else { "es" },
                        money(value)
                    ))
                    .size(14.0)
                    .strong()
                    .color(RED),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Show expired products").clicked() {
                        state.filter = StockFilter::Expired;
                    }
                });
            });
        });
    ui.add_space(12.0);
}

fn draw_metrics(ui: &mut Ui, state: &State, today: i64) {
    let total_products = state.rows.len();
    let in_stock = state.rows.iter().filter(|r| r.total_qty() > 0.0).count();
    let total_qty: f64 = state.rows.iter().map(|r| r.qty).sum();
    let total_value: i64 = state.rows.iter().map(|r| r.stock_value).sum();
    let low = state.rows.iter().filter(|r| r.is_low()).count();
    let out = state.rows.iter().filter(|r| r.total_qty() <= 0.0).count();
    let (expired_batches, expired_value) = expired_exposure(&state.rows, today);
    let expiring_batches: usize = state
        .rows
        .iter()
        .flat_map(|r| r.batches.iter())
        .filter(|b| {
            b.has_stock()
                && b.days_left(today)
                    .is_some_and(|d| (0..=EXPIRY_WINDOW_DAYS).contains(&d))
        })
        .count();

    ui.columns(4, |cols| {
        metric(
            &mut cols[0],
            "PRODUCTS",
            &total_products.to_string(),
            &format!("{in_stock} in stock · {out} out of stock"),
            PURPLE,
        );
        metric(
            &mut cols[1],
            "STOCK VALUATION",
            &money(total_value),
            &format!("{} units on hand", format_qty(total_qty)),
            PURPLE,
        );
        metric(
            &mut cols[2],
            "LOW STOCK",
            &low.to_string(),
            "At or below reorder level",
            if low > 0 { AMBER } else { GREEN },
        );
        metric(
            &mut cols[3],
            "EXPIRY ALERTS",
            &(expired_batches + expiring_batches).to_string(),
            &format!(
                "{expired_batches} expired · {expiring_batches} within {EXPIRY_WINDOW_DAYS} days{}",
                if expired_batches > 0 {
                    format!(" · {} at risk", money(expired_value))
                } else {
                    String::new()
                }
            ),
            if expired_batches > 0 {
                RED
            } else if expiring_batches > 0 {
                YELLOW
            } else {
                GREEN
            },
        );
    });
}

fn expired_exposure(rows: &[StockRow], today: i64) -> (usize, i64) {
    let mut count = 0;
    let mut value = 0;
    for batch in rows.iter().flat_map(|r| r.batches.iter()) {
        if batch.has_stock() && batch.days_left(today).is_some_and(|d| d < 0) {
            count += 1;
            value += batch.value();
        }
    }
    (count, value)
}

struct Seg {
    label: &'static str,
    count: usize,
    color: Color32,
    filter: StockFilter,
}

fn health_card(ui: &mut Ui, state: &mut State, today: i64) {
    card(ui, |ui| {
        ui.set_min_height(250.0);
        section_title(ui, "Stock health", "Products by condition · click to filter");
        ui.add_space(12.0);

        let mut counts = [0_usize; 5];
        for row in &state.rows {
            let idx = match row.health(today) {
                Health::Healthy => 0,
                Health::Low => 1,
                Health::Expiring => 2,
                Health::Expired => 3,
                Health::OutOfStock => 4,
            };
            counts[idx] += 1;
        }

        let segs = [
            Seg { label: "Healthy", count: counts[0], color: GREEN, filter: StockFilter::Healthy },
            Seg { label: "Low stock", count: counts[1], color: AMBER, filter: StockFilter::Low },
            Seg { label: "Expiring soon", count: counts[2], color: YELLOW, filter: StockFilter::Expiring },
            Seg { label: "Expired", count: counts[3], color: RED, filter: StockFilter::Expired },
            Seg { label: "Out of stock", count: counts[4], color: GRAY, filter: StockFilter::OutOfStock },
        ];
        let total: usize = counts.iter().sum();

        ui.horizontal(|ui| {
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(172.0), Sense::hover());
            donut(ui, rect, &resp, &segs, &total.to_string(), "products");

            ui.add_space(10.0);
            ui.vertical(|ui| {
                for seg in &segs {
                    ui.horizontal(|ui| {
                        dot(ui, seg.color);
                        let r = ui.add(
                            egui::Button::new(RichText::new(seg.label).size(14.0).color(TEXT))
                                .frame(false),
                        );
                        if r.on_hover_text("Filter the stock table").clicked() {
                            state.filter = seg.filter;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(seg.count.to_string())
                                    .size(14.0)
                                    .strong()
                                    .color(TEXT),
                            );
                        });
                    });
                }
            });
        });
    });
}

fn top_value_card(ui: &mut Ui, state: &mut State) {
    card(ui, |ui| {
        ui.set_min_height(250.0);
        section_title(
            ui,
            "Highest stock value",
            "Top six products at cost · click to open",
        );
        ui.add_space(12.0);

        let mut rows: Vec<(i64, String, i64)> = state
            .rows
            .iter()
            .filter(|r| r.stock_value > 0)
            .map(|r| (r.product.id, r.product.name.clone(), r.stock_value))
            .collect();
        rows.sort_by(|a, b| b.2.cmp(&a.2));
        rows.truncate(6);

        if rows.is_empty() {
            ui.add_space(40.0);
            empty(ui, "No stock value yet", "Received batches will appear here.");
            return;
        }

        let max = rows.iter().map(|r| r.2).max().unwrap_or(1).max(1);
        let total: i64 = state.rows.iter().map(|r| r.stock_value).sum();

        for (id, name, value) in &rows {
            let selected = state.selected == Some(*id);
            ui.horizontal(|ui| {
                let text = RichText::new(truncate(name, 26))
                    .size(14.0)
                    .color(if selected { PURPLE } else { TEXT });
                let text = if selected { text.strong() } else { text };
                let r = ui.add(egui::Button::new(text).frame(false));
                if r.on_hover_text(name.as_str()).clicked() {
                    state.selected = if selected { None } else { Some(*id) };
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{}  ·  {:.0}%",
                            money(*value),
                            *value as f64 / total.max(1) as f64 * 100.0
                        ))
                        .size(13.5)
                        .strong()
                        .color(TEXT),
                    );
                });
            });
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 8.0), Sense::hover());
            ui.painter().rect_filled(rect, CornerRadius::same(4), PURPLE_BG);
            ui.painter().rect_filled(
                Rect::from_min_size(
                    rect.min,
                    Vec2::new(rect.width() * (*value as f32 / max as f32), rect.height()),
                ),
                CornerRadius::same(4),
                if selected { PURPLE } else { Color32::from_rgb(140, 110, 190) },
            );
            ui.add_space(5.0);
        }
    });
}

fn expiry_card(ui: &mut Ui, state: &State, today: i64) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                section_title(
                    ui,
                    "Expiry exposure",
                    "Stock value at cost, grouped by time left to expiry",
                );
            });
        });
        ui.add_space(12.0);

        const LABELS: [&str; 7] = [
            "Expired",
            "0–30 days",
            "31–60 days",
            "61–90 days",
            "91–180 days",
            "180+ days",
            "No expiry",
        ];
        let colors = [RED, AMBER, YELLOW, Color32::from_rgb(120, 150, 60), SOFT_GREEN, GREEN, GRAY];

        let mut value = [0_i64; 7];
        let mut units = [0.0_f64; 7];
        let mut batches = [0_usize; 7];
        for batch in state.rows.iter().flat_map(|r| r.batches.iter()) {
            if !batch.has_stock() {
                continue;
            }
            let idx = expiry_bucket(batch.days_left(today));
            value[idx] += batch.value();
            units[idx] += batch.qty + batch.free_qty;
            batches[idx] += 1;
        }

        if batches.iter().sum::<usize>() == 0 {
            ui.add_space(30.0);
            empty(ui, "No batches on hand", "Expiry exposure will appear once stock is received.");
            return;
        }

        let max = value.iter().copied().max().unwrap_or(0);
        let top = nice_max(max);

        let (rect, resp) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 230.0), Sense::hover());
        let painter = ui.painter_at(rect.expand(2.0));
        let plot = Rect::from_min_max(
            Pos2::new(rect.left() + 62.0, rect.top() + 22.0),
            Pos2::new(rect.right() - 4.0, rect.bottom() - 28.0),
        );
        let group_w = plot.width() / 7.0;

        let hovered = resp.hover_pos().and_then(|p| {
            if plot.expand2(Vec2::new(0.0, 14.0)).contains(p) {
                Some((((p.x - plot.left()) / group_w) as usize).min(6))
            } else {
                None
            }
        });

        for i in 0..=4 {
            let y = plot.bottom() - plot.height() * i as f32 / 4.0;
            painter.line_segment(
                [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
                Stroke::new(1.0_f32, if i == 0 { MUTED } else { BORDER }),
            );
            painter.text(
                Pos2::new(plot.left() - 8.0, y),
                Align2::RIGHT_CENTER,
                compact(top / 4 * i),
                FontId::proportional(12.0),
                MUTED,
            );
        }

        let bar_w = (group_w * 0.5).clamp(16.0, 70.0);
        for i in 0..7 {
            let cx = plot.left() + group_w * (i as f32 + 0.5);

            if hovered == Some(i) {
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(cx - group_w / 2.0 + 3.0, plot.top() - 8.0),
                        Pos2::new(cx + group_w / 2.0 - 3.0, plot.bottom()),
                    ),
                    CornerRadius::same(6),
                    PURPLE_BG,
                );
            }

            if value[i] > 0 {
                let h = (plot.height() * value[i] as f32 / top as f32).max(3.0);
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(cx - bar_w / 2.0, plot.bottom() - h),
                        Pos2::new(cx + bar_w / 2.0, plot.bottom()),
                    ),
                    CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 },
                    colors[i],
                );
                painter.text(
                    Pos2::new(cx, plot.bottom() - h - 4.0),
                    Align2::CENTER_BOTTOM,
                    compact(value[i]),
                    FontId::proportional(12.0),
                    TEXT,
                );
            }

            painter.text(
                Pos2::new(cx, plot.bottom() + 8.0),
                Align2::CENTER_TOP,
                LABELS[i],
                FontId::proportional(12.0),
                if hovered == Some(i) { TEXT } else { MUTED },
            );
        }

        if let Some(i) = hovered {
            resp.on_hover_ui_at_pointer(|ui| {
                ui.label(RichText::new(LABELS[i]).size(14.0).strong().color(colors[i]));
                ui.label(RichText::new(format!("Batches: {}", batches[i])).size(13.0));
                ui.label(RichText::new(format!("Units: {}", format_qty(units[i]))).size(13.0));
                ui.label(
                    RichText::new(format!("Value at cost: {}", money(value[i])))
                        .size(13.0)
                        .strong(),
                );
            });
        }
    });
}

fn expiry_bucket(days: Option<i64>) -> usize {
    match days {
        None => 6,
        Some(d) if d < 0 => 0,
        Some(d) if d <= 30 => 1,
        Some(d) if d <= 60 => 2,
        Some(d) if d <= 90 => 3,
        Some(d) if d <= 180 => 4,
        Some(_) => 5,
    }
}

fn draw_workspace(ui: &mut Ui, state: &mut State, today: i64) {
    let available = ui.available_width();
    let gap = 14.0;
    let detail_open = state.selected.is_some();
    let detail_w = if detail_open {
        (available * 0.34).clamp(340.0, 440.0)
    } else {
        0.0
    };
    let table_w = (available - detail_w - if detail_open { gap } else { 0.0 }).max(420.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(table_w, 0.0),
            Layout::top_down(Align::Min),
            |ui| draw_register(ui, state, today),
        );

        if detail_open {
            ui.add_space(gap);
            ui.allocate_ui_with_layout(
                Vec2::new(detail_w, 0.0),
                Layout::top_down(Align::Min),
                |ui| draw_detail(ui, state, today),
            );
        }
    });
}

fn draw_register(ui: &mut Ui, state: &mut State, today: i64) {
    card(ui, |ui| {
        section_title(
            ui,
            "Stock register",
            "Search, filter and sort the catalogue. Click a row for the batch report.",
        );
        ui.add_space(14.0);

        // Toolbar
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(caps("SEARCH"));
                ui.add_sized(
                    [320.0, 34.0],
                    egui::TextEdit::singleline(&mut state.search)
                        .hint_text("Medicine, brand, batch or barcode"),
                );
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(caps("SORT BY"));
                egui::ComboBox::from_id_salt("stock_sort")
                    .width(230.0)
                    .selected_text(RichText::new(sort_label(state.sort)).size(14.0))
                    .show_ui(ui, |ui| {
                        for s in [SortBy::Name, SortBy::Value, SortBy::QtyLow, SortBy::Expiry] {
                            ui.selectable_value(&mut state.sort, s, sort_label(s));
                        }
                    });
            });
            if !state.search.is_empty() || state.filter != StockFilter::All {
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(caps(" "));
                    if ui.button("Clear filters").clicked() {
                        state.search.clear();
                        state.filter = StockFilter::All;
                    }
                });
            }
        });

        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            for (label, filter) in [
                ("All", StockFilter::All),
                ("Healthy", StockFilter::Healthy),
                ("Low stock", StockFilter::Low),
                ("Expiring in 90 days", StockFilter::Expiring),
                ("Expired", StockFilter::Expired),
                ("Out of stock", StockFilter::OutOfStock),
            ] {
                let n = state
                    .rows
                    .iter()
                    .filter(|r| row_matches(r, filter, today))
                    .count();
                if chip(ui, &format!("{label}  {n}"), state.filter == filter) {
                    state.filter = filter;
                }
            }
        });

        ui.add_space(14.0);

        // Visible rows
        let mut view: Vec<&StockRow> = state
            .rows
            .iter()
            .filter(|r| row_matches(r, state.filter, today))
            .collect();
        match state.sort {
            SortBy::Name => view.sort_by_key(|r| r.product.name.to_lowercase()),
            SortBy::Value => view.sort_by(|a, b| b.stock_value.cmp(&a.stock_value)),
            SortBy::QtyLow => view.sort_by(|a, b| {
                a.total_qty()
                    .partial_cmp(&b.total_qty())
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortBy::Expiry => view.sort_by_key(|r| r.next_expiry(today).map_or(i64::MAX, |(_, d)| d)),
        }

        if view.is_empty() {
            empty(
                ui,
                "No matching stock items",
                "Try another search, filter or sort.",
            );
            return;
        }

        let inner = (ui.available_width() - 24.0).max(560.0);
        let w = [
            inner * 0.27, // medicine
            inner * 0.13, // status
            inner * 0.12, // on hand
            inner * 0.08, // batches
            inner * 0.20, // next expiry
            inner * 0.20, // value
        ];

        egui::Frame::new()
            .fill(HEADER_BG)
            .corner_radius(CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 })
            .inner_margin(Margin::symmetric(12, 4))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (i, (label, align)) in [
                        ("MEDICINE / BRAND", Align::Min),
                        ("STATUS", Align::Min),
                        ("ON HAND", Align::Max),
                        ("BATCHES", Align::Max),
                        ("NEXT EXPIRY", Align::Max),
                        ("VALUE AT COST", Align::Max),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        cell(ui, w[i], 30.0, align, |ui| {
                            ui.label(caps(label));
                        });
                    }
                });
            });

        let mut clicked: Option<i64> = None;

        egui::Frame::new()
            .stroke(Stroke::new(1.0_f32, BORDER))
            .corner_radius(CornerRadius { nw: 0, ne: 0, sw: 8, se: 8 })
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ScrollArea::vertical()
                    .id_salt("stock_table_rows")
                    .max_height(560.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;

                        for (idx, row) in view.iter().enumerate() {
                            let selected = state.selected == Some(row.product.id);
                            let bg = if selected {
                                PURPLE_BG
                            } else if idx % 2 == 0 {
                                CARD
                            } else {
                                ROW_ALT
                            };

                            let frame = egui::Frame::new()
                                .fill(bg)
                                .inner_margin(Margin::symmetric(12, 4))
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 0.0;
                                        stock_row(ui, row, today, &w);
                                    });
                                });

                            let resp = frame
                                .response
                                .interact(Sense::click())
                                .on_hover_cursor(CursorIcon::PointingHand);
                            if resp.hovered() && !selected {
                                ui.painter().rect_filled(
                                    resp.rect,
                                    CornerRadius::ZERO,
                                    Color32::from_black_alpha(6),
                                );
                            }
                            if resp.clicked() {
                                clicked = Some(row.product.id);
                            }
                        }
                    });
            });

        let shown_value: i64 = view.iter().map(|r| r.stock_value).sum();
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "Showing {} of {} products",
                    view.len(),
                    state.rows.len()
                ))
                .size(12.5)
                .color(MUTED),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("Value in view: {}", money(shown_value)))
                        .size(13.0)
                        .strong()
                        .color(TEXT),
                );
            });
        });

        if let Some(id) = clicked {
            state.selected = if state.selected == Some(id) { None } else { Some(id) };
        }
    });
}

fn stock_row(ui: &mut Ui, row: &StockRow, today: i64, w: &[f32; 6]) {
    let health = row.health(today);
    let (fg, bg) = health.colors();

    cell(ui, w[0], 54.0, Align::Min, |ui| {
        ui.add(
            egui::Label::new(
                RichText::new(&row.product.name).size(14.0).strong().color(TEXT),
            )
            .truncate(),
        );
        ui.add(
            egui::Label::new(
                RichText::new(if row.product.brand.is_empty() {
                    "—"
                } else {
                    &row.product.brand
                })
                .size(12.5)
                .color(MUTED),
            )
            .truncate(),
        );
    });

    cell(ui, w[1], 54.0, Align::Min, |ui| {
        badge(ui, health.label(), fg, bg);
    });

    cell(ui, w[2], 54.0, Align::Max, |ui| {
        ui.label(
            RichText::new(format!(
                "{} {}",
                format_qty(row.qty),
                row.product.unit
            ))
            .size(14.0)
            .strong()
            .color(if row.is_low() { AMBER } else { TEXT }),
        );
        if row.product.reorder > 0.0 {
            ui.label(
                RichText::new(format!("reorder at {}", format_qty(row.product.reorder)))
                    .size(11.5)
                    .color(MUTED),
            );
        }
    });

    cell(ui, w[3], 54.0, Align::Max, |ui| {
        ui.label(RichText::new(row.batches.len().to_string()).size(14.0).color(TEXT));
    });

    cell(ui, w[4], 54.0, Align::Max, |ui| match row.next_expiry(today) {
        Some((date, days)) => {
            let (color, _) = expiry_tone(days);
            ui.label(RichText::new(date).size(13.5).color(TEXT));
            ui.label(RichText::new(expiry_text(days)).size(12.0).strong().color(color));
        }
        None => {
            ui.label(RichText::new("—").size(14.0).color(MUTED));
        }
    });

    cell(ui, w[5], 54.0, Align::Max, |ui| {
        ui.label(RichText::new(money(row.stock_value)).size(14.0).strong().color(TEXT));
    });
}

fn draw_detail(ui: &mut Ui, state: &mut State, today: i64) {
    let Some(id) = state.selected else {
        return;
    };
    let Some(row) = state.rows.iter().find(|r| r.product.id == id) else {
        return;
    };

    let mut close = false;
    let health = row.health(today);
    let (fg, bg) = health.colors();

    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(caps("PRODUCT REPORT"));
                ui.label(RichText::new(&row.product.name).size(20.0).strong().color(TEXT));
                if !row.product.brand.is_empty() {
                    ui.label(RichText::new(&row.product.brand).size(13.5).color(MUTED));
                }
                ui.add_space(2.0);
                badge(ui, health.label(), fg, bg);
            });
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if ui
                    .button(RichText::new("✕").strong())
                    .on_hover_text("Close report")
                    .clicked()
                {
                    close = true;
                }
            });
        });

        ui.add_space(14.0);
        ui.columns(2, |cols| {
            mini_stat(&mut cols[0], "ON HAND", &format!("{} {}", format_qty(row.qty), row.product.unit), TEXT);
            mini_stat(&mut cols[1], "FREE QTY", &format_qty(row.free_qty), TEXT);
        });
        ui.add_space(4.0);
        ui.columns(2, |cols| {
            mini_stat(&mut cols[0], "BATCHES", &row.batches.len().to_string(), TEXT);
            mini_stat(&mut cols[1], "STOCK VALUE", &money(row.stock_value), PURPLE);
        });

        // Stock level vs reorder
        if row.product.reorder > 0.0 {
            ui.add_space(12.0);
            ui.label(caps("STOCK LEVEL VS REORDER POINT"));
            let cap = (row.product.reorder * 2.0).max(row.qty).max(1.0);
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 12.0), Sense::hover());
            ui.painter().rect_filled(rect, CornerRadius::same(6), PURPLE_BG);
            ui.painter().rect_filled(
                Rect::from_min_size(
                    rect.min,
                    Vec2::new(rect.width() * (row.qty / cap) as f32, rect.height()),
                ),
                CornerRadius::same(6),
                if row.is_low() { AMBER } else { GREEN },
            );
            let marker_x = rect.left() + rect.width() * (row.product.reorder / cap) as f32;
            ui.painter().line_segment(
                [Pos2::new(marker_x, rect.top() - 3.0), Pos2::new(marker_x, rect.bottom() + 3.0)],
                Stroke::new(2.0_f32, TEXT),
            );
            ui.label(
                RichText::new(format!(
                    "{} on hand · reorder at {}",
                    format_qty(row.qty),
                    format_qty(row.product.reorder)
                ))
                .size(12.5)
                .color(if row.is_low() { AMBER } else { MUTED }),
            );
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(10.0);
        ui.label(caps("BATCH BREAKDOWN  ·  SOONEST EXPIRY FIRST"));
        ui.add_space(6.0);

        if row.batches.is_empty() {
            empty(ui, "No batches recorded", "Receive stock to create batches.");
            return;
        }

        let mut batches: Vec<&Batch> = row.batches.iter().collect();
        batches.sort_by_key(|b| b.days_left(today).unwrap_or(i64::MAX));
        let max_qty = batches
            .iter()
            .map(|b| b.qty + b.free_qty)
            .fold(0.0_f64, f64::max)
            .max(1.0);

        ScrollArea::vertical()
            .id_salt("stock_detail_batches")
            .max_height(420.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for batch in batches {
                    let days = batch.days_left(today);
                    let (tone, tone_bg) = match days {
                        Some(d) => expiry_tone(d),
                        None => (GRAY, GRAY_BG),
                    };

                    egui::Frame::new()
                        .fill(ROW_ALT)
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(Margin::same(12))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("Batch {}", batch.batch))
                                        .size(14.5)
                                        .strong()
                                        .color(TEXT),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let text = match days {
                                        Some(d) => format!("{} · {}", batch.expiry, expiry_text(d)),
                                        None => "No expiry".to_owned(),
                                    };
                                    badge(ui, &text, tone, tone_bg);
                                });
                            });

                            ui.add_space(4.0);
                            let total = batch.qty + batch.free_qty;
                            ui.label(
                                RichText::new(format!(
                                    "Qty {}  +  {} free",
                                    format_qty(batch.qty),
                                    format_qty(batch.free_qty)
                                ))
                                .size(13.0)
                                .color(TEXT),
                            );
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 8.0),
                                Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, CornerRadius::same(4), PURPLE_BG);
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    rect.min,
                                    Vec2::new(
                                        rect.width() * (total / max_qty) as f32,
                                        rect.height(),
                                    ),
                                ),
                                CornerRadius::same(4),
                                tone,
                            );

                            ui.add_space(4.0);
                            ui.columns(3, |cols| {
                                cols[0].label(
                                    RichText::new(format!("MRP  {}", money(batch.mrp)))
                                        .size(12.5)
                                        .color(MUTED),
                                );
                                cols[1].label(
                                    RichText::new(format!("Sale  {}", money(batch.sale_rate)))
                                        .size(12.5)
                                        .color(MUTED),
                                );
                                cols[2].label(
                                    RichText::new(format!("Cost  {}", money(batch.cost)))
                                        .size(12.5)
                                        .color(MUTED),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("Value  {}", money(batch.value())))
                                        .size(13.0)
                                        .strong()
                                        .color(TEXT),
                                );
                                if batch.sale_rate > 0 && batch.cost > 0 {
                                    let margin = (batch.sale_rate - batch.cost) as f64
                                        / batch.sale_rate as f64
                                        * 100.0;
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(format!("Margin {margin:.1}%"))
                                                .size(13.0)
                                                .strong()
                                                .color(if margin >= 0.0 { GREEN } else { RED }),
                                        );
                                    });
                                }
                            });
                        });
                }
            });
    });

    if close {
        state.selected = None;
    }
}

// ───────────────────────── Charts ─────────────────────────
fn arc_points(center: Pos2, radius: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    let steps = (((a1 - a0).abs() * radius) / 3.0).ceil().max(2.0) as usize;
    (0..=steps)
        .map(|k| {
            let a = a0 + (a1 - a0) * k as f32 / steps as f32;
            center + Vec2::new(a.cos(), a.sin()) * radius
        })
        .collect()
}

fn donut(ui: &mut Ui, rect: Rect, resp: &Response, segs: &[Seg], center_top: &str, center_bottom: &str) {
    let painter = ui.painter_at(rect.expand(6.0));
    let center = rect.center();
    let thickness = 24.0_f32;
    let radius = rect.width() / 2.0 - thickness / 2.0 - 4.0;
    let total: f32 = segs.iter().map(|s| s.count as f32).sum();

    if total <= 0.0 {
        painter.circle_stroke(center, radius, Stroke::new(thickness, PURPLE_BG));
    } else {
        let nonzero = segs.iter().filter(|s| s.count > 0).count();
        let gap = if nonzero > 1 { 0.06 } else { 0.0 };

        let hovered = resp.hover_pos().and_then(|p| {
            let d = p - center;
            if (d.length() - radius).abs() > thickness / 2.0 + 3.0 {
                return None;
            }
            let mut ang = d.y.atan2(d.x) + PI / 2.0;
            if ang < 0.0 {
                ang += TAU;
            }
            let mut acc = 0.0;
            for (i, s) in segs.iter().enumerate() {
                let sweep = s.count as f32 / total * TAU;
                if s.count > 0 && ang >= acc && ang < acc + sweep {
                    return Some(i);
                }
                acc += sweep;
            }
            None
        });

        let mut start = -PI / 2.0;
        for (i, seg) in segs.iter().enumerate() {
            if seg.count == 0 {
                continue;
            }
            let sweep = seg.count as f32 / total * TAU;
            let width = if hovered == Some(i) { thickness + 5.0 } else { thickness };
            if nonzero == 1 {
                painter.circle_stroke(center, radius, Stroke::new(width, seg.color));
            } else {
                let points = arc_points(center, radius, start + gap / 2.0, start + sweep - gap / 2.0);
                painter.add(Shape::line(points, Stroke::new(width, seg.color)));
            }
            start += sweep;
        }

        if let Some(i) = hovered {
            let seg = &segs[i];
            resp.clone().on_hover_text(format!(
                "{}: {} products ({:.0}%)",
                seg.label,
                seg.count,
                seg.count as f32 / total * 100.0
            ));
        }
    }

    painter.text(
        center - Vec2::new(0.0, 9.0),
        Align2::CENTER_CENTER,
        center_top,
        FontId::proportional(30.0),
        TEXT,
    );
    painter.text(
        center + Vec2::new(0.0, 17.0),
        Align2::CENTER_CENTER,
        center_bottom,
        FontId::proportional(13.0),
        MUTED,
    );
}

fn nice_max(max: i64) -> i64 {
    if max <= 0 {
        return 10_000;
    }
    let mag = 10f64.powf((max as f64).log10().floor());
    for m in [1.0, 2.0, 4.0, 5.0, 8.0, 10.0] {
        let v = m * mag;
        if v >= max as f64 {
            return v as i64;
        }
    }
    (10.0 * mag) as i64
}

// ───────────────────────── UI helpers ─────────────────────────
fn apply_style(ui: &mut Ui) {
    let style = ui.style_mut();
    style.spacing.item_spacing = Vec2::new(12.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.text_styles.insert(TextStyle::Body, FontId::proportional(14.0));
    style.text_styles.insert(TextStyle::Button, FontId::proportional(14.0));
    style.text_styles.insert(TextStyle::Small, FontId::proportional(12.0));
}

fn caps(text: &str) -> RichText {
    RichText::new(text).size(11.0).strong().color(MUTED)
}

fn card(ui: &mut Ui, content: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            content(ui);
        });
}

fn section_title(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(17.0).strong().color(TEXT));
    ui.label(RichText::new(subtitle).size(12.5).color(MUTED));
}

fn metric(ui: &mut Ui, label: &str, value: &str, hint: &str, accent: Color32) {
    card(ui, |ui| {
        ui.set_min_height(92.0);
        ui.horizontal(|ui| {
            dot(ui, accent);
            ui.label(caps(label));
        });
        ui.add_space(4.0);
        ui.label(RichText::new(value).size(24.0).strong().color(TEXT));
        ui.label(RichText::new(hint).size(12.5).color(MUTED));
    });
}

fn mini_stat(ui: &mut Ui, label: &str, value: &str, color: Color32) {
    egui::Frame::new()
        .fill(PURPLE_BG)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(label).size(10.5).strong().color(MUTED));
            ui.label(RichText::new(value).size(16.0).strong().color(color));
        });
}

fn dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 5.0, color);
}

fn badge(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(10, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(12.0).strong().color(fg));
        });
}

fn chip(ui: &mut Ui, label: &str, selected: bool) -> bool {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(13.0)
                .strong()
                .color(if selected { PURPLE } else { MUTED }),
        )
        .fill(if selected { PURPLE_BG } else { CARD })
        .stroke(Stroke::new(1.0_f32, if selected { PURPLE } else { BORDER }))
        .corner_radius(CornerRadius::same(16))
        .min_size(Vec2::new(0.0, 30.0)),
    )
    .clicked()
}

fn cell(ui: &mut Ui, width: f32, height: f32, align: Align, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, height),
        Layout::top_down(align).with_main_align(Align::Center),
        |ui| {
            ui.set_width(width);
            add(ui);
        },
    );
}

fn empty(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(14.0);
        ui.label(RichText::new(title).size(15.0).strong().color(TEXT));
        ui.add_space(4.0);
        ui.label(RichText::new(subtitle).size(13.0).color(MUTED));
        ui.add_space(14.0);
    });
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() > max {
        let cut: String = text.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    } else {
        text.to_owned()
    }
}

// ───────────────────────── Formatting & dates ─────────────────────────
fn money(paise: i64) -> String {
    format_inr(paise as f64 / 100.0)
}

fn trim_num(value: f64) -> String {
    let s = format!("{value:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn compact(paise: i64) -> String {
    let rupees = paise as f64 / 100.0;
    if rupees >= 1e7 {
        format!("₹{}Cr", trim_num(rupees / 1e7))
    } else if rupees >= 1e5 {
        format!("₹{}L", trim_num(rupees / 1e5))
    } else if rupees >= 1e3 {
        format!("₹{}K", trim_num(rupees / 1e3))
    } else {
        format!("₹{}", trim_num(rupees))
    }
}

fn format_qty(value: f64) -> String {
    format!("{value:.3}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

fn expiry_text(days: i64) -> String {
    match days {
        d if d < 0 => format!("expired {}d ago", -d),
        0 => "expires today".to_owned(),
        d => format!("in {d} days"),
    }
}

fn expiry_tone(days: i64) -> (Color32, Color32) {
    if days < 0 {
        (RED, RED_BG)
    } else if days <= 30 {
        (AMBER, AMBER_BG)
    } else if days <= EXPIRY_WINDOW_DAYS {
        (YELLOW, YELLOW_BG)
    } else {
        (GREEN, GREEN_BG)
    }
}

fn today_days() -> i64 {
    let (year, month, day) = crate::date::local_date();
    days_from_civil(year as i64, month as i64, day as i64)
}

fn parse_date(date: &str) -> Option<i64> {
    if date.len() < 10 {
        return None;
    }
    let year = date.get(0..4)?.parse::<i64>().ok()?;
    let month = date.get(5..7)?.parse::<i64>().ok()?;
    let day = date.get(8..10)?.parse::<i64>().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

/// Days since 1970-01-01 for a proleptic Gregorian date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}