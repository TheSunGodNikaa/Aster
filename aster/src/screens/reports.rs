use crate::app::AsterApp;
use crate::utils::currency::format_inr;
use eframe::egui::{self, Align, Color32, Id, Layout, RichText, ScrollArea, Sense};
use rusqlite::{Connection, Row, ToSql};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

// ── Palette ────────────────────────────────────────────────────────────────
const INK: Color32 = Color32::from_rgb(33, 30, 43);
const MUTED: Color32 = Color32::from_rgb(110, 104, 122);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const PURPLE_TINT: Color32 = Color32::from_rgb(244, 241, 250);
const BORDER: Color32 = Color32::from_rgb(228, 225, 234);
const SURFACE: Color32 = Color32::from_rgb(250, 249, 252);
const GREEN: Color32 = Color32::from_rgb(30, 122, 80);
const GREEN_TINT: Color32 = Color32::from_rgb(232, 245, 238);
const RED: Color32 = Color32::from_rgb(177, 50, 50);
const RED_TINT: Color32 = Color32::from_rgb(252, 238, 238);
const AMBER: Color32 = Color32::from_rgb(176, 112, 16);
const BLUE: Color32 = Color32::from_rgb(38, 99, 170);

const ACCENTS: [Color32; 4] = [PURPLE, GREEN, BLUE, AMBER];
const PAGE_SIZE: usize = 50;
const NEAR_EXPIRY_DAYS: i64 = 90;
const MONTHS: [&str; 12] = [
    "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "Jan", "Feb", "Mar",
];

// ── Types ──────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum ReportKind {
    #[default]
    Sales,
    Purchases,
    Stock,
    Receivables,
    Payments,
    Gst,
}

impl ReportKind {
    const ALL: [Self; 6] = [
        Self::Sales,
        Self::Purchases,
        Self::Stock,
        Self::Receivables,
        Self::Payments,
        Self::Gst,
    ];

    fn title(self) -> &'static str {
        match self {
            Self::Sales => "Sales",
            Self::Purchases => "Purchases",
            Self::Stock => "Stock",
            Self::Receivables => "Receivables",
            Self::Payments => "Payments",
            Self::Gst => "GST summary",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::Sales => "sales",
            Self::Purchases => "purchases",
            Self::Stock => "stock",
            Self::Receivables => "receivables",
            Self::Payments => "payments",
            Self::Gst => "gst_summary",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Sales => "Posted sales invoices within the selected date range",
            Self::Purchases => "Posted purchase documents within the selected date range",
            Self::Stock => "Current batch-wise stock, expiry status and valuation",
            Self::Receivables => "Outstanding balances with ageing, from posted sales invoices",
            Self::Payments => "Customer receipts recorded within the selected date range",
            Self::Gst => "Output tax breakup by HSN code for posted sales",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ColKind {
    Text,
    Money, // value = paise, summed in footer
    Qty,   // value = milli-units, summed in footer
    Num,   // numeric, right aligned, not summed
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    Normal,
    Muted,
    Good,
    Bad,
    Warn,
}

#[derive(Clone)]
struct Col {
    name: &'static str,
    kind: ColKind,
}

impl Col {
    fn text(name: &'static str) -> Self {
        Self { name, kind: ColKind::Text }
    }
    fn money(name: &'static str) -> Self {
        Self { name, kind: ColKind::Money }
    }
    fn qty(name: &'static str) -> Self {
        Self { name, kind: ColKind::Qty }
    }
    fn num(name: &'static str) -> Self {
        Self { name, kind: ColKind::Num }
    }
    fn right(&self) -> bool {
        self.kind != ColKind::Text
    }
}

#[derive(Clone)]
struct Cell {
    text: String,
    value: Option<i64>,
    tone: Tone,
}

impl Cell {
    fn text(text: impl Into<String>) -> Self {
        Self { text: text.into(), value: None, tone: Tone::Normal }
    }
    fn toned(text: impl Into<String>, tone: Tone) -> Self {
        Self { text: text.into(), value: None, tone }
    }
    fn money(paise: i64) -> Self {
        Self { text: money(paise), value: Some(paise), tone: Tone::Normal }
    }
    fn money_tone(paise: i64, tone: Tone) -> Self {
        Self { text: money(paise), value: Some(paise), tone }
    }
    fn qty(milli: i64) -> Self {
        Self { text: qty_text(milli), value: Some(milli), tone: Tone::Normal }
    }
    fn num(value: i64, text: String, tone: Tone) -> Self {
        Self { text, value: Some(value), tone }
    }
}

#[derive(Clone, Default)]
struct ReportRow {
    cells: Vec<Cell>,
}

#[derive(Clone)]
struct Metric {
    label: String,
    value: String,
    sub: String,
}

impl Metric {
    fn new(label: &str, value: impl Into<String>, sub: impl Into<String>) -> Self {
        Self { label: label.into(), value: value.into(), sub: sub.into() }
    }
}

#[derive(Clone, Default)]
struct ReportState {
    year: i32,
    kind: ReportKind,
    from_date: String,
    to_date: String,
    search: String,
    cols: Vec<Col>,
    rows: Vec<ReportRow>,
    metrics: Vec<Metric>,
    sort_col: Option<usize>,
    sort_asc: bool,
    page: usize,
    error: String,
    error_detail: String,
    notice: String,
    loaded: bool,
}

type Res = rusqlite::Result<()>;

// ── Entry point ────────────────────────────────────────────────────────────
pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let id = Id::new("aster_reports_workspace_v2");
    let mut state = ui
        .ctx()
        .data_mut(|data| data.get_temp::<ReportState>(id).unwrap_or_default());

    if state.year != app.selected_year {
        state = ReportState {
            year: app.selected_year,
            from_date: format!("{}-04-01", app.selected_year),
            to_date: format!("{}-03-31", app.selected_year + 1),
            ..Default::default()
        };
    }

    if !state.loaded {
        load_report(app, &mut state);
    }

    ScrollArea::vertical()
        .id_salt("aster_reports_page")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            draw_header(ui, app, &mut state);
            ui.add_space(14.0);
            draw_report_selector(ui, app, &mut state);
            ui.add_space(12.0);
            draw_filters(ui, app, &mut state);
            ui.add_space(14.0);

            if !state.error.is_empty() {
                draw_error(ui, app, &mut state);
            } else {
                draw_metrics(ui, &state);
                ui.add_space(16.0);
                draw_table(ui, &mut state);
            }

            ui.add_space(24.0);
        });

    ui.ctx().data_mut(|data| data.insert_temp(id, state));
}

