use crate::app::AsterApp;
use crate::utils::currency::format_inr;
use eframe::egui::{
    self, Align, Color32, CornerRadius, Id, Label, Layout, RichText, ScrollArea, Sense, Stroke,
};
use rusqlite::{params, Connection, Row};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

// ───────────────────────────── palette ─────────────────────────────

const INK: Color32 = Color32::from_rgb(43, 39, 52);
const MUTED: Color32 = Color32::from_rgb(105, 99, 116);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const PURPLE_TINT: Color32 = Color32::from_rgb(241, 237, 248);
const BORDER: Color32 = Color32::from_rgb(226, 223, 232);
const GREEN: Color32 = Color32::from_rgb(38, 125, 83);
const RED: Color32 = Color32::from_rgb(177, 57, 57);
const RED_TINT: Color32 = Color32::from_rgb(253, 244, 244);
const RED_BORDER: Color32 = Color32::from_rgb(236, 205, 205);
const AMBER: Color32 = Color32::from_rgb(176, 112, 20);
const BLUE: Color32 = Color32::from_rgb(52, 98, 170);
const WHITE: Color32 = Color32::WHITE;

// Financial-year month order (April → March).
const FY_MONTHS: [(&str, u32); 12] = [
    ("April", 4),
    ("May", 5),
    ("June", 6),
    ("July", 7),
    ("August", 8),
    ("September", 9),
    ("October", 10),
    ("November", 11),
    ("December", 12),
    ("January", 1),
    ("February", 2),
    ("March", 3),
];

// ───────────────────────────── types ─────────────────────────────

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

    fn heading(self) -> &'static str {
        match self {
            Self::Gst => "Output tax by HSN code",
            other => other.title(),
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
            Self::Sales => "Posted sales invoices within the selected period",
            Self::Purchases => "Posted purchase documents within the selected period",
            Self::Stock => "Current batch-wise stock, expiry and inventory valuation",
            Self::Receivables => "Outstanding balances with ageing, by invoice date",
            Self::Payments => "Customer receipts recorded within the selected period",
            Self::Gst => "Output tax by HSN, input tax credit and net GST position",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ColKind {
    Text,
    Money,  // paise, summed in footer
    Rate,   // paise, not summed
    Qty,    // milli-units, summed in footer
    Count,  // integer, summed in footer
    Number, // any other number, not summed
}

impl ColKind {
    fn right_aligned(self) -> bool {
        self != Self::Text
    }

    fn summable(self) -> bool {
        matches!(self, Self::Money | Self::Qty | Self::Count)
    }

    fn min_width(self) -> f32 {
        match self {
            Self::Text => 70.0,
            Self::Money | Self::Rate => 100.0,
            Self::Qty => 85.0,
            Self::Count | Self::Number => 70.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    Normal,
    Good,
    Warn,
    Bad,
    Muted,
}

impl Tone {
    fn color(self) -> Color32 {
        match self {
            Self::Normal => INK,
            Self::Good => GREEN,
            Self::Warn => AMBER,
            Self::Bad => RED,
            Self::Muted => MUTED,
        }
    }
}

#[derive(Clone)]
struct Column {
    title: &'static str,
    kind: ColKind,
}

#[derive(Clone)]
struct Cell {
    text: String,
    num: Option<f64>,
    tone: Tone,
}

impl Cell {
    fn text(value: impl Into<String>) -> Self {
        let text: String = value.into();
        let text = if text.trim().is_empty() { "—".to_owned() } else { text };
        Self { text, num: None, tone: Tone::Normal }
    }

    fn money(paise: i64) -> Self {
        Self { text: money(paise), num: Some(paise as f64), tone: Tone::Normal }
    }

    fn qty(milli: i64) -> Self {
        Self {
            text: format!("{:.3}", milli as f64 / 1000.0),
            num: Some(milli as f64),
            tone: Tone::Normal,
        }
    }

    fn count(value: i64) -> Self {
        Self { text: value.to_string(), num: Some(value as f64), tone: Tone::Normal }
    }

    fn number(text: String, value: f64) -> Self {
        Self { text, num: Some(value), tone: Tone::Normal }
    }

    fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

#[derive(Clone)]
struct ReportRow {
    cells: Vec<Cell>,
}

#[derive(Clone)]
struct Metric {
    label: String,
    value: String,
    note: String,
    accent: Color32,
}

fn metric(label: &str, value: impl Into<String>, note: impl Into<String>, accent: Color32) -> Metric {
    Metric { label: label.to_owned(), value: value.into(), note: note.into(), accent }
}

#[derive(Clone)]
struct Breakdown {
    title: String,
    items: Vec<(String, i64)>,
    total: i64,
}

#[derive(Clone)]
struct ReportError {
    summary: String,
    detail: String,
}

impl ReportError {
    fn user(summary: &str) -> Self {
        Self { summary: summary.to_owned(), detail: String::new() }
    }

    fn technical(kind: ReportKind, detail: String) -> Self {
        Self {
            summary: format!(
                "The {} report could not be generated. Please try again. If the problem continues, \
                 share the technical details below with support.",
                kind.title().to_lowercase()
            ),
            detail,
        }
    }
}

#[derive(Clone, Default)]
struct ReportState {
    year: i32,
    kind: ReportKind,
    from_date: String,
    to_date: String,
    search: String,
    columns: Vec<Column>,
    rows: Vec<ReportRow>,
    metrics: Vec<Metric>,
    breakdown: Option<Breakdown>,
    footnote: String,
    error: Option<ReportError>,
    notice: String,
    notice_is_error: bool,
    loaded: bool,
    sort_col: Option<usize>,
    sort_desc: bool,
    page: usize,
    page_size: usize,
    from_text: String,
    to_text: String,
    picker: Option<PickerTarget>,
    picker_anchor: Option<egui::Rect>,
    picker_year: i32,
    picker_month: u32,
}

impl ReportState {
    fn for_year(year: i32) -> Self {
        let (from_date, to_date) = fy_range(year);
        Self {
            year,
            from_text: display_date(&from_date),
            to_text: display_date(&to_date),
            from_date,
            to_date,
            page_size: 50,
            ..Default::default()
        }
    }
}

// ───────────────────────────── entry point ─────────────────────────────

pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let id = Id::new("aster_reports_workspace_v2");
    let mut state = ui
        .ctx()
        .data_mut(|data| data.get_temp::<ReportState>(id).unwrap_or_default());

    if state.year != app.selected_year {
        let kind = state.kind;
        state = ReportState::for_year(app.selected_year);
        state.kind = kind;
    }

    if !state.loaded && state.error.is_none() {
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

            if state.error.is_some() {
                draw_error(ui, app, &mut state);
            } else {
                draw_metrics(ui, &state);
                draw_breakdown(ui, &state);
                ui.add_space(16.0);
                draw_table(ui, &mut state);
            }

            ui.add_space(24.0);
        });

    ui.ctx().data_mut(|data| data.insert_temp(id, state));
}

// ───────────────────────────── ui helpers ─────────────────────────────

fn card(ui: &egui::Ui, margin: f32) -> egui::Frame {
    egui::Frame::group(ui.style())
        .fill(WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(margin)
}

fn primary_button(text: &str) -> impl egui::Widget {
    egui::Button::new(RichText::new(text).size(12.5).strong().color(WHITE))
        .fill(PURPLE)
        .stroke(Stroke::new(1.0_f32, PURPLE))
        .corner_radius(CornerRadius::same(8))
        .min_size(egui::vec2(0.0, 30.0))
}

fn secondary_button(text: &str) -> impl egui::Widget {
    egui::Button::new(RichText::new(text).size(12.5).color(INK))
        .fill(WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(8))
        .min_size(egui::vec2(0.0, 30.0))
}

fn chip(ui: &mut egui::Ui, text: &str) -> bool {
    ui.add(
        egui::Button::new(RichText::new(text).size(12.0).strong().color(PURPLE))
            .fill(PURPLE_TINT)
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(12))
            .min_size(egui::vec2(0.0, 26.0)),
    )
    .clicked()
}

fn draw_header(ui: &mut egui::Ui, app: &AsterApp, state: &mut ReportState) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("BUSINESS INTELLIGENCE")
                    .size(11.0)
                    .strong()
                    .color(PURPLE),
            );
            ui.add_space(3.0);
            ui.heading(RichText::new("Reports").size(28.0).color(INK).strong());
            ui.label(
                RichText::new(format!(
                    "Financial year {}–{}  ·  {}",
                    app.selected_year,
                    app.selected_year + 1,
                    state.kind.description()
                ))
                .size(12.5)
                .color(MUTED),
            );
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.add(primary_button("Export CSV")).clicked() {
                match export_csv(state) {
                    Ok(path) => {
                        state.notice = format!("Exported to {path}");
                        state.notice_is_error = false;
                    }
                    Err(error) => {
                        state.notice = error;
                        state.notice_is_error = true;
                    }
                }
            }
            ui.add_space(6.0);
            if ui.add(secondary_button("↻  Refresh")).clicked() {
                load_report(app, state);
            }
        });
    });

    if !state.notice.is_empty() {
        ui.add_space(6.0);
        let (color, prefix) = if state.notice_is_error { (RED, "!") } else { (GREEN, "✓") };
        ui.label(
            RichText::new(format!("{prefix}  {}", state.notice))
                .size(12.0)
                .color(color),
        );
    }
}

