use crate::app::AsterApp;
use eframe::egui::{
    self, Align, Color32, CornerRadius, Id, Layout, Margin, RichText,
    ScrollArea, Sense, Stroke, Vec2,
};

const INK: Color32 = Color32::from_rgb(43, 39, 52);
const MUTED: Color32 = Color32::from_rgb(111, 105, 122);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const BORDER: Color32 = Color32::from_rgb(226, 223, 232);
const CANVAS: Color32 = Color32::from_rgb(248, 247, 251);
const GREEN: Color32 = Color32::from_rgb(38, 126, 81);
const AMBER: Color32 = Color32::from_rgb(176, 116, 23);
const RED: Color32 = Color32::from_rgb(176, 58, 58);

const STATE_ID: &str = "aster-stock-screen-state";
const SELECTED_ID: &str = "stock-selected-product";

#[derive(Clone, Default)]
struct Batch {
    batch: String,
    expiry: String,
    qty: f64,
    free_qty: f64,
    mrp: i64,
    sale_rate: i64,
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

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum StockFilter {
    #[default]
    All,
    Low,
    Expiring,
    Expired,
    OutOfStock,
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
}

pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let state_id = Id::new(STATE_ID);
    let selected_id = Id::new(SELECTED_ID);

    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<State>(state_id).unwrap_or_default());

    let selected = ui.ctx().data(|d| {
        d.get_temp::<i64>(selected_id).unwrap_or(-1)
    });
    state.selected = (selected > 0).then_some(selected);

    if !state.loaded
        || state.year != app.selected_year
        || state.loaded_search != state.search
    {
        state.year = app.selected_year;
        state.loaded_search = state.search.clone();
        state.loaded = true;
        reload(app, &mut state);
    }

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("INVENTORY CONTROL")
                        .size(10.0)
                        .strong()
                        .color(PURPLE),
                );

                ui.heading(
                    RichText::new("Stock overview")
                        .size(25.0)
                        .strong()
                        .color(INK),
                );

                ui.label(
                    RichText::new(
                        "On-hand quantities, batch expiry and valuation",
                    )
                    .size(12.0)
                    .color(MUTED),
                );
            });

            ui.with_layout(
                Layout::right_to_left(Align::Center),
                |ui| {
                    if ui.button("↻ Refresh").clicked() {
                        reload(app, &mut state);
                        state.loaded_search = state.search.clone();
                        state.year = app.selected_year;
                        state.loaded = true;
                    }

                    ui.label(
                        RichText::new(format!(
                            "FY {}–{}",
                            state.year,
                            state.year + 1
                        ))
                        .size(12.0)
                        .color(MUTED),
                    );
                },
            );
        });

        ui.add_space(16.0);

        let total_products = state.rows.len();
        let total_qty: f64 = state.rows.iter().map(|r| r.qty).sum();
        let total_value: i64 =
            state.rows.iter().map(|r| r.stock_value).sum();

        let low_stock = state
            .rows
            .iter()
            .filter(|r| {
                r.product.reorder > 0.0
                    && r.qty <= r.product.reorder
            })
            .count();

        ui.columns(3, |cols| {
            metric(
                &mut cols[0],
                "PRODUCTS",
                &total_products.to_string(),
                "Matching catalogue items",
                PURPLE,
            );

            metric(
                &mut cols[1],
                "LOW STOCK",
                &low_stock.to_string(),
                "At or below reorder level",
                if low_stock > 0 { AMBER } else { GREEN },
            );

            metric(
                &mut cols[2],
                "STOCK VALUATION",
                &money(total_value),
                &format!("{} units on hand", format_qty(total_qty)),
                INK,
            );
        });

        ui.add_space(14.0);

        ui.horizontal(|ui| {
            let search_width = ui.available_width().min(360.0);

            ui.add_sized(
                [search_width, 36.0],
                egui::TextEdit::singleline(&mut state.search)
                    .hint_text("Search medicine, brand, batch or barcode…"),
            );

            ui.add_space(10.0);

            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("SHOW")
                        .size(10.0)
                        .strong()
                        .color(MUTED),
                );
                ui.add_space(4.0);

                for (filter, label) in [
                    (StockFilter::All, "All inventory"),
                    (StockFilter::Low, "Low stock"),
                    (StockFilter::Expiring, "Expiring in 90 days"),
                    (StockFilter::Expired, "Expired"),
                    (StockFilter::OutOfStock, "Out of stock"),
                ] {
                    if ui
                        .selectable_label(state.filter == filter, label)
                        .clicked()
                    {
                        state.filter = filter;
                    }
                }
            });

            if ui.button("Clear").clicked() {
                state.search.clear();
            }
        });

        if !state.error.is_empty() {
            ui.add_space(8.0);
            ui.colored_label(RED, &state.error);
        }

        ui.add_space(10.0);

        let detail_open = state.selected.is_some();

        let slide = ui.ctx().animate_bool_with_time(
            Id::new("stock-detail-open"),
            detail_open,
            0.22,
        );

        let available = ui.available_size();
        let desired_detail_width =
            (available.x * 0.36).clamp(300.0, 440.0);
        let detail_width = desired_detail_width * slide;

        ui.horizontal_top(|ui| {
            let total_width = ui.available_width();
            let gap = if detail_open { 12.0 } else { 0.0 };

            let table_width =
                (total_width - detail_width - gap).max(300.0);

            ui.allocate_ui_with_layout(
                Vec2::new(table_width, available.y),
                Layout::top_down(Align::Min),
                |ui| draw_table(ui, &state),
            );

            if detail_open && detail_width > 4.0 {
                ui.add_space(12.0);

                ui.allocate_ui_with_layout(
                    Vec2::new(detail_width, available.y),
                    Layout::top_down(Align::Min),
                    |ui| draw_detail(ui, &state),
                );
            }
        });
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

    let records = match db.stock_batches(
        app.selected_year,
        &state.search,
    ) {
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
            let free_qty =
                record.free_quantity_milli as f64 / 1000.0;

            row.qty += qty;
            row.free_qty += free_qty;

            row.stock_value +=
                (qty * record.purchase_rate_paise as f64).round()
                    as i64;

            row.batches.push(Batch {
                batch: record.batch_no.clone(),
                expiry: record.expiry_date.clone(),
                qty,
                free_qty,
                mrp: record.mrp_paise,
                sale_rate: record.sale_rate_paise,
            });
        }
    }
}