// ── Layout helpers ─────────────────────────────────────────────────────────
fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(egui::Stroke::new(1.0_f32, BORDER))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(14.0)
}

fn tone_color(tone: Tone) -> Color32 {
    match tone {
        Tone::Normal => INK,
        Tone::Muted => MUTED,
        Tone::Good => GREEN,
        Tone::Bad => RED,
        Tone::Warn => AMBER,
    }
}

fn small_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_owned()).size(12.0).color(INK))
        .fill(SURFACE)
        .stroke(egui::Stroke::new(1.0_f32, BORDER))
        .corner_radius(egui::CornerRadius::same(7))
}

fn draw_header(ui: &mut egui::Ui, app: &AsterApp, state: &mut ReportState) {
    egui::Frame::new()
        .fill(PURPLE_TINT)
        .stroke(egui::Stroke::new(1.0_f32, BORDER))
        .corner_radius(egui::CornerRadius::same(14))
        .inner_margin(egui::Margin::symmetric(18, 16))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("BUSINESS INTELLIGENCE")
                            .size(11.0)
                            .strong()
                            .color(PURPLE),
                    );
                    ui.add_space(2.0);
                    ui.heading(RichText::new("Reports").size(28.0).color(INK).strong());
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(format!(
                            "FY {}–{}  ·  {}",
                            app.selected_year,
                            (app.selected_year + 1) % 100,
                            state.kind.description()
                        ))
                        .size(12.5)
                        .color(MUTED),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let refresh = egui::Button::new(
                        RichText::new("↻  Refresh").size(12.5).strong().color(Color32::WHITE),
                    )
                    .fill(PURPLE)
                    .corner_radius(egui::CornerRadius::same(8));
                    if ui.add(refresh).clicked() {
                        state.notice.clear();
                        load_report(app, state);
                    }
                    if ui
                        .add_enabled(state.error.is_empty(), small_button("⬇  Export CSV"))
                        .clicked()
                    {
                        state.notice = export_csv(state);
                    }
                });
            });
        });

    if !state.notice.is_empty() {
        ui.add_space(6.0);
        ui.label(RichText::new(&state.notice).size(11.5).color(GREEN));
    }
}

fn draw_report_selector(ui: &mut egui::Ui, app: &AsterApp, state: &mut ReportState) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        for kind in ReportKind::ALL {
            let selected = state.kind == kind;
            let button = egui::Button::new(
                RichText::new(kind.title())
                    .size(12.5)
                    .strong()
                    .color(if selected { Color32::WHITE } else { INK }),
            )
            .fill(if selected { PURPLE } else { Color32::WHITE })
            .stroke(egui::Stroke::new(1.0_f32, if selected { PURPLE } else { BORDER }))
            .corner_radius(egui::CornerRadius::same(18))
            .min_size(egui::vec2(0.0, 30.0));

            if ui.add(button).clicked() && state.kind != kind {
                state.kind = kind;
                state.search.clear();
                state.sort_col = None;
                state.page = 0;
                state.notice.clear();
                load_report(app, state);
            }
        }
    });
}