fn draw_report_selector(ui: &mut egui::Ui, app: &AsterApp, state: &mut ReportState) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for kind in ReportKind::ALL {
            let selected = state.kind == kind;
            let button = egui::Button::new(
                RichText::new(kind.title())
                    .size(12.5)
                    .strong()
                    .color(if selected { WHITE } else { INK }),
            )
            .fill(if selected { PURPLE } else { WHITE })
            .stroke(Stroke::new(1.0_f32, if selected { PURPLE } else { BORDER }))
            .corner_radius(CornerRadius::same(16))
            .min_size(egui::vec2(0.0, 30.0));

            if ui.add(button).on_hover_text(kind.description()).clicked() && state.kind != kind {
                state.kind = kind;
                state.search.clear();
                state.sort_col = None;
                state.sort_desc = false;
                load_report(app, state);
            }
        }
    });
}

fn draw_filters(ui: &mut egui::Ui, app: &AsterApp, state: &mut ReportState) {
    let fy = state.year;
    let mut preset: Option<(String, String)> = None;

    card(ui, 12.0).show(ui, |ui| {
        ui.set_min_width(ui.available_width());

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("PERIOD").size(10.5).strong().color(MUTED));
            if chip(ui, "Full year") {
                preset = Some(fy_range(fy));
            }
            for (index, label) in ["Q1", "Q2", "Q3", "Q4"].iter().enumerate() {
                if chip(ui, label) {
                    preset = Some(quarter_range(fy, index));
                }
            }
            egui::ComboBox::from_id_salt("aster_reports_month")
                .selected_text("Month")
                .width(110.0)
                .show_ui(ui, |ui| {
                    for (name, month) in FY_MONTHS {
                        if ui.selectable_label(false, name).clicked() {
                            preset = Some(month_range(fy, month));
                        }
                    }
                });
        });

        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("From").size(12.0).color(MUTED));
            let from_btn = date_button(ui, &state.from_text, state.picker == Some(PickerTarget::From));
            if from_btn.clicked() {
                open_picker(state, PickerTarget::From, from_btn.rect);
            }

            ui.add_space(4.0);
            ui.label(RichText::new("To").size(12.0).color(MUTED));
            let to_btn = date_button(ui, &state.to_text, state.picker == Some(PickerTarget::To));
            if to_btn.clicked() {
                open_picker(state, PickerTarget::To, to_btn.rect);
            }

            ui.separator();

            ui.label(RichText::new("Search").size(12.0).color(MUTED));
            let search = ui.add(
                egui::TextEdit::singleline(&mut state.search)
                    .desired_width(210.0)
                    .hint_text("Filter rows in this report"),
            );
            if search.changed() {
                state.page = 0;
            }
            if !state.search.is_empty() && ui.small_button("✕").on_hover_text("Clear search").clicked() {
                state.search.clear();
                state.page = 0;
            }
        });

        let note = match state.kind {
            ReportKind::Stock => Some("Stock is a live snapshot; the period does not change quantities."),
            ReportKind::Receivables => Some("Age is counted in days from the invoice date to today."),
            _ => None,
        };
        if let Some(note) = note {
            ui.add_space(4.0);
            ui.label(RichText::new(note).size(11.0).color(MUTED));
        }
    });

    let picked = draw_date_picker(ui.ctx(), state);

    if let Some((from, to)) = preset {
        state.from_text = display_date(&from);
        state.to_text = display_date(&to);
        state.from_date = from;
        state.to_date = to;
        state.picker = None;
        load_report(app, state);
    } else if let Some((target, iso)) = picked {
        match target {
            PickerTarget::From => {
                state.from_date = iso.clone();
                if state.to_date < iso {
                    state.to_date = iso;
                }
            }
            PickerTarget::To => {
                state.to_date = iso.clone();
                if state.from_date > iso {
                    state.from_date = iso;
                }
            }
        }
        state.from_text = display_date(&state.from_date);
        state.to_text = display_date(&state.to_date);
        state.picker = None;
        load_report(app, state);
    }
}