fn metric(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    foot: &str,
    accent: Color32,
) {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_height(82.0);

            ui.label(
                RichText::new(label)
                    .size(10.0)
                    .strong()
                    .color(MUTED),
            );

            ui.add_space(4.0);

            ui.label(
                RichText::new(value)
                    .size(21.0)
                    .strong()
                    .color(accent),
            );

            ui.label(
                RichText::new(foot)
                    .size(10.0)
                    .color(MUTED),
            );
        });
}

fn draw_table(ui: &mut egui::Ui, state: &State) {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            egui::Frame::new()
                .fill(CANVAS)
                .inner_margin(Margin::symmetric(12, 11))
                .show(ui, |ui| {
                    let widths = columns(ui.available_width());

                    ui.horizontal(|ui| {
                        for (label, width) in [
                            ("MEDICINE / BRAND", widths[0]),
                            ("ON HAND", widths[1]),
                            ("BATCHES", widths[2]),
                            ("VALUE", widths[3]),
                        ] {
                            ui.allocate_ui_with_layout(
                                Vec2::new(width, 18.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(label)
                                            .size(9.0)
                                            .strong()
                                            .color(MUTED),
                                    );
                                },
                            );
                        }
                    });
                });

            let (today, cutoff) = date_window();

            let visible_rows: Vec<&StockRow> = state
                .rows
                .iter()
                .filter(|row| {
                    match state.filter {
                        StockFilter::All => true,

                        StockFilter::Low => {
                            row.product.reorder > 0.0
                                && row.qty <= row.product.reorder
                        }

                        StockFilter::Expiring => {
                            row.batches.iter().any(|batch| {
                                let total = batch.qty + batch.free_qty;
                                total > 0.0
                                    && !batch.expiry.is_empty()
                                    && batch.expiry >= today
                                    && batch.expiry <= cutoff
                            })
                        }

                        StockFilter::Expired => {
                            row.batches.iter().any(|batch| {
                                batch.qty + batch.free_qty > 0.0
                                    && !batch.expiry.is_empty()
                                    && batch.expiry < today
                            })
                        }

                        StockFilter::OutOfStock => row.qty + row.free_qty <= 0.0,
                    }
                })
                .collect();

            ScrollArea::vertical()
                .id_salt("stock-table-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {

                    for (index, row) in visible_rows.iter().enumerate() {
                        let row = *row;
                        let selected =
                            state.selected == Some(row.product.id);

                        let row_width = ui.available_width();

                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(row_width, 58.0),
                            Sense::click(),
                        );

                        if response.clicked() {
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(
                                    Id::new(SELECTED_ID),
                                    row.product.id,
                                );
                            });
                        }

                        let current_selected = ui.ctx().data(|d| {
                            d.get_temp::<i64>(Id::new(SELECTED_ID))
                                .unwrap_or(-1)
                        }) == row.product.id;

                        let background = if selected || current_selected {
                            Color32::from_rgb(244, 240, 250)
                        } else if index % 2 == 1 {
                            Color32::from_rgb(252, 251, 253)
                        } else {
                            Color32::WHITE
                        };

                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::ZERO,
                            background,
                        );

                        ui.painter().line_segment(
                            [rect.left_bottom(), rect.right_bottom()],
                            Stroke::new(1.0_f32, BORDER),
                        );

                        let widths = columns(ui.available_width() + 24.0);

                        let mut child = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(
                                    rect.shrink2(Vec2::new(12.0, 0.0)),
                                )
                                .layout(Layout::left_to_right(Align::Center)),
                        );

                        child.allocate_ui_with_layout(
                            Vec2::new(widths[0], 48.0),
                            Layout::top_down(Align::Min),
                            |ui| {
                                ui.add_space(4.0);

                                ui.label(
                                    RichText::new(&row.product.name)
                                        .size(12.0)
                                        .strong()
                                        .color(INK),
                                );

                                ui.label(
                                    RichText::new(
                                        if row.product.brand.is_empty() {
                                            "—"
                                        } else {
                                            &row.product.brand
                                        },
                                    )
                                    .size(10.0)
                                    .color(MUTED),
                                );
                            },
                        );

                        child.allocate_ui_with_layout(
                            Vec2::new(widths[1], 48.0),
                            Layout::top_down(Align::Center),
                            |ui| {
                                let low = row.product.reorder > 0.0
                                    && row.qty <= row.product.reorder;

                                ui.label(
                                    RichText::new(format_qty(row.qty))
                                        .size(12.0)
                                        .strong()
                                        .color(if low { AMBER } else { INK }),
                                );

                                ui.label(
                                    RichText::new(&row.product.unit)
                                        .size(9.0)
                                        .color(MUTED),
                                );
                            },
                        );

                        child.allocate_ui_with_layout(
                            Vec2::new(widths[2], 48.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.label(
                                    RichText::new(
                                        row.batches.len().to_string(),
                                    )
                                    .size(12.0)
                                    .color(INK),
                                );
                            },
                        );

                        child.allocate_ui_with_layout(
                            Vec2::new(widths[3], 48.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.label(
                                    RichText::new(money(row.stock_value))
                                        .size(11.0)
                                        .strong()
                                        .color(INK),
                                );
                            },
                        );
                    }

                    if visible_rows.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                RichText::new("No matching stock items")
                                    .color(MUTED),
                            );
                        });
                    }
                });
        });
}