fn draw_filters(ui: &mut egui::Ui, app: &AsterApp, state: &mut ReportState) {
    let mut preset: Option<(String, String)> = None;
    let mut apply = false;
    let uses_dates = state.kind != ReportKind::Stock;
    let year = state.year;

    card().show(ui, |ui| {
        ui.add_enabled_ui(uses_dates, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                ui.label(RichText::new("From").size(12.0).color(MUTED));
                let from = ui.add(
                    egui::TextEdit::singleline(&mut state.from_date)
                        .desired_width(100.0)
                        .hint_text("YYYY-MM-DD"),
                );
                ui.label(RichText::new("To").size(12.0).color(MUTED));
                let to = ui.add(
                    egui::TextEdit::singleline(&mut state.to_date)
                        .desired_width(100.0)
                        .hint_text("YYYY-MM-DD"),
                );
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.add(small_button("Apply")).clicked()
                    || (from.lost_focus() && enter)
                    || (to.lost_focus() && enter)
                {
                    apply = true;
                }

                ui.separator();
                ui.label(RichText::new("Quick range").size(12.0).color(MUTED));
                if ui.add(small_button("Full year")).clicked() {
                    preset = Some((format!("{year}-04-01"), format!("{}-03-31", year + 1)));
                }
                for (name, (m1, y1), (m2, y2)) in [
                    ("Q1", (4, year), (6, year)),
                    ("Q2", (7, year), (9, year)),
                    ("Q3", (10, year), (12, year)),
                    ("Q4", (1, year + 1), (3, year + 1)),
                ] {
                    if ui.add(small_button(name)).clicked() {
                        preset = Some((
                            format!("{y1}-{m1:02}-01"),
                            format!("{y2}-{m2:02}-{:02}", days_in_month(y2, m2)),
                        ));
                    }
                }
                egui::ComboBox::from_id_salt("aster_reports_month")
                    .selected_text("Month")
                    .width(70.0)
                    .show_ui(ui, |ui| {
                        for (i, name) in MONTHS.iter().enumerate() {
                            let m = ((3 + i) % 12 + 1) as u32;
                            let y = if m >= 4 { year } else { year + 1 };
                            if ui.selectable_label(false, *name).clicked() {
                                preset = Some((
                                    format!("{y}-{m:02}-01"),
                                    format!("{y}-{m:02}-{:02}", days_in_month(y, m)),
                                ));
                            }
                        }
                    });
            });
        });

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Search").size(12.0).color(MUTED));
            let before = state.search.clone();
            ui.add(
                egui::TextEdit::singleline(&mut state.search)
                    .desired_width(240.0)
                    .hint_text("Filter rows by any value"),
            );
            if before != state.search {
                state.page = 0;
            }
            if !state.search.is_empty() && ui.add(small_button("Clear")).clicked() {
                state.search.clear();
                state.page = 0;
            }
            if !uses_dates {
                ui.separator();
                ui.label(
                    RichText::new("Stock is a live snapshot; date filters don't apply.")
                        .size(11.5)
                        .color(MUTED),
                );
            }
        });
    });

    if let Some((from, to)) = preset {
        state.from_date = from;
        state.to_date = to;
        apply = true;
    }
    if apply {
        state.notice.clear();
        load_report(app, state);
    }
}

fn draw_error(ui: &mut egui::Ui, app: &AsterApp, state: &mut ReportState) {
    egui::Frame::new()
        .fill(RED_TINT)
        .stroke(egui::Stroke::new(1.0_f32, RED))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!("{} report unavailable", state.kind.title()))
                    .size(15.0)
                    .strong()
                    .color(RED),
            );
            ui.add_space(4.0);
            ui.label(RichText::new(&state.error).size(12.5).color(INK));
            if !state.error_detail.is_empty() {
                ui.add_space(6.0);
                ui.collapsing(RichText::new("Technical details").size(11.5).color(MUTED), |ui| {
                    ui.label(RichText::new(&state.error_detail).size(11.0).monospace().color(MUTED));
                });
            }
            ui.add_space(8.0);
            if ui.add(small_button("Try again")).clicked() {
                load_report(app, state);
            }
        });
}

fn draw_metrics(ui: &mut egui::Ui, state: &ReportState) {
    if state.metrics.is_empty() {
        return;
    }
    ui.columns(state.metrics.len(), |columns| {
        for (index, column) in columns.iter_mut().enumerate() {
            let metric = &state.metrics[index];
            let accent = ACCENTS[index % ACCENTS.len()];
            card().show(column, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(4.0, 14.0), Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, accent);
                    ui.label(
                        RichText::new(metric.label.to_uppercase())
                            .size(10.5)
                            .strong()
                            .color(MUTED),
                    );
                });
                ui.add_space(6.0);
                ui.label(RichText::new(&metric.value).size(22.0).strong().color(INK));
                if !metric.sub.is_empty() {
                    ui.add_space(2.0);
                    ui.label(RichText::new(&metric.sub).size(11.0).color(MUTED));
                }
            });
        }
    });
}