fn draw_error(ui: &mut egui::Ui, app: &AsterApp, state: &mut ReportState) {
    let mut retry = false;

    egui::Frame::group(ui.style())
        .fill(RED_TINT)
        .stroke(Stroke::new(1.0_f32, RED_BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new("Unable to load this report").size(15.0).strong().color(RED));
            ui.add_space(4.0);

            if let Some(error) = &state.error {
                ui.label(RichText::new(&error.summary).size(12.5).color(INK));
                if !error.detail.is_empty() {
                    ui.add_space(6.0);
                    egui::CollapsingHeader::new(RichText::new("Technical details").size(11.5).color(MUTED))
                        .id_salt("aster_reports_error_details")
                        .show(ui, |ui| {
                            ui.add(
                                Label::new(
                                    RichText::new(&error.detail).monospace().size(11.0).color(MUTED),
                                )
                                .selectable(true),
                            );
                        });
                }
            }

            ui.add_space(10.0);
            if ui.add(primary_button("Try again")).clicked() {
                retry = true;
            }
        });

    if retry {
        load_report(app, state);
    }
}

fn draw_metrics(ui: &mut egui::Ui, state: &ReportState) {
    let count = state.metrics.len();
    if count == 0 {
        return;
    }

    ui.columns(count, |columns| {
        for (metric, column) in state.metrics.iter().zip(columns.iter_mut()) {
            card(column, 14.0).show(column, |ui| {
                ui.set_min_width(ui.available_width());
                ui.set_min_height(88.0);

                let (bar, _) = ui.allocate_exact_size(egui::vec2(28.0, 3.0), Sense::hover());
                ui.painter().rect_filled(bar, CornerRadius::same(2), metric.accent);
                ui.add_space(6.0);

                ui.label(RichText::new(metric.label.to_uppercase()).size(10.5).strong().color(MUTED));
                ui.add_space(2.0);
                ui.label(RichText::new(&metric.value).size(22.0).strong().color(INK));
                if !metric.note.is_empty() {
                    ui.label(RichText::new(&metric.note).size(11.0).color(MUTED));
                }
            });
        }
    });
}

fn draw_breakdown(ui: &mut egui::Ui, state: &ReportState) {
    let Some(breakdown) = &state.breakdown else {
        return;
    };
    if breakdown.items.is_empty() {
        return;
    }

    ui.add_space(14.0);
    card(ui, 14.0).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(RichText::new(&breakdown.title).size(14.0).strong().color(INK));
        ui.add_space(10.0);

        let max = breakdown.items.iter().map(|item| item.1).max().unwrap_or(1).max(1);

        for (name, value) in &breakdown.items {
            let share = if breakdown.total > 0 {
                *value as f64 * 100.0 / breakdown.total as f64
            } else {
                0.0
            };

            ui.horizontal(|ui| {
                ui.label(RichText::new(name).size(12.0).color(INK));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{}   ·   {:.1}%", money(*value), share))
                            .size(11.5)
                            .color(MUTED),
                    );
                });
            });

            let (track, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), Sense::hover());
            ui.painter().rect_filled(track, CornerRadius::same(3), PURPLE_TINT);
            if *value > 0 {
                let fraction = (*value as f32 / max as f32).clamp(0.015, 1.0);
                let mut fill = track;
                fill.set_width(track.width() * fraction);
                ui.painter().rect_filled(fill, CornerRadius::same(3), PURPLE);
            }
            ui.add_space(8.0);
        }
    });
}

fn draw_cell(ui: &mut egui::Ui, cell: &Cell, kind: ColKind) {
    let mut text = RichText::new(&cell.text).size(12.0).color(cell.tone.color());
    if cell.tone != Tone::Normal && cell.tone != Tone::Muted {
        text = text.strong();
    }
    if kind.right_aligned() {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.set_min_width(kind.min_width());
            ui.label(text);
        });
    } else {
        ui.label(text);
    }
}

fn draw_table(ui: &mut egui::Ui, state: &mut ReportState) {
    let mut click_col: Option<usize> = None;
    let mut page_req: Option<usize> = None;
    let mut size_req: Option<usize> = None;

    {
        let rows = visible_rows(state);
        let total_rows = rows.len();
        let page_size = state.page_size.max(1);
        let pages = total_rows.div_ceil(page_size).max(1);
        let page = state.page.min(pages - 1);
        let start = page * page_size;
        let end = (start + page_size).min(total_rows);

        ui.horizontal(|ui| {
            ui.heading(RichText::new(state.kind.heading()).size(17.0).strong().color(INK));
            let label = if state.search.trim().is_empty() {
                format!("{total_rows} records")
            } else {
                format!("{total_rows} of {} records", state.rows.len())
            };
            ui.label(RichText::new(label).size(11.5).color(MUTED));

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add_enabled(page + 1 < pages, secondary_button("Next ›")).clicked() {
                    page_req = Some(page + 1);
                }
                ui.label(
                    RichText::new(format!("Page {} of {}", page + 1, pages))
                        .size(11.5)
                        .color(MUTED),
                );
                if ui.add_enabled(page > 0, secondary_button("‹ Prev")).clicked() {
                    page_req = Some(page - 1);
                }
                ui.add_space(10.0);
                egui::ComboBox::from_id_salt("aster_reports_page_size")
                    .selected_text(format!("{page_size} / page"))
                    .width(96.0)
                    .show_ui(ui, |ui| {
                        for size in [25usize, 50, 100, 250] {
                            if ui.selectable_label(page_size == size, format!("{size} rows")).clicked() {
                                size_req = Some(size);
                            }
                        }
                    });
            });
        });
        ui.add_space(8.0);

        card(ui, 10.0).show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            if state.columns.is_empty() {
                ui.label(RichText::new("No report columns are available.").color(MUTED));
                return;
            }

            ScrollArea::horizontal()
                .id_salt("aster_reports_horizontal")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    egui::Grid::new("aster_reports_table")
                        .striped(true)
                        .min_col_width(70.0)
                        .spacing([18.0, 9.0])
                        .show(ui, |ui| {
                            // header (click to sort)
                            for (index, column) in state.columns.iter().enumerate() {
                                let active = state.sort_col == Some(index);
                                let arrow = if active {
                                    if state.sort_desc { "  ▼" } else { "  ▲" }
                                } else {
                                    ""
                                };
                                let text = RichText::new(format!("{}{}", column.title.to_uppercase(), arrow))
                                    .size(10.5)
                                    .strong()
                                    .color(if active { PURPLE } else { MUTED });
                                let label = Label::new(text).sense(Sense::click());

                                let response = if column.kind.right_aligned() {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.set_min_width(column.kind.min_width());
                                        ui.add(label)
                                    })
                                    .inner
                                } else {
                                    ui.add(label)
                                };

                                if response
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .on_hover_text("Click to sort")
                                    .clicked()
                                {
                                    click_col = Some(index);
                                }
                            }
                            ui.end_row();

                            // current page
                            for row in &rows[start..end] {
                                for (index, cell) in row.cells.iter().enumerate() {
                                    draw_cell(ui, cell, state.columns[index].kind);
                                }
                                ui.end_row();
                            }

                            // totals over every filtered row
                            if total_rows > 0 {
                                for (index, column) in state.columns.iter().enumerate() {
                                    let text = if index == 0 {
                                        "TOTAL".to_owned()
                                    } else if column.kind.summable() {
                                        let sum: f64 = rows
                                            .iter()
                                            .map(|row| row.cells[index].num.unwrap_or(0.0))
                                            .sum();
                                        match column.kind {
                                            ColKind::Money => money(sum as i64),
                                            ColKind::Qty => format!("{:.3}", sum / 1000.0),
                                            _ => (sum as i64).to_string(),
                                        }
                                    } else {
                                        String::new()
                                    };
                                    let rich = RichText::new(text).size(12.0).strong().color(PURPLE);
                                    if column.kind.right_aligned() {
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            ui.set_min_width(column.kind.min_width());
                                            ui.label(rich);
                                        });
                                    } else {
                                        ui.label(rich);
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });

            if total_rows == 0 {
                ui.add_space(16.0);
                ui.vertical_centered(|ui| {
                    let headline = if state.search.trim().is_empty() {
                        "No records for this period".to_owned()
                    } else {
                        format!("No records match “{}”", state.search.trim())
                    };
                    ui.label(RichText::new(headline).size(14.0).strong().color(INK));
                    ui.label(
                        RichText::new("Try a different date range, a period preset or clear the search.")
                            .size(11.5)
                            .color(MUTED),
                    );
                });
                ui.add_space(16.0);
            }
        });

        if !state.footnote.is_empty() {
            ui.add_space(6.0);
            ui.label(RichText::new(&state.footnote).size(11.0).color(MUTED));
        }
    }

    if let Some(column) = click_col {
        if state.sort_col == Some(column) {
            if state.sort_desc {
                state.sort_col = None;
                state.sort_desc = false;
            } else {
                state.sort_desc = true;
            }
        } else {
            state.sort_col = Some(column);
            state.sort_desc = false;
        }
        state.page = 0;
    }
    if let Some(page) = page_req {
        state.page = page;
    }
    if let Some(size) = size_req {
        state.page_size = size;
        state.page = 0;
    }
}