fn columns(width: f32) -> [f32; 4] {
    let usable = (width - 24.0).max(280.0);

    [
        usable * 0.48,
        usable * 0.17,
        usable * 0.13,
        usable * 0.22,
    ]
}

fn draw_detail(ui: &mut egui::Ui, state: &State) {
    let Some(id) = state.selected else {
        return;
    };

    let Some(row) = state.rows.iter().find(|r| r.product.id == id) else {
        return;
    };

    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("PRODUCT REPORT")
                            .size(9.0)
                            .strong()
                            .color(PURPLE),
                    );

                    ui.label(
                        RichText::new(&row.product.name)
                            .size(17.0)
                            .strong()
                            .color(INK),
                    );

                    if !row.product.brand.is_empty() {
                        ui.label(
                            RichText::new(&row.product.brand)
                                .size(11.0)
                                .color(MUTED),
                        );
                    }
                });

                ui.with_layout(
                    Layout::right_to_left(Align::Min),
                    |ui| {
                        if ui
                            .button(RichText::new("✕").strong())
                            .on_hover_text("Close product report")
                            .clicked()
                        {
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(
                                    Id::new(SELECTED_ID),
                                    -1_i64,
                                );
                            });
                        }
                    },
                );
            });

            ui.add_space(12.0);

            ui.columns(2, |cols| {
                mini_stat(&mut cols[0], "ON HAND", &format_qty(row.qty));
                mini_stat(
                    &mut cols[1],
                    "FREE QTY",
                    &format_qty(row.free_qty),
                );
            });

            ui.add_space(6.0);

            ui.columns(2, |cols| {
                mini_stat(
                    &mut cols[0],
                    "BATCHES",
                    &row.batches.len().to_string(),
                );
                mini_stat(
                    &mut cols[1],
                    "STOCK VALUE",
                    &money(row.stock_value),
                );
            });

            ui.add_space(16.0);

            ui.label(
                RichText::new("BATCH BREAKDOWN")
                    .size(10.0)
                    .strong()
                    .color(MUTED),
            );

            ui.add_space(6.0);

            ScrollArea::vertical()
                .id_salt("stock-detail-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for batch in &row.batches {
                        egui::Frame::new()
                            .fill(CANVAS)
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::same(10))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "Batch {}",
                                            batch.batch
                                        ))
                                        .strong()
                                        .color(INK),
                                    );

                                    ui.with_layout(
                                        Layout::right_to_left(Align::Center),
                                        |ui| {
                                            let expiry =
                                                expiry_label(&batch.expiry);

                                            ui.label(
                                                RichText::new(expiry.0)
                                                    .size(10.0)
                                                    .strong()
                                                    .color(expiry.1),
                                            );
                                        },
                                    );
                                });

                                ui.add_space(4.0);

                                ui.columns(2, |cols| {
                                    cols[0].label(
                                        RichText::new(format!(
                                            "Qty  {}",
                                            format_qty(batch.qty)
                                        ))
                                        .size(11.0)
                                        .color(INK),
                                    );

                                    cols[1].label(
                                        RichText::new(format!(
                                            "Free  {}",
                                            format_qty(batch.free_qty)
                                        ))
                                        .size(11.0)
                                        .color(MUTED),
                                    );
                                });

                                ui.columns(2, |cols| {
                                    cols[0].label(
                                        RichText::new(format!(
                                            "MRP  {}",
                                            money(batch.mrp)
                                        ))
                                        .size(10.0)
                                        .color(MUTED),
                                    );

                                    cols[1].label(
                                        RichText::new(format!(
                                            "Sale  {}",
                                            money(batch.sale_rate)
                                        ))
                                        .size(10.0)
                                        .color(MUTED),
                                    );
                                });
                            });

                        ui.add_space(6.0);
                    }

                    if row.batches.is_empty() {
                        ui.label(
                            RichText::new(
                                "No batches recorded for this product.",
                            )
                            .color(MUTED),
                        );
                    }
                });
        });
}