fn draw_table(ui: &mut egui::Ui, state: &mut ReportState) {
    let order = visible_indices(state);
    let total_rows = order.len();
    let pages = total_rows.div_ceil(PAGE_SIZE).max(1);
    if state.page >= pages {
        state.page = pages - 1;
    }
    let page = state.page;
    let start = page * PAGE_SIZE;
    let end = (start + PAGE_SIZE).min(total_rows);

    let mut clicked_sort: Option<usize> = None;
    let mut new_page = page;

    {
        let st: &ReportState = state;

        ui.horizontal(|ui| {
            ui.heading(RichText::new(st.kind.title()).size(17.0).strong().color(INK));
            let badge = if st.search.trim().is_empty() {
                format!("{} records", st.rows.len())
            } else {
                format!("{} of {} records", total_rows, st.rows.len())
            };
            ui.label(RichText::new(badge).size(11.5).color(MUTED));
            if st.cols.is_empty() {
                return;
            }
            ui.label(RichText::new("· click a column to sort").size(11.0).color(MUTED));
        });
        ui.add_space(8.0);

        card().inner_margin(10.0).show(ui, |ui| {
            if st.cols.is_empty() {
                ui.label("No report columns are available.");
                return;
            }

            ScrollArea::horizontal()
                .id_salt("aster_reports_horizontal")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    egui::Grid::new("aster_reports_table")
                        .striped(true)
                        .min_col_width(92.0)
                        .spacing([18.0, 10.0])
                        .show(ui, |ui| {
                            for (i, col) in st.cols.iter().enumerate() {
                                let arrow = match (st.sort_col == Some(i), st.sort_asc) {
                                    (true, true) => " ▲",
                                    (true, false) => " ▼",
                                    _ => "",
                                };
                                let label = egui::Label::new(
                                    RichText::new(format!("{}{}", col.name.to_uppercase(), arrow))
                                        .size(10.5)
                                        .strong()
                                        .color(PURPLE),
                                )
                                .sense(Sense::click());
                                let response = if col.right() {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.add(label)
                                    })
                                    .inner
                                } else {
                                    ui.add(label)
                                };
                                if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    clicked_sort = Some(i);
                                }
                            }
                            ui.end_row();

                            for &ri in &order[start..end] {
                                let row = &st.rows[ri];
                                for (ci, cell) in row.cells.iter().enumerate() {
                                    let rt = RichText::new(&cell.text)
                                        .size(12.0)
                                        .color(tone_color(cell.tone));
                                    if st.cols[ci].right() {
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            ui.label(rt);
                                        });
                                    } else {
                                        ui.label(rt);
                                    }
                                }
                                ui.end_row();
                            }

                            // Totals footer (covers every filtered row, not just this page)
                            if total_rows > 0 && st.cols.iter().any(|c| matches!(c.kind, ColKind::Money | ColKind::Qty)) {
                                for (ci, col) in st.cols.iter().enumerate() {
                                    match col.kind {
                                        ColKind::Money | ColKind::Qty => {
                                            let sum: i64 = order
                                                .iter()
                                                .filter_map(|&r| st.rows[r].cells[ci].value)
                                                .sum();
                                            let text = if col.kind == ColKind::Money {
                                                money(sum)
                                            } else {
                                                qty_text(sum)
                                            };
                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                ui.label(RichText::new(text).size(12.0).strong().color(PURPLE));
                                            });
                                        }
                                        _ if ci == 0 => {
                                            ui.label(RichText::new("TOTAL").size(11.0).strong().color(PURPLE));
                                        }
                                        _ => {
                                            ui.label("");
                                        }
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });

            if total_rows == 0 {
                ui.add_space(18.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("No matching records").size(14.0).strong().color(INK));
                    ui.label(
                        RichText::new("Try a different date range or clear the search filter.")
                            .size(12.0)
                            .color(MUTED),
                    );
                });
                ui.add_space(18.0);
            }
        });

        if pages > 1 {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(page > 0, small_button("‹ Previous")).clicked() {
                    new_page = page - 1;
                }
                ui.label(
                    RichText::new(format!(
                        "Page {} of {}  ·  rows {}–{}",
                        page + 1,
                        pages,
                        start + 1,
                        end
                    ))
                    .size(11.5)
                    .color(MUTED),
                );
                if ui.add_enabled(page + 1 < pages, small_button("Next ›")).clicked() {
                    new_page = page + 1;
                }
            });
        }
    }

    if let Some(i) = clicked_sort {
        if state.sort_col == Some(i) {
            state.sort_asc = !state.sort_asc;
        } else {
            state.sort_col = Some(i);
            state.sort_asc = true;
        }
        state.page = 0;
    } else {
        state.page = new_page;
    }
}

/// Indices of rows that match the search, in the active sort order.
fn visible_indices(state: &ReportState) -> Vec<usize> {
    let search = state.search.trim().to_lowercase();
    let mut idx: Vec<usize> = state
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            search.is_empty() || row.cells.iter().any(|c| c.text.to_lowercase().contains(&search))
        })
        .map(|(i, _)| i)
        .collect();

    if let Some(col) = state.sort_col {
        idx.sort_by(|&a, &b| {
            let (ca, cb) = (&state.rows[a].cells[col], &state.rows[b].cells[col]);
            let ord = match (ca.value, cb.value) {
                (Some(x), Some(y)) => x.cmp(&y),
                _ => ca.text.to_lowercase().cmp(&cb.text.to_lowercase()),
            };
            if state.sort_asc { ord } else { ord.reverse() }
        });
    }
    idx
}