// ───────────────────────────── row filtering / sorting ─────────────────────────────

fn visible_rows(state: &ReportState) -> Vec<&ReportRow> {
    let search = state.search.trim().to_lowercase();

    let mut rows: Vec<&ReportRow> = state
        .rows
        .iter()
        .filter(|row| {
            search.is_empty()
                || row
                    .cells
                    .iter()
                    .any(|cell| cell.text.to_lowercase().contains(&search))
        })
        .collect();

    if let Some(column) = state.sort_col {
        rows.sort_by(|a, b| {
            let (Some(x), Some(y)) = (a.cells.get(column), b.cells.get(column)) else {
                return Ordering::Equal;
            };
            let order = match (x.num, y.num) {
                (Some(p), Some(q)) => p.partial_cmp(&q).unwrap_or(Ordering::Equal),
                _ => x.text.to_lowercase().cmp(&y.text.to_lowercase()),
            };
            if state.sort_desc { order.reverse() } else { order }
        });
    }

    rows
}

// ───────────────────────────── CSV export ─────────────────────────────

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn csv_line(values: impl Iterator<Item = String>) -> String {
    let mut line = values.map(|value| csv_escape(&value)).collect::<Vec<_>>().join(",");
    line.push_str("\r\n");
    line
}

fn csv_value(cell: &Cell, kind: ColKind) -> String {
    match (kind, cell.num) {
        (ColKind::Money | ColKind::Rate, Some(n)) => format!("{:.2}", n / 100.0),
        (ColKind::Qty, Some(n)) => format!("{:.3}", n / 1000.0),
        _ => cell.text.clone(),
    }
}

fn export_csv(state: &ReportState) -> Result<String, String> {
    if state.columns.is_empty() {
        return Err("Export failed: there is nothing to export.".into());
    }
    let rows = visible_rows(state);
    if rows.is_empty() {
        return Err("Export failed: there are no rows to export.".into());
    }

    // BOM so Excel opens the file as UTF-8.
    let mut out = String::from("\u{feff}");
    out.push_str(&csv_line(state.columns.iter().map(|c| c.title.to_owned())));
    for row in rows {
        out.push_str(&csv_line(
            row.cells
                .iter()
                .zip(&state.columns)
                .map(|(cell, column)| csv_value(cell, column.kind)),
        ));
    }

    let dir = std::path::Path::new("exports");
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Export failed: could not create the exports folder ({e})."))?;
    let path = dir.join(format!(
        "{}_{}_to_{}.csv",
        state.kind.slug(),
        state.from_date,
        state.to_date
    ));
    std::fs::write(&path, out).map_err(|e| format!("Export failed: could not write the file ({e})."))?;

    Ok(path.display().to_string())
}

// ───────────────────────────── loading ─────────────────────────────