fn mini_stat(ui: &mut egui::Ui, label: &str, value: &str) {
    egui::Frame::new()
        .fill(CANVAS)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(9))
        .show(ui, |ui| {
            ui.label(
                RichText::new(label)
                    .size(9.0)
                    .strong()
                    .color(MUTED),
            );

            ui.label(
                RichText::new(value)
                    .size(14.0)
                    .strong()
                    .color(INK),
            );
        });
}

fn format_qty(value: f64) -> String {
    format!("{value:.3}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

fn money(paise: i64) -> String {
    format!("₹{:.2}", paise as f64 / 100.0)
}

fn expiry_label(date: &str) -> (String, Color32) {
    if date.is_empty() {
        return ("No expiry".into(), MUTED);
    }

    let (year, month, day) = crate::date::local_date();
    let today = format!("{year:04}-{month:02}-{day:02}");

    if date < today.as_str() {
        (format!("Expired · {date}"), RED)
    } else {
        (format!("Exp {date}"), GREEN)
    }
}
fn date_window() -> (String, String) {
    let (year, month, day) = crate::date::local_date();
    let (mut y, mut m, mut d) = (year, month as i32, day as i32);

    for _ in 0..90 {
        d += 1;
        if d > days_in_month(y, m) {
            d = 1;
            m += 1;
            if m > 12 {
                m = 1;
                y += 1;
            }
        }
    }

    (
        format!("{year:04}-{month:02}-{day:02}"),
        format!("{y:04}-{m:02}-{d:02}"),
    )
}

fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        _ => 30,
    }
}