// ── CSV export ─────────────────────────────────────────────────────────────
fn export_csv(state: &ReportState) -> String {
    fn esc(value: &str) -> String {
        if value.contains([',', '"', '\n']) {
            format!("\"{}\"", value.replace('"', "\"\""))
        } else {
            value.to_owned()
        }
    }

    let mut out = String::new();
    out.push_str(&state.cols.iter().map(|c| esc(c.name)).collect::<Vec<_>>().join(","));
    out.push('\n');
    for &i in &visible_indices(state) {
        let line = state.rows[i]
            .cells
            .iter()
            .map(|c| esc(&c.text.replace('₹', "")))
            .collect::<Vec<_>>()
            .join(",");
        out.push_str(&line);
        out.push('\n');
    }

    let name = if state.kind == ReportKind::Stock {
        format!("aster_stock_{}.csv", today_string())
    } else {
        format!("aster_{}_{}_to_{}.csv", state.kind.slug(), state.from_date, state.to_date)
    };
    let path = std::env::current_dir().unwrap_or_default().join(&name);
    match std::fs::write(&path, out) {
        Ok(()) => format!("Exported {} rows to {}", visible_indices(state).len(), path.display()),
        Err(error) => format!("Export failed: {error}"),
    }
}

// ── Loading ────────────────────────────────────────────────────────────────
fn load_report(app: &AsterApp, state: &mut ReportState) {
    state.error.clear();
    state.error_detail.clear();
    state.rows.clear();
    state.cols.clear();
    state.metrics.clear();
    state.page = 0;
    // Always mark as attempted so a failure never re-runs every frame.
    state.loaded = true;

    if state.kind != ReportKind::Stock && !valid_date_range(&state.from_date, &state.to_date) {
        state.error =
            "Enter valid dates in YYYY-MM-DD format. The From date must not be after the To date."
                .into();
        return;
    }

    let Some(db) = app.db.as_ref() else {
        state.error = "The database is not available. Restart the application and try again.".into();
        return;
    };

    let connection = match db.year_connection(app.selected_year) {
        Ok(connection) => connection,
        Err(error) => {
            state.error = format!(
                "Could not open the books for financial year {}.",
                app.selected_year
            );
            state.error_detail = error.to_string();
            return;
        }
    };

    let result = match state.kind {
        ReportKind::Sales => load_sales(&connection, state),
        ReportKind::Purchases => load_purchases(&connection, state),
        ReportKind::Stock => load_stock(&connection, state),
        ReportKind::Receivables => load_receivables(&connection, state),
        ReportKind::Payments => load_payments(&connection, state),
        ReportKind::Gst => load_gst(&connection, state),
    };

    if let Err(error) = result {
        // Never surface raw SQL to the user; keep it behind "Technical details".
        eprintln!("[reports] {} report failed: {error}", state.kind.title());
        state.rows.clear();
        state.cols.clear();
        state.metrics.clear();
        state.error = format!(
            "The {} report could not be generated. Your data is unchanged. \
             Try refreshing, and contact support if this continues.",
            state.kind.title().to_lowercase()
        );
        state.error_detail = error.to_string();
    }
}

fn query_rows<F>(
    connection: &Connection,
    sql: &str,
    args: &[&dyn ToSql],
    mut map: F,
) -> rusqlite::Result<Vec<ReportRow>>
where
    F: FnMut(&Row) -> rusqlite::Result<Vec<Cell>>,
{
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map(rusqlite::params_from_iter(args.iter().copied()), |row| {
        Ok(ReportRow { cells: map(row)? })
    })?;
    rows.collect()
}

fn txt(row: &Row, i: usize) -> rusqlite::Result<String> {
    Ok(row
        .get::<_, Option<String>>(i)?
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "—".into()))
}

fn int(row: &Row, i: usize) -> rusqlite::Result<i64> {
    Ok(row.get::<_, Option<i64>>(i)?.unwrap_or(0))
}

fn col_sum(rows: &[ReportRow], col: usize) -> i64 {
    rows.iter().filter_map(|r| r.cells[col].value).sum()
}

fn percent(part: i64, whole: i64) -> String {
    if whole <= 0 {
        "0.0%".into()
    } else {
        format!("{:.1}%", part as f64 * 100.0 / whole as f64)
    }
}