fn load_report(app: &AsterApp, state: &mut ReportState) {
    state.error = None;
    state.notice.clear();
    state.rows.clear();
    state.columns.clear();
    state.metrics.clear();
    state.breakdown = None;
    state.footnote.clear();
    state.loaded = false;
    state.page = 0;
    state.from_date = state.from_date.trim().to_owned();
    state.to_date = state.to_date.trim().to_owned();

    if !valid_date_range(&state.from_date, &state.to_date) {
        state.error = Some(ReportError::user(
            "Enter valid dates in DD-MM-YYYY format. The From date must not be after the To date.",
        ));
        return;
    }

    let Some(db) = app.db.as_ref() else {
        state.error = Some(ReportError::user("The database is not available."));
        return;
    };

    let connection = match db.year_connection(app.selected_year) {
        Ok(connection) => connection,
        Err(error) => {
            state.error = Some(ReportError::technical(state.kind, error));
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

    match result {
        Ok(()) => {
            state.loaded = true;
            if state.sort_col.is_some_and(|c| c >= state.columns.len()) {
                state.sort_col = None;
            }
        }
        Err(detail) => {
            state.rows.clear();
            state.columns.clear();
            state.metrics.clear();
            state.breakdown = None;
            state.error = Some(ReportError::technical(state.kind, detail));
        }
    }
}

fn cols(list: &[(&'static str, ColKind)]) -> Vec<Column> {
    list.iter().map(|&(title, kind)| Column { title, kind }).collect()
}

fn text_at(row: &Row<'_>, index: usize) -> rusqlite::Result<String> {
    Ok(row.get::<_, Option<String>>(index)?.unwrap_or_default())
}

fn int_at(row: &Row<'_>, index: usize) -> rusqlite::Result<i64> {
    Ok(row.get::<_, Option<i64>>(index)?.unwrap_or(0))
}

fn query_rows<P, F>(
    connection: &Connection,
    what: &str,
    sql: &str,
    args: P,
    mut map: F,
) -> Result<Vec<ReportRow>, String>
where
    P: rusqlite::Params,
    F: FnMut(&Row<'_>) -> rusqlite::Result<Vec<Cell>>,
{
    let mut statement = connection
        .prepare(sql)
        .map_err(|e| format!("{what}: could not prepare query: {e}\n\nSQL:\n{sql}"))?;

    let rows = statement
        .query_map(args, |row| map(row).map(|cells| ReportRow { cells }))
        .map_err(|e| format!("{what}: query failed: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{what}: could not read rows: {e}"))
}

fn sum_col(rows: &[ReportRow], index: usize) -> i64 {
    rows.iter()
        .map(|row| row.cells.get(index).and_then(|c| c.num).unwrap_or(0.0))
        .sum::<f64>() as i64
}

fn group_sum(rows: &[ReportRow], key: usize, value: usize) -> Vec<(String, i64)> {
    let mut map: HashMap<String, i64> = HashMap::new();
    for row in rows {
        let amount = row.cells[value].num.unwrap_or(0.0) as i64;
        *map.entry(row.cells[key].text.clone()).or_insert(0) += amount;
    }
    let mut items: Vec<(String, i64)> = map.into_iter().collect();
    items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    items
}

fn distinct(rows: &[ReportRow], index: usize) -> usize {
    rows.iter()
        .map(|row| row.cells[index].text.as_str())
        .collect::<HashSet<_>>()
        .len()
}

fn percent(part: i64, whole: i64) -> f64 {
    if whole > 0 { part as f64 * 100.0 / whole as f64 } else { 0.0 }
}

fn average(total: i64, count: usize) -> i64 {
    if count > 0 { total / count as i64 } else { 0 }
}

fn set_breakdown(state: &mut ReportState, title: &str, mut items: Vec<(String, i64)>, total: i64, limit: usize) {
    items.truncate(limit);
    state.breakdown = Some(Breakdown { title: title.to_owned(), items, total });
}

fn load_sales(connection: &Connection, state: &mut ReportState) -> Result<(), String> {
    use ColKind::{Money, Text};
    let (from, to) = (state.from_date.clone(), state.to_date.clone());

    state.columns = cols(&[
        ("Invoice no.", Text),
        ("Date", Text),
        ("Customer", Text),
        ("Taxable", Money),
        ("CGST", Money),
        ("SGST", Money),
        ("IGST", Money),
        ("Total", Money),
        ("Received", Money),
        ("Due", Money),
        ("Status", Text),
    ]);

    state.rows = query_rows(
        connection,
        "Sales report",
        "SELECT invoice_no, invoice_date, customer_name_snap,
                taxable_paise, cgst_paise, sgst_paise, igst_paise,
                total_paise, received_paise
         FROM sales_invoices
         WHERE status = 'posted'
           AND invoice_date BETWEEN ?1 AND ?2
         ORDER BY invoice_date DESC, id DESC",
        params![from, to],
        |r| {
            let total = int_at(r, 7)?;
            let received = int_at(r, 8)?;
            let due = (total - received).max(0);
            let (label, tone) = if due == 0 {
                ("Paid", Tone::Good)
            } else if received > 0 {
                ("Partial", Tone::Warn)
            } else {
                ("Unpaid", Tone::Bad)
            };
            Ok(vec![
                Cell::text(text_at(r, 0)?),
                Cell::text(text_at(r, 1)?),
                Cell::text(text_at(r, 2)?),
                Cell::money(int_at(r, 3)?),
                Cell::money(int_at(r, 4)?),
                Cell::money(int_at(r, 5)?),
                Cell::money(int_at(r, 6)?),
                Cell::money(total),
                Cell::money(received),
                Cell::money(due).tone(if due > 0 { Tone::Bad } else { Tone::Muted }),
                Cell::text(label).tone(tone),
            ])
        },
    )?;

    let count = state.rows.len();
    let taxable = sum_col(&state.rows, 3);
    let total = sum_col(&state.rows, 7);
    let received = sum_col(&state.rows, 8);
    let due = sum_col(&state.rows, 9);

    state.metrics = vec![
        metric("Invoices", count.to_string(), format!("Average {}", money(average(total, count))), PURPLE),
        metric("Sales total", money(total), format!("Taxable {}", money(taxable)), BLUE),
        metric(
            "Amount received",
            money(received),
            format!("{:.1}% collected", percent(received, total)),
            GREEN,
        ),
        metric(
            "Outstanding",
            money(due),
            if due > 0 { "Unpaid balance on these invoices" } else { "Everything is settled" },
            if due > 0 { RED } else { GREEN },
        ),
    ];

    let top = group_sum(&state.rows, 2, 7);
    set_breakdown(state, "Top customers by sales", top, total, 6);
    Ok(())
}

fn load_purchases(connection: &Connection, state: &mut ReportState) -> Result<(), String> {
    use ColKind::{Money, Text};
    let (from, to) = (state.from_date.clone(), state.to_date.clone());

    state.columns = cols(&[
        ("Purchase no.", Text),
        ("Date", Text),
        ("Supplier", Text),
        ("Taxable", Money),
        ("CGST", Money),
        ("SGST", Money),
        ("IGST", Money),
        ("Total tax", Money),
        ("Total", Money),
    ]);

    state.rows = query_rows(
        connection,
        "Purchase report",
        "SELECT purchase_no, invoice_date, supplier_name_snap,
                taxable_paise, cgst_paise, sgst_paise, igst_paise, total_paise
         FROM purchase_documents
         WHERE status = 'posted'
           AND invoice_date BETWEEN ?1 AND ?2
         ORDER BY invoice_date DESC, id DESC",
        params![from, to],
        |r| {
            let (cgst, sgst, igst) = (int_at(r, 4)?, int_at(r, 5)?, int_at(r, 6)?);
            Ok(vec![
                Cell::text(text_at(r, 0)?),
                Cell::text(text_at(r, 1)?),
                Cell::text(text_at(r, 2)?),
                Cell::money(int_at(r, 3)?),
                Cell::money(cgst),
                Cell::money(sgst),
                Cell::money(igst),
                Cell::money(cgst + sgst + igst),
                Cell::money(int_at(r, 7)?),
            ])
        },
    )?;

    let count = state.rows.len();
    let taxable = sum_col(&state.rows, 3);
    let tax = sum_col(&state.rows, 7);
    let total = sum_col(&state.rows, 8);

    state.metrics = vec![
        metric(
            "Purchase documents",
            count.to_string(),
            format!("Average {}", money(average(total, count))),
            PURPLE,
        ),
        metric("Purchase total", money(total), "Including tax", BLUE),
        metric("Taxable value", money(taxable), "Before tax", GREEN),
        metric("Input tax", money(tax), "Available as credit", AMBER),
    ];

    let top = group_sum(&state.rows, 2, 8);
    set_breakdown(state, "Top suppliers by purchase value", top, total, 6);
    Ok(())
}

fn load_stock(connection: &Connection, state: &mut ReportState) -> Result<(), String> {
    use ColKind::{Count as _Count, Money, Number, Qty, Rate, Text};
    let _ = _Count;

    state.columns = cols(&[
        ("Product", Text),
        ("Batch", Text),
        ("Expiry", Text),
        ("Days left", Number),
        ("Qty", Qty),
        ("Purchase rate", Rate),
        ("Stock value", Money),
        ("Status", Text),
    ]);

    state.rows = query_rows(
        connection,
        "Stock report",
        "SELECT p.name, b.batch_no, b.expiry_date,
                b.qty_on_hand_milli, b.purchase_rate_paise,
                CASE WHEN b.expiry_date IS NULL OR b.expiry_date = '' THEN NULL
                     ELSE CAST(julianday(b.expiry_date) - julianday('now') AS INTEGER)
                END AS days_left
         FROM batches b
         JOIN products p ON p.id = b.product_id
         WHERE b.active = 1 AND b.qty_on_hand_milli > 0
         ORDER BY p.name, b.expiry_date, b.batch_no",
        [],
        |r| {
            let qty = int_at(r, 3)?;
            let rate = int_at(r, 4)?;
            let value = ((qty as i128 * rate as i128) / 1000) as i64;
            let days: Option<i64> = r.get::<_, Option<i64>>(5)?;

            let (status, tone) = match days {
                Some(d) if d < 0 => ("Expired", Tone::Bad),
                Some(d) if d <= 90 => ("Expiring soon", Tone::Warn),
                Some(_) => ("In date", Tone::Good),
                None => ("No expiry", Tone::Muted),
            };
            let days_cell = match days {
                Some(d) => Cell::number(d.to_string(), d as f64).tone(if d < 0 {
                    Tone::Bad
                } else if d <= 90 {
                    Tone::Warn
                } else {
                    Tone::Normal
                }),
                None => Cell::text("—"),
            };

            Ok(vec![
                Cell::text(text_at(r, 0)?),
                Cell::text(text_at(r, 1)?),
                Cell::text(text_at(r, 2)?),
                days_cell,
                Cell::qty(qty),
                Cell::money(rate),
                Cell::money(value),
                Cell::text(status).tone(tone),
            ])
        },
    )?;

    let batches = state.rows.len();
    let products = distinct(&state.rows, 0);
    let quantity = sum_col(&state.rows, 4);
    let value = sum_col(&state.rows, 6);
    let expired = state.rows.iter().filter(|r| r.cells[7].text == "Expired").count();
    let expiring = state.rows.iter().filter(|r| r.cells[7].text == "Expiring soon").count();

    state.metrics = vec![
        metric("Active batches", batches.to_string(), format!("{products} products"), PURPLE),
        metric("Quantity on hand", format!("{:.3}", quantity as f64 / 1000.0), "Across all batches", BLUE),
        metric("Stock valuation", money(value), "At purchase rate", GREEN),
        metric(
            "Needs attention",
            (expired + expiring).to_string(),
            format!("{expired} expired · {expiring} within 90 days"),
            if expired > 0 { RED } else if expiring > 0 { AMBER } else { GREEN },
        ),
    ];

    let top = group_sum(&state.rows, 0, 6);
    set_breakdown(state, "Highest-value products in stock", top, value, 6);
    Ok(())
}

fn age_bucket(age: Option<i64>) -> (&'static str, Tone) {
    match age {
        Some(d) if d <= 30 => ("0–30 days", Tone::Good),
        Some(d) if d <= 60 => ("31–60 days", Tone::Warn),
        Some(d) if d <= 90 => ("61–90 days", Tone::Warn),
        Some(_) => ("90+ days", Tone::Bad),
        None => ("Unknown", Tone::Muted),
    }
}

fn load_receivables(connection: &Connection, state: &mut ReportState) -> Result<(), String> {
    use ColKind::{Money, Number, Text};
    let (from, to) = (state.from_date.clone(), state.to_date.clone());

    state.columns = cols(&[
        ("Invoice no.", Text),
        ("Date", Text),
        ("Customer", Text),
        ("Age (days)", Number),
        ("Invoice total", Money),
        ("Received", Money),
        ("Outstanding", Money),
        ("Ageing", Text),
    ]);

    state.rows = query_rows(
        connection,
        "Receivables report",
        "SELECT invoice_no, invoice_date, customer_name_snap,
                total_paise, received_paise,
                CAST(julianday('now') - julianday(invoice_date) AS INTEGER) AS age_days
         FROM sales_invoices
         WHERE status = 'posted'
           AND invoice_date BETWEEN ?1 AND ?2
           AND total_paise > received_paise
         ORDER BY invoice_date, id",
        params![from, to],
        |r| {
            let total = int_at(r, 3)?;
            let received = int_at(r, 4)?;
            let age: Option<i64> = r.get::<_, Option<i64>>(5)?;
            let (bucket, tone) = age_bucket(age);
            let age_cell = match age {
                Some(d) => Cell::number(d.to_string(), d as f64).tone(tone),
                None => Cell::text("—"),
            };
            Ok(vec![
                Cell::text(text_at(r, 0)?),
                Cell::text(text_at(r, 1)?),
                Cell::text(text_at(r, 2)?),
                age_cell,
                Cell::money(total),
                Cell::money(received),
                Cell::money((total - received).max(0)).tone(Tone::Bad),
                Cell::text(bucket).tone(tone),
            ])
        },
    )?;

    let count = state.rows.len();
    let customers = distinct(&state.rows, 2);
    let outstanding = sum_col(&state.rows, 6);

    let mut overdue = 0_i64;
    let mut overdue_count = 0_usize;
    let mut oldest: Option<(i64, String)> = None;
    for row in &state.rows {
        if let Some(age) = row.cells[3].num {
            let age = age as i64;
            if age > 30 {
                overdue += row.cells[6].num.unwrap_or(0.0) as i64;
                overdue_count += 1;
            }
            if oldest.as_ref().is_none_or(|(a, _)| age > *a) {
                oldest = Some((age, row.cells[0].text.clone()));
            }
        }
    }

    state.metrics = vec![
        metric("Invoices due", count.to_string(), format!("{customers} customers"), PURPLE),
        metric("Outstanding", money(outstanding), "Total receivable", RED),
        metric(
            "Older than 30 days",
            money(overdue),
            format!("{overdue_count} invoices"),
            if overdue > 0 { AMBER } else { GREEN },
        ),
        match oldest {
            Some((age, invoice)) => metric("Oldest invoice", format!("{age} days"), invoice, BLUE),
            None => metric("Oldest invoice", "—", "Nothing outstanding", GREEN),
        },
    ];

    let buckets = ["0–30 days", "31–60 days", "61–90 days", "90+ days", "Unknown"];
    let sums: HashMap<String, i64> = group_sum(&state.rows, 7, 6).into_iter().collect();
    let items: Vec<(String, i64)> = buckets
        .iter()
        .filter_map(|b| sums.get(*b).map(|v| ((*b).to_owned(), *v)))
        .collect();
    set_breakdown(state, "Outstanding by ageing bucket", items, outstanding, 5);
    Ok(())
}

fn load_payments(connection: &Connection, state: &mut ReportState) -> Result<(), String> {
    use ColKind::{Money, Text};
    let (from, to) = (state.from_date.clone(), state.to_date.clone());

    state.columns = cols(&[
        ("Payment date", Text),
        ("Invoice no.", Text),
        ("Customer", Text),
        ("Mode", Text),
        ("Reference", Text),
        ("Amount", Money),
    ]);

    state.rows = query_rows(
        connection,
        "Payment report",
        "SELECT p.payment_date, i.invoice_no, i.customer_name_snap,
                p.payment_mode, p.reference_no, p.amount_paise
         FROM sales_payments p
         JOIN sales_invoices i ON i.id = p.invoice_id
         WHERE i.status = 'posted'
           AND p.payment_date BETWEEN ?1 AND ?2
         ORDER BY p.payment_date DESC, p.id DESC",
        params![from, to],
        |r| {
            Ok(vec![
                Cell::text(text_at(r, 0)?),
                Cell::text(text_at(r, 1)?),
                Cell::text(text_at(r, 2)?),
                Cell::text(text_at(r, 3)?),
                Cell::text(text_at(r, 4)?),
                Cell::money(int_at(r, 5)?),
            ])
        },
    )?;

    let count = state.rows.len();
    let total = sum_col(&state.rows, 5);
    let customers = distinct(&state.rows, 2);
    let modes = group_sum(&state.rows, 3, 5);

    let top_mode = modes.first().cloned();
    state.metrics = vec![
        metric("Receipts", count.to_string(), format!("Average {}", money(average(total, count))), PURPLE),
        metric("Collected", money(total), "In the selected period", GREEN),
        match top_mode {
            Some((name, value)) => metric(
                "Top payment mode",
                name,
                format!("{:.1}% of collections", percent(value, total)),
                BLUE,
            ),
            None => metric("Top payment mode", "—", "No receipts", BLUE),
        },
        metric("Paying customers", customers.to_string(), "Distinct customers", AMBER),
    ];

    set_breakdown(state, "Collections by payment mode", modes, total, 6);
    Ok(())
}

fn load_gst(connection: &Connection, state: &mut ReportState) -> Result<(), String> {
    use ColKind::{Money, Number, Text};
    let (from, to) = (state.from_date.clone(), state.to_date.clone());

    state.columns = cols(&[
        ("HSN", Text),
        ("Invoices", Number),
        ("Taxable value", Money),
        ("CGST", Money),
        ("SGST", Money),
        ("IGST", Money),
        ("Total tax", Money),
        ("Effective rate", Number),
    ]);

    // Every column is table-qualified: both sales_invoices and sales_invoice_items
    // carry taxable/cgst/sgst/igst columns, so unqualified names are ambiguous.
    state.rows = query_rows(
        connection,
        "GST report",
        "SELECT COALESCE(NULLIF(TRIM(si.hsn_snap), ''), 'Unspecified') AS hsn,
                COUNT(DISTINCT si.invoice_id),
                COALESCE(SUM(si.taxable_paise), 0),
                COALESCE(SUM(si.cgst_paise), 0),
                COALESCE(SUM(si.sgst_paise), 0),
                COALESCE(SUM(si.igst_paise), 0)
         FROM sales_invoice_items si
         JOIN sales_invoices s ON s.id = si.invoice_id
         WHERE s.status = 'posted'
           AND s.invoice_date BETWEEN ?1 AND ?2
         GROUP BY 1
         ORDER BY 1",
        params![from, to],
        |r| {
            let taxable = int_at(r, 2)?;
            let (cgst, sgst, igst) = (int_at(r, 3)?, int_at(r, 4)?, int_at(r, 5)?);
            let tax = cgst + sgst + igst;
            let rate = percent(tax, taxable);
            Ok(vec![
                Cell::text(text_at(r, 0)?),
                Cell::count(int_at(r, 1)?),
                Cell::money(taxable),
                Cell::money(cgst),
                Cell::money(sgst),
                Cell::money(igst),
                Cell::money(tax),
                Cell::number(format!("{rate:.2}%"), rate),
            ])
        },
    )?;

    // Input tax credit from posted purchases in the same period.
    let (in_cgst, in_sgst, in_igst): (i64, i64, i64) = connection
        .query_row(
            "SELECT COALESCE(SUM(pd.cgst_paise), 0),
                    COALESCE(SUM(pd.sgst_paise), 0),
                    COALESCE(SUM(pd.igst_paise), 0)
             FROM purchase_documents pd
             WHERE pd.status = 'posted'
               AND pd.invoice_date BETWEEN ?1 AND ?2",
            params![from, to],
            |r| Ok((int_at(r, 0)?, int_at(r, 1)?, int_at(r, 2)?)),
        )
        .map_err(|e| format!("GST report: could not read input tax credit: {e}"))?;

    let taxable = sum_col(&state.rows, 2);
    let cgst = sum_col(&state.rows, 3);
    let sgst = sum_col(&state.rows, 4);
    let igst = sum_col(&state.rows, 5);
    let output_tax = cgst + sgst + igst;
    let input_tax = in_cgst + in_sgst + in_igst;
    let net = output_tax - input_tax;

    state.metrics = vec![
        metric("Taxable sales", money(taxable), format!("{} HSN codes", state.rows.len()), BLUE),
        metric(
            "Output tax",
            money(output_tax),
            format!("CGST {} · SGST {} · IGST {}", money(cgst), money(sgst), money(igst)),
            PURPLE,
        ),
        metric(
            "Input tax credit",
            money(input_tax),
            format!("CGST {} · SGST {} · IGST {}", money(in_cgst), money(in_sgst), money(in_igst)),
            GREEN,
        ),
        if net >= 0 {
            metric("Net GST payable", money(net), "Output tax − input credit", if net > 0 { AMBER } else { GREEN })
        } else {
            metric("Input credit surplus", money(-net), "Carry forward", GREEN)
        },
    ];

    set_breakdown(
        state,
        "Output tax composition",
        vec![
            ("CGST".to_owned(), cgst),
            ("SGST".to_owned(), sgst),
            ("IGST".to_owned(), igst),
        ],
        output_tax,
        3,
    );

    state.footnote = "Computed from posted sales and purchase documents in the selected period. \
        Net payable is an estimate (output tax minus input tax credit); reconcile with GSTR-1 / GSTR-3B before filing."
        .to_owned();
    Ok(())
}

// ───────────────────────────── money & dates ─────────────────────────────

fn money(paise: i64) -> String {
    format_inr(paise as f64 / 100.0)
}

fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn fy_range(fy: i32) -> (String, String) {
    (format!("{fy}-04-01"), format!("{}-03-31", fy + 1))
}

fn month_range(fy: i32, month: u32) -> (String, String) {
    let year = if month >= 4 { fy } else { fy + 1 };
    let last = days_in_month(year, month);
    (format!("{year}-{month:02}-01"), format!("{year}-{month:02}-{last:02}"))
}

fn quarter_range(fy: i32, quarter: usize) -> (String, String) {
    let (first, last) = match quarter {
        0 => (4, 6),
        1 => (7, 9),
        2 => (10, 12),
        _ => (1, 3),
    };
    (month_range(fy, first).0, month_range(fy, last).1)
}

fn display_date(iso: &str) -> String {
    let p: Vec<&str> = iso.split('-').collect();
    if p.len() == 3 { format!("{}-{}-{}", p[2], p[1], p[0]) } else { iso.to_owned() }
}

fn valid_date(value: &str) -> bool {
    let parts: Vec<&str> = value.trim().split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        parts[0].parse::<i32>(),
        parts[1].parse::<u32>(),
        parts[2].parse::<u32>(),
    ) else {
        return false;
    };
    (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month)
}

fn valid_date_range(from: &str, to: &str) -> bool {
    valid_date(from) && valid_date(to) && from <= to
}

// ───────────────────────────── date picker ─────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum PickerTarget {
    From,
    To,
}

const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = i64::from(if m <= 2 { y - 1 } else { y });
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = i64::from((m + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

/// Today's date in IST (UTC+5:30) as YYYY-MM-DD.
fn today_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
        + 19_800;
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    format!("{y}-{m:02}-{d:02}")
}

/// 0 = Monday … 6 = Sunday
fn weekday_monday_first(y: i32, m: u32, d: u32) -> usize {
    (days_from_civil(y, m, d) + 3).rem_euclid(7) as usize
}

fn split_iso(iso: &str) -> Option<(i32, u32, u32)> {
    let mut parts = iso.split('-');
    let y = parts.next()?.parse().ok()?;
    let m = parts.next()?.parse().ok()?;
    let d = parts.next()?.parse().ok()?;
    Some((y, m, d))
}

fn date_button(ui: &mut egui::Ui, text: &str, active: bool) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(format!("{text}   ▼")).size(12.5).color(INK))
            .fill(WHITE)
            .stroke(Stroke::new(1.0_f32, if active { PURPLE } else { BORDER }))
            .corner_radius(CornerRadius::same(8))
            .min_size(egui::vec2(124.0, 30.0)),
    )
}

fn open_picker(state: &mut ReportState, target: PickerTarget, anchor: egui::Rect) {
    if state.picker == Some(target) {
        state.picker = None;
        return;
    }
    let iso = match target {
        PickerTarget::From => state.from_date.clone(),
        PickerTarget::To => state.to_date.clone(),
    };
    let (y, m, _) = split_iso(&iso).unwrap_or((state.year, 4, 1));
    state.picker = Some(target);
    state.picker_anchor = Some(anchor);
    state.picker_year = y;
    state.picker_month = m.clamp(1, 12);
}

fn draw_date_picker(ctx: &egui::Context, state: &mut ReportState) -> Option<(PickerTarget, String)> {
    let target = state.picker?;
    let anchor = state.picker_anchor?;
    let selected = match target {
        PickerTarget::From => state.from_date.clone(),
        PickerTarget::To => state.to_date.clone(),
    };
    let today = today_iso();
    let (mut year, mut month) = (state.picker_year, state.picker_month.clamp(1, 12));
    let mut picked: Option<String> = None;
    let mut close = false;

    let area = egui::Area::new(Id::new("aster_reports_date_picker"))
        .order(egui::Order::Foreground)
        .fixed_pos(anchor.left_bottom() + egui::vec2(0.0, 6.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(WHITE)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.set_width(252.0);

                    ui.horizontal(|ui| {
                        if ui.small_button("«").clicked() {
                            year -= 1;
                        }
                        if ui.small_button("‹").clicked() {
                            if month == 1 {
                                month = 12;
                                year -= 1;
                            } else {
                                month -= 1;
                            }
                        }
                        ui.add_sized(
                            [108.0, 22.0],
                            Label::new(
                                RichText::new(format!("{} {}", MONTH_NAMES[(month - 1) as usize], year))
                                    .strong()
                                    .color(INK),
                            ),
                        );
                        if ui.small_button("›").clicked() {
                            if month == 12 {
                                month = 1;
                                year += 1;
                            } else {
                                month += 1;
                            }
                        }
                        if ui.small_button("»").clicked() {
                            year += 1;
                        }
                    });

                    ui.add_space(6.0);

                    let lead = weekday_monday_first(year, month, 1);
                    let dim = days_in_month(year, month) as usize;
                    let rows = (lead + dim).div_ceil(7);

                    egui::Grid::new("aster_reports_date_grid")
                        .spacing([2.0, 2.0])
                        .show(ui, |ui| {
                            for name in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
                                ui.add_sized(
                                    [34.0, 20.0],
                                    Label::new(RichText::new(name).size(10.5).strong().color(MUTED)),
                                );
                            }
                            ui.end_row();

                            for cell in 0..rows * 7 {
                                if cell >= lead && cell - lead < dim {
                                    let day = cell - lead + 1;
                                    let iso = format!("{year}-{month:02}-{day:02}");
                                    let is_selected = iso == selected;
                                    let is_today = iso == today;
                                    let button = egui::Button::new(
                                        RichText::new(day.to_string())
                                            .size(12.0)
                                            .color(if is_selected { WHITE } else { INK }),
                                    )
                                    .fill(if is_selected { PURPLE } else { WHITE })
                                    .stroke(if is_today && !is_selected {
                                        Stroke::new(1.0_f32, PURPLE)
                                    } else {
                                        Stroke::NONE
                                    })
                                    .corner_radius(CornerRadius::same(8));

                                    if ui.add_sized([34.0, 28.0], button).clicked() {
                                        picked = Some(iso);
                                    }
                                } else {
                                    ui.allocate_space(egui::vec2(34.0, 28.0));
                                }
                                if cell % 7 == 6 {
                                    ui.end_row();
                                }
                            }
                        });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if chip(ui, "Today") {
                            picked = Some(today.clone());
                        }
                        if chip(ui, "Close") {
                            close = true;
                        }
                    });
                });
        });

    state.picker_year = year.clamp(2000, 2100);
    state.picker_month = month;

    let rect = area.response.rect;
    let clicked_outside = ctx.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !rect.contains(p) && !anchor.contains(p))
    });
    if close || clicked_outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.picker = None;
    }

    picked.map(|iso| (target, iso))
}