fn load_sales(connection: &Connection, state: &mut ReportState) -> Res {
    state.cols = vec![
        Col::text("Invoice no."),
        Col::text("Date"),
        Col::text("Customer"),
        Col::money("Taxable"),
        Col::money("CGST"),
        Col::money("SGST"),
        Col::money("IGST"),
        Col::money("Total"),
        Col::money("Received"),
        Col::money("Due"),
    ];
    let (from, to) = (state.from_date.clone(), state.to_date.clone());

    state.rows = query_rows(
        connection,
        "SELECT invoice_no, invoice_date, customer_name_snap,
                taxable_paise, cgst_paise, sgst_paise, igst_paise,
                total_paise, received_paise
         FROM sales_invoices
         WHERE status = 'posted' AND invoice_date BETWEEN ?1 AND ?2
         ORDER BY invoice_date DESC, id DESC",
        &[&from, &to],
        |r| {
            let total = int(r, 7)?;
            let received = int(r, 8)?;
            let due = (total - received).max(0);
            Ok(vec![
                Cell::text(txt(r, 0)?),
                Cell::text(txt(r, 1)?),
                Cell::text(txt(r, 2)?),
                Cell::money(int(r, 3)?),
                Cell::money(int(r, 4)?),
                Cell::money(int(r, 5)?),
                Cell::money(int(r, 6)?),
                Cell::money(total),
                Cell::money_tone(received, if received > 0 { Tone::Good } else { Tone::Muted }),
                Cell::money_tone(due, if due > 0 { Tone::Bad } else { Tone::Muted }),
            ])
        },
    )?;

    let total = col_sum(&state.rows, 7);
    let received = col_sum(&state.rows, 8);
    let due = col_sum(&state.rows, 9);
    let open = state.rows.iter().filter(|r| r.cells[9].value.unwrap_or(0) > 0).count();
    let n = state.rows.len() as i64;
    let avg = if n > 0 { total / n } else { 0 };

    state.metrics = vec![
        Metric::new("Invoices", n.to_string(), format!("Avg. invoice {}", money(avg))),
        Metric::new("Sales total", money(total), format!("Taxable {}", money(col_sum(&state.rows, 3)))),
        Metric::new("Amount received", money(received), format!("{} collected", percent(received, total))),
        Metric::new("Outstanding", money(due), format!("{open} invoices unpaid")),
    ];
    Ok(())
}

fn load_purchases(connection: &Connection, state: &mut ReportState) -> Res {
    state.cols = vec![
        Col::text("Purchase no."),
        Col::text("Date"),
        Col::text("Supplier"),
        Col::money("Taxable"),
        Col::money("CGST"),
        Col::money("SGST"),
        Col::money("IGST"),
        Col::money("Total"),
    ];
    let (from, to) = (state.from_date.clone(), state.to_date.clone());

    state.rows = query_rows(
        connection,
        "SELECT purchase_no, invoice_date, supplier_name_snap,
                taxable_paise, cgst_paise, sgst_paise, igst_paise, total_paise
         FROM purchase_documents
         WHERE status = 'posted' AND invoice_date BETWEEN ?1 AND ?2
         ORDER BY invoice_date DESC, id DESC",
        &[&from, &to],
        |r| {
            Ok(vec![
                Cell::text(txt(r, 0)?),
                Cell::text(txt(r, 1)?),
                Cell::text(txt(r, 2)?),
                Cell::money(int(r, 3)?),
                Cell::money(int(r, 4)?),
                Cell::money(int(r, 5)?),
                Cell::money(int(r, 6)?),
                Cell::money(int(r, 7)?),
            ])
        },
    )?;

    let tax = col_sum(&state.rows, 4) + col_sum(&state.rows, 5) + col_sum(&state.rows, 6);
    let suppliers: HashSet<&str> = state.rows.iter().map(|r| r.cells[2].text.as_str()).collect();
    state.metrics = vec![
        Metric::new("Purchase documents", state.rows.len().to_string(), format!("{} suppliers", suppliers.len())),
        Metric::new("Purchase total", money(col_sum(&state.rows, 7)), "Including tax"),
        Metric::new("Taxable value", money(col_sum(&state.rows, 3)), "Before tax"),
        Metric::new("Input tax", money(tax), "CGST + SGST + IGST"),
    ];
    Ok(())
}

fn load_stock(connection: &Connection, state: &mut ReportState) -> Res {
    state.cols = vec![
        Col::text("Product"),
        Col::text("Batch"),
        Col::text("Expiry"),
        Col::text("Status"),
        Col::qty("Qty"),
        Col::money("Purchase rate"),
        Col::money("Stock value"),
    ];
    let today = today_days();

    state.rows = query_rows(
        connection,
        "SELECT p.name, b.batch_no, b.expiry_date,
                b.qty_on_hand_milli, b.purchase_rate_paise
         FROM batches b
         JOIN products p ON p.id = b.product_id
         WHERE b.active = 1 AND b.qty_on_hand_milli > 0
         ORDER BY p.name, b.expiry_date, b.batch_no",
        &[],
        |r| {
            let expiry = txt(r, 2)?;
            let qty = int(r, 3)?;
            let rate = int(r, 4)?;
            let value = ((qty as i128 * rate as i128) / 1000) as i64;
            let (status, tone) = match parse_ymd(&expiry).map(|(y, m, d)| days_from_civil(y, m, d) - today) {
                Some(d) if d < 0 => ("Expired".to_string(), Tone::Bad),
                Some(d) if d <= NEAR_EXPIRY_DAYS => (format!("Expires in {d} d"), Tone::Warn),
                Some(_) => ("In date".to_string(), Tone::Good),
                None => ("No expiry".to_string(), Tone::Muted),
            };
            Ok(vec![
                Cell::text(txt(r, 0)?),
                Cell::text(txt(r, 1)?),
                Cell::text(expiry),
                Cell::toned(status, tone),
                Cell::qty(qty),
                Cell::money(rate),
                Cell::money(value),
            ])
        },
    )?;

    let expired = state.rows.iter().filter(|r| r.cells[3].tone == Tone::Bad).count();
    let near = state.rows.iter().filter(|r| r.cells[3].tone == Tone::Warn).count();
    let products: HashSet<&str> = state.rows.iter().map(|r| r.cells[0].text.as_str()).collect();

    state.metrics = vec![
        Metric::new("Active batches", state.rows.len().to_string(), format!("{} products", products.len())),
        Metric::new("Quantity on hand", qty_text(col_sum(&state.rows, 4)), "Across all batches"),
        Metric::new("Stock valuation", money(col_sum(&state.rows, 6)), "At purchase rate"),
        Metric::new("Needs attention", (expired + near).to_string(), format!("{expired} expired · {near} within {NEAR_EXPIRY_DAYS} d")),
    ];
    Ok(())
}

fn load_receivables(connection: &Connection, state: &mut ReportState) -> Res {
    state.cols = vec![
        Col::text("Invoice no."),
        Col::text("Date"),
        Col::text("Customer"),
        Col::num("Age"),
        Col::money("Invoice total"),
        Col::money("Received"),
        Col::money("Outstanding"),
    ];
    let (from, to) = (state.from_date.clone(), state.to_date.clone());
    let today = today_days();

    state.rows = query_rows(
        connection,
        "SELECT invoice_no, invoice_date, customer_name_snap,
                total_paise, received_paise
         FROM sales_invoices
         WHERE status = 'posted'
           AND invoice_date BETWEEN ?1 AND ?2
           AND total_paise > received_paise
         ORDER BY invoice_date, id",
        &[&from, &to],
        |r| {
            let date = txt(r, 1)?;
            let total = int(r, 3)?;
            let received = int(r, 4)?;
            let age = parse_ymd(&date)
                .map(|(y, m, d)| (today - days_from_civil(y, m, d)).max(0))
                .unwrap_or(0);
            let tone = if age > 60 {
                Tone::Bad
            } else if age > 30 {
                Tone::Warn
            } else {
                Tone::Normal
            };
            Ok(vec![
                Cell::text(txt(r, 0)?),
                Cell::text(date),
                Cell::text(txt(r, 2)?),
                Cell::num(age, format!("{age} d"), tone),
                Cell::money(total),
                Cell::money_tone(received, Tone::Muted),
                Cell::money_tone((total - received).max(0), Tone::Bad),
            ])
        },
    )?;

    let customers: HashSet<&str> = state.rows.iter().map(|r| r.cells[2].text.as_str()).collect();
    let oldest = state.rows.iter().filter_map(|r| r.cells[3].value).max().unwrap_or(0);
    let overdue60: i64 = state
        .rows
        .iter()
        .filter(|r| r.cells[3].value.unwrap_or(0) > 60)
        .filter_map(|r| r.cells[6].value)
        .sum();

    state.metrics = vec![
        Metric::new("Invoices due", state.rows.len().to_string(), format!("Oldest {oldest} days")),
        Metric::new("Outstanding", money(col_sum(&state.rows, 6)), "Total receivable"),
        Metric::new("Customers with dues", customers.len().to_string(), "Distinct customers"),
        Metric::new("Over 60 days", money(overdue60), "Needs follow-up"),
    ];
    Ok(())
}

fn load_payments(connection: &Connection, state: &mut ReportState) -> Res {
    state.cols = vec![
        Col::text("Payment date"),
        Col::text("Invoice no."),
        Col::text("Customer"),
        Col::text("Mode"),
        Col::text("Reference"),
        Col::money("Amount"),
    ];
    let (from, to) = (state.from_date.clone(), state.to_date.clone());

    state.rows = query_rows(
        connection,
        "SELECT p.payment_date, i.invoice_no, i.customer_name_snap,
                p.payment_mode, p.reference_no, p.amount_paise
         FROM sales_payments p
         JOIN sales_invoices i ON i.id = p.invoice_id
         WHERE i.status = 'posted' AND p.payment_date BETWEEN ?1 AND ?2
         ORDER BY p.payment_date DESC, p.id DESC",
        &[&from, &to],
        |r| {
            Ok(vec![
                Cell::text(txt(r, 0)?),
                Cell::text(txt(r, 1)?),
                Cell::text(txt(r, 2)?),
                Cell::text(txt(r, 3)?),
                Cell::text(txt(r, 4)?),
                Cell::money_tone(int(r, 5)?, Tone::Good),
            ])
        },
    )?;

    let total = col_sum(&state.rows, 5);
    let n = state.rows.len() as i64;
    let mut by_mode: HashMap<String, i64> = HashMap::new();
    for row in &state.rows {
        *by_mode.entry(row.cells[3].text.clone()).or_default() += row.cells[5].value.unwrap_or(0);
    }
    let top = by_mode
        .iter()
        .max_by_key(|(_, v)| **v)
        .map(|(k, v)| (k.clone(), *v))
        .unwrap_or_else(|| ("—".into(), 0));

    state.metrics = vec![
        Metric::new("Receipts", n.to_string(), format!("{} payment modes", by_mode.len())),
        Metric::new("Collected amount", money(total), "In selected period"),
        Metric::new("Average receipt", money(if n > 0 { total / n } else { 0 }), "Per payment"),
        Metric::new("Top mode", top.0, format!("{} collected", money(top.1))),
    ];
    Ok(())
}

fn load_gst(connection: &Connection, state: &mut ReportState) -> Res {
    state.cols = vec![
        Col::text("HSN"),
        Col::money("Taxable value"),
        Col::money("CGST"),
        Col::money("SGST"),
        Col::money("IGST"),
        Col::money("Total tax"),
        Col::num("Effective rate"),
    ];
    let (from, to) = (state.from_date.clone(), state.to_date.clone());

    // Every column is qualified with its table alias: both sales_invoice_items and
    // sales_invoices carry *_paise columns, so unqualified names are ambiguous.
    state.rows = query_rows(
        connection,
        "SELECT COALESCE(NULLIF(TRIM(si.hsn_snap), ''), 'Unspecified') AS hsn,
                COALESCE(SUM(si.taxable_paise), 0),
                COALESCE(SUM(si.cgst_paise), 0),
                COALESCE(SUM(si.sgst_paise), 0),
                COALESCE(SUM(si.igst_paise), 0)
         FROM sales_invoice_items AS si
         JOIN sales_invoices AS s ON s.id = si.invoice_id
         WHERE s.status = 'posted'
           AND s.invoice_date BETWEEN ?1 AND ?2
         GROUP BY hsn
         ORDER BY hsn",
        &[&from, &to],
        |r| {
            let taxable = int(r, 1)?;
            let (cgst, sgst, igst) = (int(r, 2)?, int(r, 3)?, int(r, 4)?);
            let tax = cgst + sgst + igst;
            let rate_bp = if taxable > 0 { tax * 10_000 / taxable } else { 0 };
            Ok(vec![
                Cell::text(txt(r, 0)?),
                Cell::money(taxable),
                Cell::money(cgst),
                Cell::money(sgst),
                Cell::money(igst),
                Cell::money(tax),
                Cell::num(rate_bp, format!("{:.2}%", rate_bp as f64 / 100.0), Tone::Muted),
            ])
        },
    )?;

    let taxable = col_sum(&state.rows, 1);
    let (cgst, sgst, igst) = (col_sum(&state.rows, 2), col_sum(&state.rows, 3), col_sum(&state.rows, 4));
    let tax = cgst + sgst + igst;

    state.metrics = vec![
        Metric::new("Taxable sales", money(taxable), format!("{} HSN codes", state.rows.len())),
        Metric::new("Total GST", money(tax), format!("Effective {}", percent(tax, taxable))),
        Metric::new("CGST + SGST", money(cgst + sgst), format!("CGST {} · SGST {}", money(cgst), money(sgst))),
        Metric::new("IGST", money(igst), "Inter-state supplies"),
    ];
    Ok(())
}

// ── Formatting & date utilities ────────────────────────────────────────────
fn money(paise: i64) -> String {
    format_inr(paise as f64 / 100.0)
}

fn qty_text(milli: i64) -> String {
    let value = milli as f64 / 1000.0;
    if milli % 1000 == 0 {
        format!("{value:.0}")
    } else {
        format!("{value:.3}")
    }
}

fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        _ => 28,
    }
}

fn parse_ymd(value: &str) -> Option<(i32, u32, u32)> {
    let parts: Vec<&str> = value.trim().split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 {
        return None;
    }
    let year = parts[0].parse::<i32>().ok()?;
    let month = parts[1].parse::<u32>().ok()?;
    let day = parts[2].parse::<u32>().ok()?;
    if !(1..=12).contains(&month) || !(1..=days_in_month(year, month)).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

fn valid_date_range(from: &str, to: &str) -> bool {
    parse_ymd(from).is_some() && parse_ymd(to).is_some() && from.trim() <= to.trim()
}

/// Days since 1970-01-01 for a civil date.
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = year as i64 - if month <= 2 { 1 } else { 0 };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = month as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = (yoe + era * 400) as i32 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

/// Today's day number in IST (UTC+05:30).
fn today_days() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
        + 19_800;
    secs.div_euclid(86_400)
}

fn today_string() -> String {
    let (y, m, d) = civil_from_days(today_days());
    format!("{y}-{m:02}-{d:02}")
}