use crate::utils::currency::format_inr;
use crate::{app::AsterApp, database::state_name, date::local_date};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Id, Key, Layout, Margin, Pos2, Rect,
    Response, RichText, ScrollArea, Sense, Shape, Stroke, TextStyle, Ui, Vec2,
};
use std::collections::BTreeMap;
use std::f32::consts::{PI, TAU};

// ───────────────────────── Palette ─────────────────────────
const TEXT: Color32 = Color32::from_rgb(28, 25, 38);
const MUTED: Color32 = Color32::from_rgb(92, 86, 108);
const FAINT: Color32 = Color32::from_rgb(150, 145, 162);
const PURPLE: Color32 = Color32::from_rgb(99, 68, 151);
const PURPLE_BG: Color32 = Color32::from_rgb(240, 235, 249);
const CARD: Color32 = Color32::WHITE;
const BORDER: Color32 = Color32::from_rgb(219, 214, 229);
const ROW_ALT: Color32 = Color32::from_rgb(249, 247, 253);
const HEADER_BG: Color32 = Color32::from_rgb(242, 239, 249);
const GREEN: Color32 = Color32::from_rgb(28, 124, 82);
const GREEN_BG: Color32 = Color32::from_rgb(225, 244, 234);
const ORANGE: Color32 = Color32::from_rgb(190, 104, 8);
const ORANGE_BG: Color32 = Color32::from_rgb(253, 240, 221);
const RED: Color32 = Color32::from_rgb(190, 50, 50);
const RED_BG: Color32 = Color32::from_rgb(250, 229, 229);

const MODES: [&str; 4] = ["Cash", "UPI", "Bank transfer", "Cheque"];
const MONTHS: [&str; 12] = [
    "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "Jan", "Feb", "Mar",
];

/// Height of one register row, and how many are visible before scrolling.
const ROW_H: f32 = 48.0;
const VISIBLE_ROWS: usize = 10;
const ACTIONS_W: f32 = 322.0;

// ───────────────────────── Data ─────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Filter {
    #[default]
    All,
    Pending,
    Unpaid,
    Partial,
    Paid,
}

fn matches_filter(invoice: &Invoice, filter: Filter) -> bool {
    match filter {
        Filter::All => true,
        Filter::Pending => invoice.balance() > 0,
        Filter::Unpaid => invoice.received == 0 && invoice.balance() > 0,
        Filter::Partial => invoice.received > 0 && invoice.balance() > 0,
        Filter::Paid => invoice.balance() == 0,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum SortKey {
    Invoice,
    #[default]
    Date,
    Customer,
    Total,
    Received,
    Balance,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Period {
    #[default]
    All,
    ThisMonth,
    Last30,
    Last90,
}

impl Period {
    const ALL: [Period; 4] = [
        Period::All,
        Period::ThisMonth,
        Period::Last30,
        Period::Last90,
    ];

    fn label(self) -> &'static str {
        match self {
            Period::All => "Whole year",
            Period::ThisMonth => "This month",
            Period::Last30 => "Last 30 days",
            Period::Last90 => "Last 90 days",
        }
    }
}

#[derive(Clone, Default)]
struct Invoice {
    id: i64,
    number: String,
    date: String,
    customer: String,
    total: i64,
    received: i64,
}

impl Invoice {
    fn balance(&self) -> i64 {
        self.total.saturating_sub(self.received).max(0)
    }

    fn status(&self) -> (&'static str, Color32, Color32) {
        if self.balance() == 0 {
            ("Paid", GREEN, GREEN_BG)
        } else if self.received > 0 {
            ("Partial", ORANGE, ORANGE_BG)
        } else {
            ("Unpaid", RED, RED_BG)
        }
    }
}

#[derive(Clone, Default)]
struct Receipt {
    date: String,
    amount: i64,
    mode: String,
    reference: String,
    notes: String,
}

#[derive(Clone, Default)]
struct PaymentDraft {
    invoice: Invoice,
    amount: String,
    mode: usize,
    date: String,
    reference: String,
    notes: String,
    error: String,
}

#[derive(Clone, Default)]
struct State {
    year: i32,
    loaded: bool,
    search: String,
    filter: Filter,
    sort_key: SortKey,
    /// false = descending (newest / largest first)
    sort_asc: bool,
    period: Period,
    customer: Option<String>,
    invoices: Vec<Invoice>,
    payment: Option<PaymentDraft>,
    history_for: Option<i64>,
    history: Vec<Receipt>,
    invoice_detail: Option<crate::database::SalesInvoiceDetail>,
    error: String,
    message: String,

    // screen-open animation bookkeeping
    seen: bool,
    last_frame: u64,
    intro_start: f64,
    epoch: u32,
    copied_at: f64,
}

#[derive(Default)]
struct Stats {
    total: i64,
    received: i64,
    outstanding: i64,
    paid: usize,
    partial: usize,
    unpaid: usize,
}

impl Stats {
    fn pending(&self) -> usize {
        self.partial + self.unpaid
    }

    fn percent(&self) -> f64 {
        if self.total > 0 {
            self.received as f64 / self.total as f64 * 100.0
        } else {
            0.0
        }
    }
}

fn stats(list: &[Invoice]) -> Stats {
    let mut s = Stats::default();
    for invoice in list {
        s.total += invoice.total;
        s.received += invoice.received;
        s.outstanding += invoice.balance();
        if invoice.balance() == 0 {
            s.paid += 1;
        } else if invoice.received > 0 {
            s.partial += 1;
        } else {
            s.unpaid += 1;
        }
    }
    s
}

// ───────────────────────── Animation helpers ─────────────────────────
fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// Staggered section entrance: section `index` starts a little after the previous one.
fn reveal_progress(now: f64, start: f64, index: usize) -> f32 {
    ease_out((((now - start) - index as f64 * 0.08) / 0.55).clamp(0.0, 1.0) as f32)
}

/// Generic delayed ease-out progress (used to grow charts after the page appears).
fn grow_progress(now: f64, start: f64, delay: f64, duration: f64) -> f32 {
    ease_out((((now - start) - delay) / duration).clamp(0.0, 1.0) as f32)
}

/// Fades a section in while it slides up into place.
fn reveal(ui: &mut Ui, progress: f32, add: impl FnOnce(&mut Ui)) {
    if progress >= 1.0 {
        add(ui);
        return;
    }
    ui.scope(|ui| {
        ui.set_opacity(progress);
        ui.add_space((1.0 - progress) * 22.0);
        add(ui);
    });
}

/// 0→1 progress since a dialog opened. Resets as soon as it is closed so the
/// pop-in plays every time it is opened again.
fn open_progress(ctx: &egui::Context, key: &str, open: bool, now: f64) -> f32 {
    let id = Id::new(("inv_dialog_open", key));
    if !open {
        ctx.data_mut(|d| d.remove::<f64>(id));
        return 0.0;
    }
    let t0 = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(id, || now));
    let t = ((now - t0) / 0.2).clamp(0.0, 1.0) as f32;
    if t < 1.0 {
        ctx.request_repaint();
    }
    ease_out(t)
}

/// Dimmed backdrop that blocks the page behind a dialog. Returns true when clicked.
fn backdrop(ctx: &egui::Context, salt: &'static str, progress: f32) -> bool {
    let screen = ctx.content_rect();
    egui::Area::new(Id::new(salt))
        .order(egui::Order::Middle)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_black_alpha((95.0 * progress) as u8));
            resp.clicked()
        })
        .inner
}

/// Counts a money value up (or between values) smoothly.
fn tween_money(
    ctx: &egui::Context,
    epoch: u32,
    key: &str,
    target: i64,
    now: f64,
    first_start: f64,
) -> i64 {
    let id = Id::new(("inv_tween", epoch, key));
    let (mut from, mut to, mut start): (i64, i64, f64) = ctx
        .data(|d| d.get_temp(id))
        .unwrap_or((0, target, first_start));
    let duration = 0.7_f64;
    let shown = |from: i64, to: i64, start: f64| -> (i64, bool) {
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

// ───────────────────────── Entry point ─────────────────────────
pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let ctx = ui.ctx().clone();
    let now = ctx.input(|i| i.time);
    let state_id = Id::new("aster_invoice_register_state");
    let mut state = ctx.data_mut(|d| d.get_temp::<State>(state_id).unwrap_or_default());

    // The screen counts as "opened" when it was not drawn on the previous frame
    // (first visit, or the user navigated away and came back).
    let frame_nr = ctx.cumulative_frame_nr();
    let opened = !state.seen || state.last_frame + 1 < frame_nr;
    state.last_frame = frame_nr;
    if opened {
        state.seen = true;
        state.intro_start = now;
        state.epoch = state.epoch.wrapping_add(1);
        state.message.clear();
        state.payment = None;
        state.history_for = None;
        state.history.clear();
        state.invoice_detail = None;
    }

    if opened || !state.loaded || state.year != app.selected_year {
        load(app, &mut state);
    }

    // Distinct customers for the picker.
    let mut customers: Vec<String> = state.invoices.iter().map(|i| i.customer.clone()).collect();
    customers.sort_by_key(|c| c.to_lowercase());
    customers.dedup();

    if let Some(selected) = &state.customer {
        if !customers.contains(selected) {
            state.customer = None;
        }
    }

    // Everything on the page (KPIs, charts, register) is scoped to the chosen customer.
    let scoped: Vec<Invoice> = state
        .invoices
        .iter()
        .filter(|i| state.customer.as_ref().map_or(true, |c| &i.customer == c))
        .cloned()
        .collect();
    let st = stats(&scoped);

    // Animation progress
    let start = state.intro_start;
    let epoch = state.epoch;
    if now - start < 1.8 {
        ctx.request_repaint();
    }
    let chart_grow = grow_progress(now, start, 0.55, 0.9);
    let kpi_total = tween_money(&ctx, epoch, "total", st.total, now, start + 0.15);
    let kpi_received = tween_money(&ctx, epoch, "received", st.received, now, start + 0.2);
    let kpi_outstanding = tween_money(&ctx, epoch, "outstanding", st.outstanding, now, start + 0.25);

    let mut open_payment: Option<Invoice> = None;
    let mut open_history: Option<i64> = None;
    let mut open_invoice: Option<i64> = None;

    ScrollArea::vertical()
        .id_salt("invoice_register_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            apply_style(ui);
            ui.add_space(6.0);

            reveal(ui, reveal_progress(now, start, 0), |ui| {
                header(ui, app, &mut state);
            });
            ui.add_space(18.0);

            // Notices
            if !state.error.is_empty() {
                let mut retry = false;
                egui::Frame::new()
                    .fill(RED_BG)
                    .stroke(Stroke::new(1.0_f32, RED))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&state.error).size(13.5).strong().color(RED));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("Retry").clicked() {
                                    retry = true;
                                }
                            });
                        });
                    });
                if retry {
                    load(app, &mut state);
                }
                ui.add_space(12.0);
            }

            if !state.message.is_empty() {
                let mut dismiss = false;
                egui::Frame::new()
                    .fill(GREEN_BG)
                    .stroke(Stroke::new(1.0_f32, GREEN))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("✓  {}", state.message))
                                    .size(13.5)
                                    .strong()
                                    .color(GREEN),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("Dismiss").clicked() {
                                    dismiss = true;
                                }
                            });
                        });
                    });
                if dismiss {
                    state.message.clear();
                }
                ui.add_space(12.0);
            }

            // Customer scope banner
            if let Some(name) = state.customer.clone() {
                egui::Frame::new()
                    .fill(PURPLE_BG)
                    .stroke(Stroke::new(1.0_f32, PURPLE))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("CUSTOMER").size(11.0).strong().color(PURPLE));
                            ui.label(RichText::new(&name).size(15.0).strong().color(TEXT));
                            ui.label(
                                RichText::new(format!("·  {} invoices", scoped.len()))
                                    .size(13.0)
                                    .color(MUTED),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("Show all customers").clicked() {
                                    state.customer = None;
                                }
                            });
                        });
                    });
                ui.add_space(12.0);
            }

            // KPI row
            reveal(ui, reveal_progress(now, start, 1), |ui| {
                ui.columns(4, |cols| {
                    metric(
                        &mut cols[0],
                        "INVOICED",
                        &money(kpi_total),
                        "Posted invoice value",
                        PURPLE,
                    );
                    metric(
                        &mut cols[1],
                        "COLLECTED",
                        &money(kpi_received),
                        &format!("{:.1}% of invoiced", st.percent()),
                        GREEN,
                    );
                    metric(
                        &mut cols[2],
                        "OUTSTANDING",
                        &money(kpi_outstanding),
                        "Balance to collect",
                        if st.outstanding > 0 { ORANGE } else { GREEN },
                    );
                    metric(
                        &mut cols[3],
                        "PENDING INVOICES",
                        &st.pending().to_string(),
                        &format!("{} unpaid · {} partial", st.unpaid, st.partial),
                        RED,
                    );
                });
            });

            ui.add_space(14.0);

            // Register comes first, ahead of the charts.
            reveal(ui, reveal_progress(now, start, 2), |ui| {
                register_card(
                    ui,
                    &mut state,
                    &scoped,
                    &customers,
                    &mut open_payment,
                    &mut open_history,
                    &mut open_invoice,
                    now,
                );
            });

            ui.add_space(14.0);

            // Charts
            reveal(ui, reveal_progress(now, start, 3), |ui| {
                ui.columns(2, |cols| {
                    donut_card(&mut cols[0], &mut state, &st, chart_grow);
                    top_customers_card(&mut cols[1], &mut state, &scoped, chart_grow);
                });
            });

            ui.add_space(14.0);
            reveal(ui, reveal_progress(now, start, 4), |ui| {
                monthly_card(ui, app.selected_year, &scoped, chart_grow);
            });

            ui.add_space(20.0);
        });

    // Actions raised by the register.
    if let Some(invoice) = open_payment {
        let (year, month, day) = local_date();
        state.payment = Some(PaymentDraft {
            invoice,
            amount: String::new(),
            mode: 0,
            date: format!("{year:04}-{month:02}-{day:02}"),
            reference: String::new(),
            notes: String::new(),
            error: String::new(),
        });
        state.message.clear();
    }

    if let Some(invoice_id) = open_history {
        state.history_for = Some(invoice_id);
        load_history(app, &mut state, invoice_id);
    }

    if let Some(invoice_id) = open_invoice {
        load_invoice_detail(app, &mut state, invoice_id);
    }

    // Dialogs
    let mut cancel = false;
    let mut save = false;
    let mut close_history = false;
    let mut close_invoice_detail = false;

    // Esc closes the top-most dialog.
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        if state.invoice_detail.is_some() {
            close_invoice_detail = true;
        } else if state.history_for.is_some() {
            close_history = true;
        } else if state.payment.is_some() {
            cancel = true;
        }
    }

    let pay_p = open_progress(&ctx, "payment", state.payment.is_some(), now);
    let his_p = open_progress(&ctx, "history", state.history_for.is_some(), now);
    let det_p = open_progress(&ctx, "detail", state.invoice_detail.is_some(), now);

    if let Some(payment) = state.payment.as_mut() {
        backdrop(&ctx, "inv_backdrop_payment", pay_p);
        payment_window(&ctx, payment, &mut cancel, &mut save, pay_p);
    }
    if cancel {
        state.payment = None;
    }

    if let Some(invoice_id) = state.history_for {
        let invoice = state.invoices.iter().find(|i| i.id == invoice_id).cloned();
        if backdrop(&ctx, "inv_backdrop_history", his_p) {
            close_history = true;
        }
        history_window(
            &ctx,
            invoice.as_ref(),
            &state.history,
            &mut close_history,
            his_p,
        );
    }
    if close_history {
        state.history_for = None;
        state.history.clear();
    }

    if let Some(detail) = state.invoice_detail.as_ref() {
        if backdrop(&ctx, "inv_backdrop_detail", det_p) {
            close_invoice_detail = true;
        }
        invoice_detail_window(&ctx, detail, &mut close_invoice_detail, det_p);
    }
    if close_invoice_detail {
        state.invoice_detail = None;
    }

    if save {
        if let Some(payment) = state.payment.clone() {
            let amount = parse_rupees(&payment.amount).unwrap_or(0);
            let mode = MODES.get(payment.mode).copied().unwrap_or("Cash");

            let result = app
                .db
                .as_ref()
                .ok_or_else(|| "Database connection is unavailable.".to_owned())
                .and_then(|db| {
                    db.record_sales_payment(
                        app.selected_year,
                        payment.invoice.id,
                        amount,
                        mode,
                        &payment.date,
                        &payment.reference,
                        &payment.notes,
                    )
                });

            match result {
                Ok(_) => {
                    state.payment = None;
                    state.message = format!(
                        "Payment saved for {}. The invoice balance has been updated.",
                        payment.invoice.number
                    );
                    load(app, &mut state);
                    if let Some(invoice_id) = state.history_for {
                        load_history(app, &mut state, invoice_id);
                    }
                }
                Err(error) => {
                    if let Some(form) = state.payment.as_mut() {
                        form.error = error;
                    }
                }
            }
        }
    }

    ctx.data_mut(|d| d.insert_temp(state_id, state));
}

// ───────────────────────── Sections ─────────────────────────
fn header(ui: &mut Ui, app: &AsterApp, state: &mut State) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("TRANSACTIONS  /  RECEIVABLES")
                    .size(11.0)
                    .strong()
                    .color(PURPLE),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new("Invoice Management")
                    .size(28.0)
                    .strong()
                    .color(TEXT),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new("Track balances, record collections and review receipts.")
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
                load(app, state);
            }
            egui::Frame::new()
                .fill(PURPLE_BG)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(12, 7))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(format!(
                            "FY {}–{}",
                            app.selected_year,
                            app.selected_year + 1
                        ))
                        .size(13.0)
                        .strong()
                        .color(PURPLE),
                    );
                });
        });
    });
}

fn donut_card(ui: &mut Ui, state: &mut State, st: &Stats, grow: f32) {
    card(ui, |ui| {
        ui.set_min_height(262.0);
        section_title(
            ui,
            "Payment status",
            "Invoices by collection state · click to filter the register",
        );
        ui.add_space(12.0);

        let segs = [
            Seg {
                label: "Paid",
                value: st.paid as f32,
                color: GREEN,
                filter: Filter::Paid,
            },
            Seg {
                label: "Partial",
                value: st.partial as f32,
                color: ORANGE,
                filter: Filter::Partial,
            },
            Seg {
                label: "Unpaid",
                value: st.unpaid as f32,
                color: RED,
                filter: Filter::Unpaid,
            },
        ];
        let count = st.paid + st.partial + st.unpaid;

        ui.horizontal(|ui| {
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(196.0), Sense::hover());
            donut(ui, rect, &resp, &segs, count, grow);

            ui.add_space(10.0);
            ui.vertical(|ui| {
                for seg in &segs {
                    let pct = if count > 0 {
                        seg.value / count as f32 * 100.0
                    } else {
                        0.0
                    };
                    ui.horizontal(|ui| {
                        dot(ui, seg.color);
                        let active = state.filter == seg.filter;
                        let r = ui.add(
                            egui::Button::new(
                                RichText::new(seg.label)
                                    .size(14.0)
                                    .color(if active { seg.color } else { TEXT })
                                    .strong(),
                            )
                            .frame(false),
                        );
                        if r.on_hover_text("Filter the register").clicked() {
                            state.filter = if active { Filter::All } else { seg.filter };
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new((seg.value as usize).to_string())
                                    .size(14.0)
                                    .strong()
                                    .color(TEXT),
                            );
                            ui.label(RichText::new(format!("{pct:.0}%")).size(12.5).color(MUTED));
                        });
                    });
                }

                ui.add_space(10.0);
                ui.label(
                    RichText::new(format!("{:.1}% collected", st.percent()))
                        .size(13.0)
                        .strong()
                        .color(GREEN),
                );
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 10.0), Sense::hover());
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(5), PURPLE_BG);
                if st.total > 0 {
                    let frac = (st.received as f32 / st.total as f32).clamp(0.0, 1.0);
                    let w = rect.width() * frac * grow;
                    if w > 0.5 {
                        ui.painter().rect_filled(
                            Rect::from_min_size(rect.min, Vec2::new(w, rect.height())),
                            CornerRadius::same(5),
                            GREEN,
                        );
                    }
                }
            });
        });
    });
}

fn top_customers_card(ui: &mut Ui, state: &mut State, scoped: &[Invoice], grow: f32) {
    let ctx = ui.ctx().clone();
    card(ui, |ui| {
        ui.set_min_height(262.0);
        section_title(
            ui,
            "Top outstanding customers",
            "Click a customer to focus the whole page",
        );
        ui.add_space(12.0);

        let mut by: BTreeMap<String, i64> = BTreeMap::new();
        for invoice in scoped {
            if invoice.balance() > 0 {
                *by.entry(invoice.customer.clone()).or_default() += invoice.balance();
            }
        }
        let mut rows: Vec<(String, i64)> = by.into_iter().collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1));
        let all_outstanding: i64 = rows.iter().map(|r| r.1).sum();
        rows.truncate(5);

        if rows.is_empty() {
            ui.add_space(36.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("All invoices are fully collected")
                        .size(14.5)
                        .strong()
                        .color(GREEN),
                );
                ui.label(
                    RichText::new("No balances pending.")
                        .size(12.5)
                        .color(MUTED),
                );
            });
            return;
        }

        let max = rows.iter().map(|r| r.1).max().unwrap_or(1).max(1);
        for (rank, (name, amount)) in rows.iter().enumerate() {
            let selected = state.customer.as_deref() == Some(name.as_str());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{}", rank + 1))
                        .size(12.0)
                        .strong()
                        .color(FAINT),
                );
                let text = RichText::new(truncate(name, 24))
                    .size(14.0)
                    .color(if selected { PURPLE } else { TEXT });
                let text = if selected { text.strong() } else { text };
                let r = ui.add(egui::Button::new(text).frame(false));
                if r.on_hover_text(name.as_str()).clicked() {
                    state.customer = if selected { None } else { Some(name.clone()) };
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(money(*amount))
                            .size(14.0)
                            .strong()
                            .color(ORANGE),
                    );
                    if all_outstanding > 0 {
                        ui.label(
                            RichText::new(format!(
                                "{:.0}%",
                                *amount as f64 / all_outstanding as f64 * 100.0
                            ))
                            .size(12.0)
                            .color(MUTED),
                        );
                    }
                });
            });

            // Bars ease towards new values and grow in with a small stagger.
            let target = *amount as f32 / max as f32;
            let eased = ctx.animate_value_with_time(Id::new(("inv_topbar", name)), target, 0.3);
            let stagger = ease_out((grow * 1.5 - rank as f32 * 0.14).clamp(0.0, 1.0));
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 8.0), Sense::hover());
            ui.painter()
                .rect_filled(rect, CornerRadius::same(4), PURPLE_BG);
            let w = rect.width() * eased * stagger;
            if w > 0.5 {
                ui.painter().rect_filled(
                    Rect::from_min_size(rect.min, Vec2::new(w, rect.height())),
                    CornerRadius::same(4),
                    if selected { PURPLE } else { ORANGE },
                );
            }
            ui.add_space(6.0);
        }
    });
}

fn monthly_card(ui: &mut Ui, year: i32, scoped: &[Invoice], grow: f32) {
    let ctx = ui.ctx().clone();
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                section_title(
                    ui,
                    "Monthly invoiced vs collected",
                    "Grouped by invoice month across the financial year",
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                legend_item(ui, GREEN, "Collected");
                ui.add_space(8.0);
                legend_item(ui, PURPLE, "Invoiced");
            });
        });
        ui.add_space(12.0);

        let mut invoiced = [0_i64; 12];
        let mut collected = [0_i64; 12];
        for invoice in scoped {
            if let Some(m) = invoice.date.get(5..7).and_then(|m| m.parse::<usize>().ok()) {
                if (1..=12).contains(&m) {
                    let idx = (m + 8) % 12;
                    invoiced[idx] += invoice.total;
                    collected[idx] += invoice.received;
                }
            }
        }

        let max = invoiced
            .iter()
            .chain(collected.iter())
            .copied()
            .max()
            .unwrap_or(0);
        let top = nice_max(max);

        // Current month marker (only when today falls inside this financial year).
        let current_idx = {
            let (cy, cm, _) = local_date();
            let in_year = if cm >= 4 { cy == year } else { cy == year + 1 };
            in_year.then(|| (cm as usize + 8) % 12)
        };

        // Average monthly invoicing across months that had activity.
        let active_months: Vec<i64> = invoiced.iter().copied().filter(|v| *v > 0).collect();
        let average = if active_months.is_empty() {
            0
        } else {
            active_months.iter().sum::<i64>() / active_months.len() as i64
        };

        let (rect, resp) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 250.0), Sense::hover());
        let painter = ui.painter_at(rect.expand(2.0));
        let plot = Rect::from_min_max(
            Pos2::new(rect.left() + 62.0, rect.top() + 10.0),
            Pos2::new(rect.right() - 6.0, rect.bottom() - 30.0),
        );
        let group_w = plot.width() / 12.0;

        let hovered = resp.hover_pos().and_then(|p| {
            if plot.expand2(Vec2::new(0.0, 10.0)).contains(p) {
                Some((((p.x - plot.left()) / group_w) as usize).min(11))
            } else {
                None
            }
        });

        // Gridlines + y labels
        for i in 0..=4 {
            let y = plot.bottom() - plot.height() * i as f32 / 4.0;
            let (a, b) = (Pos2::new(plot.left(), y), Pos2::new(plot.right(), y));
            if i == 0 {
                painter.line_segment([a, b], Stroke::new(1.0_f32, MUTED));
            } else {
                painter.extend(Shape::dashed_line(
                    &[a, b],
                    Stroke::new(1.0_f32, BORDER),
                    4.0,
                    4.0,
                ));
            }
            painter.text(
                Pos2::new(plot.left() - 8.0, y),
                Align2::RIGHT_CENTER,
                compact(top / 4 * i),
                FontId::proportional(12.0),
                MUTED,
            );
        }

        // Sliding hover band
        let last_id = Id::new("inv_month_last_hover");
        let last: usize = ctx.data(|d| d.get_temp(last_id)).unwrap_or(0);
        let target_idx = hovered.unwrap_or(last);
        ctx.data_mut(|d| d.insert_temp(last_id, target_idx));
        let band_x = ctx.animate_value_with_time(Id::new("inv_month_band_x"), target_idx as f32, 0.12);
        let band_a = ctx.animate_bool_with_time(Id::new("inv_month_band_a"), hovered.is_some(), 0.12);
        if band_a > 0.01 {
            let cx = plot.left() + group_w * (band_x + 0.5);
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(cx - group_w / 2.0 + 2.0, plot.top()),
                    Pos2::new(cx + group_w / 2.0 - 2.0, plot.bottom()),
                ),
                CornerRadius::same(6),
                PURPLE_BG.gamma_multiply(band_a),
            );
        }

        let bar_w = (group_w * 0.28).clamp(6.0, 22.0);
        for i in 0..12 {
            let cx = plot.left() + group_w * (i as f32 + 0.5);
            let bar_grow = ease_out((grow * 1.4 - i as f32 * 0.05).clamp(0.0, 1.0));

            for (k, (value, color)) in [(invoiced[i], PURPLE), (collected[i], GREEN)]
                .into_iter()
                .enumerate()
            {
                if value <= 0 {
                    continue;
                }
                let full = (plot.height() * value as f32 / top as f32).max(2.0);
                let h = full * bar_grow;
                if h < 0.5 {
                    continue;
                }
                let x0 = if k == 0 { cx - bar_w - 1.5 } else { cx + 1.5 };
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(x0, plot.bottom() - h),
                        Pos2::new(x0 + bar_w, plot.bottom()),
                    ),
                    CornerRadius {
                        nw: 5,
                        ne: 5,
                        sw: 0,
                        se: 0,
                    },
                    if hovered == Some(i) {
                        mix(color, Color32::WHITE, 0.12)
                    } else {
                        color
                    },
                );
            }

            let is_current = current_idx == Some(i);
            painter.text(
                Pos2::new(cx, plot.bottom() + 8.0),
                Align2::CENTER_TOP,
                MONTHS[i],
                FontId::proportional(12.0),
                if hovered == Some(i) || is_current {
                    TEXT
                } else {
                    MUTED
                },
            );
            if is_current {
                painter.circle_filled(Pos2::new(cx, plot.bottom() + 25.0), 2.5, PURPLE);
            }
        }

        // Average invoiced line
        if average > 0 && grow > 0.6 {
            let a = ((grow - 0.6) / 0.4).clamp(0.0, 1.0);
            let y = plot.bottom() - plot.height() * (average as f32 / top as f32).clamp(0.0, 1.0);
            painter.extend(Shape::dashed_line(
                &[Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
                Stroke::new(1.2_f32, PURPLE.gamma_multiply(0.55 * a)),
                6.0,
                4.0,
            ));
            painter.text(
                Pos2::new(plot.right() - 2.0, y - 3.0),
                Align2::RIGHT_BOTTOM,
                format!("avg invoiced {}", compact(average)),
                FontId::proportional(11.0),
                PURPLE.gamma_multiply(a),
            );
        }

        if let Some(i) = hovered {
            let y = if i < 9 { year } else { year + 1 };
            let rate = if invoiced[i] > 0 {
                collected[i] as f64 / invoiced[i] as f64 * 100.0
            } else {
                0.0
            };
            resp.on_hover_ui_at_pointer(|ui| {
                ui.label(
                    RichText::new(format!("{} {}", MONTHS[i], y))
                        .size(14.0)
                        .strong(),
                );
                ui.label(
                    RichText::new(format!("Invoiced: {}", money(invoiced[i])))
                        .size(13.0)
                        .color(PURPLE),
                );
                ui.label(
                    RichText::new(format!("Collected: {}", money(collected[i])))
                        .size(13.0)
                        .color(GREEN),
                );
                ui.label(
                    RichText::new(format!(
                        "Outstanding: {}",
                        money((invoiced[i] - collected[i]).max(0))
                    ))
                    .size(13.0)
                    .color(ORANGE),
                );
                if invoiced[i] > 0 {
                    ui.label(
                        RichText::new(format!("Collection rate: {rate:.0}%"))
                            .size(12.5)
                            .color(MUTED),
                    );
                }
            });
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn register_card(
    ui: &mut Ui,
    state: &mut State,
    scoped: &[Invoice],
    customers: &[String],
    open_payment: &mut Option<Invoice>,
    open_history: &mut Option<i64>,
    open_invoice: &mut Option<i64>,
    now: f64,
) {
    let ctx = ui.ctx().clone();
    card(ui, |ui| {
        section_title(
            ui,
            "Invoice register",
            "Search, sort, filter by customer, period or status, and record payments",
        );
        ui.add_space(14.0);

        // ── Toolbar
        ui.horizontal(|ui| {
            let avail = ui.available_width();
            let search_w = (avail - 280.0 - 160.0 - 210.0).clamp(180.0, 360.0);

            ui.vertical(|ui| {
                ui.label(caps("SEARCH"));
                ui.add_sized(
                    [search_w, 34.0],
                    egui::TextEdit::singleline(&mut state.search)
                        .hint_text("Invoice number or customer name"),
                );
            });
            ui.vertical(|ui| {
                ui.label(caps("CUSTOMER"));
                let current = state
                    .customer
                    .clone()
                    .unwrap_or_else(|| "All customers".to_owned());
                egui::ComboBox::from_id_salt("invoice_customer_filter")
                    .width(260.0)
                    .selected_text(RichText::new(truncate(&current, 30)).size(14.0))
                    .show_ui(ui, |ui| {
                        ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                            ui.selectable_value(&mut state.customer, None, "All customers");
                            for customer in customers {
                                ui.selectable_value(
                                    &mut state.customer,
                                    Some(customer.clone()),
                                    customer.as_str(),
                                );
                            }
                        });
                    });
            });
            ui.vertical(|ui| {
                ui.label(caps("PERIOD"));
                egui::ComboBox::from_id_salt("invoice_period_filter")
                    .width(140.0)
                    .selected_text(RichText::new(state.period.label()).size(14.0))
                    .show_ui(ui, |ui| {
                        for period in Period::ALL {
                            ui.selectable_value(&mut state.period, period, period.label());
                        }
                    });
            });

            let active = !state.search.is_empty()
                || state.customer.is_some()
                || state.filter != Filter::All
                || state.period != Period::All;
            if active {
                ui.vertical(|ui| {
                    ui.label(caps(" "));
                    if ui.button("Clear filters").clicked() {
                        state.search.clear();
                        state.customer = None;
                        state.filter = Filter::All;
                        state.period = Period::All;
                    }
                });
            }
        });

        // ── Search + period applied (status chips show counts for these)
        let needle = state.search.trim().to_lowercase();
        let cutoff = period_cutoff(state.period, local_date());
        let searched: Vec<&Invoice> = scoped
            .iter()
            .filter(|i| {
                (needle.is_empty()
                    || i.number.to_lowercase().contains(&needle)
                    || i.customer.to_lowercase().contains(&needle))
                    && cutoff.as_ref().map_or(true, |c| i.date.as_str() >= c.as_str())
            })
            .collect();

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            for (label, filter) in [
                ("All", Filter::All),
                ("Pending", Filter::Pending),
                ("Unpaid", Filter::Unpaid),
                ("Partial", Filter::Partial),
                ("Paid", Filter::Paid),
            ] {
                let n = searched
                    .iter()
                    .filter(|i| matches_filter(i, filter))
                    .count();
                let selected = state.filter == filter;
                let t = ctx.animate_bool_with_time(
                    Id::new(("inv_chip", label)),
                    selected,
                    0.15,
                );
                let button = egui::Button::new(
                    RichText::new(format!("{label}   {n}"))
                        .size(13.0)
                        .strong()
                        .color(mix(MUTED, PURPLE, t)),
                )
                .fill(mix(CARD, PURPLE_BG, t))
                .stroke(Stroke::new(1.0_f32, mix(BORDER, PURPLE, t)))
                .corner_radius(CornerRadius::same(16))
                .min_size(Vec2::new(0.0, 30.0));
                if ui.add(button).clicked() {
                    state.filter = filter;
                }
            }
        });

        // ── Filter + sort
        let mut filtered: Vec<&Invoice> = searched
            .iter()
            .copied()
            .filter(|i| matches_filter(i, state.filter))
            .collect();
        let (sort_key, sort_asc) = (state.sort_key, state.sort_asc);
        filtered.sort_by(|a, b| {
            let ord = match sort_key {
                SortKey::Invoice => a.number.cmp(&b.number),
                SortKey::Date => a.date.cmp(&b.date).then(a.id.cmp(&b.id)),
                SortKey::Customer => a
                    .customer
                    .to_lowercase()
                    .cmp(&b.customer.to_lowercase()),
                SortKey::Total => a.total.cmp(&b.total),
                SortKey::Received => a.received.cmp(&b.received),
                SortKey::Balance => a.balance().cmp(&b.balance()),
            };
            if sort_asc {
                ord
            } else {
                ord.reverse()
            }
        });

        ui.add_space(14.0);

        // ── Column widths (the frames below add 12px padding either side)
        let inner_w = (ui.available_width() - 24.0).max(720.0);
        let rem = (inner_w - ACTIONS_W).max(480.0);
        let fractions = [0.13_f32, 0.11, 0.20, 0.12, 0.15, 0.14, 0.15];
        let w: Vec<f32> = fractions.iter().map(|f| f * rem).collect();

        // ── Header row (click a column to sort)
        egui::Frame::new()
            .fill(HEADER_BG)
            .corner_radius(CornerRadius {
                nw: 8,
                ne: 8,
                sw: 0,
                se: 0,
            })
            .inner_margin(Margin::symmetric(12, 4))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let columns: [(&str, Align, Option<SortKey>); 7] = [
                        ("INVOICE", Align::Min, Some(SortKey::Invoice)),
                        ("DATE", Align::Min, Some(SortKey::Date)),
                        ("CUSTOMER", Align::Min, Some(SortKey::Customer)),
                        ("STATUS", Align::Min, None),
                        ("TOTAL", Align::Max, Some(SortKey::Total)),
                        ("RECEIVED", Align::Max, Some(SortKey::Received)),
                        ("BALANCE", Align::Max, Some(SortKey::Balance)),
                    ];
                    for (i, (label, align, key)) in columns.into_iter().enumerate() {
                        cell(ui, w[i], 30.0, align, |ui| match key {
                            Some(key) => {
                                let active = state.sort_key == key;
                                let arrow = if !active {
                                    ""
                                } else if state.sort_asc {
                                    "  ▲"
                                } else {
                                    "  ▼"
                                };
                                let r = ui.add(
                                    egui::Button::new(
                                        RichText::new(format!("{label}{arrow}"))
                                            .size(11.0)
                                            .strong()
                                            .color(if active { PURPLE } else { MUTED }),
                                    )
                                    .frame(false),
                                );
                                if r.on_hover_text("Click to sort").clicked() {
                                    if active {
                                        state.sort_asc = !state.sort_asc;
                                    } else {
                                        state.sort_key = key;
                                        state.sort_asc =
                                            matches!(key, SortKey::Invoice | SortKey::Customer);
                                    }
                                }
                            }
                            None => {
                                ui.label(caps(label));
                            }
                        });
                    }
                    cell(ui, ACTIONS_W, 30.0, Align::Max, |ui| {
                        ui.label(caps("ACTIONS"));
                    });
                });
            });

        // ── Rows: exactly VISIBLE_ROWS tall before it needs to scroll
        egui::Frame::new()
            .stroke(Stroke::new(1.0_f32, BORDER))
            .corner_radius(CornerRadius {
                nw: 0,
                ne: 0,
                sw: 8,
                se: 8,
            })
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 0.0;

                let total_rows = filtered.len();
                if total_rows == 0 {
                    ui.add_space(34.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("No matching invoices")
                                .size(16.0)
                                .strong()
                                .color(TEXT),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Try a different search, customer, period or status.")
                                .size(13.0)
                                .color(MUTED),
                        );
                    });
                    ui.add_space(34.0);
                    return;
                }

                let visible_h = total_rows.min(VISIBLE_ROWS) as f32 * ROW_H;
                ScrollArea::vertical()
                    .id_salt("invoice_register_rows")
                    .auto_shrink([false, false])
                    .min_scrolled_height(visible_h)
                    .max_height(visible_h)
                    .show_rows(ui, ROW_H, total_rows, |ui, range| {
                        for idx in range {
                            let invoice = filtered[idx];
                            let width = ui.available_width();
                            let (rect, resp) =
                                ui.allocate_exact_size(Vec2::new(width, ROW_H), Sense::click());

                            let hovering = ui.rect_contains_pointer(rect);
                            let hot = ctx.animate_bool_with_time(
                                Id::new(("inv_row_hover", invoice.id)),
                                hovering,
                                0.12,
                            );
                            let base = if idx % 2 == 0 { CARD } else { ROW_ALT };
                            ui.painter().rect_filled(rect, 0.0, mix(base, PURPLE_BG, hot * 0.8));
                            if hot > 0.01 {
                                ui.painter().rect_filled(
                                    Rect::from_min_size(rect.min, Vec2::new(3.0, ROW_H)),
                                    0.0,
                                    PURPLE.gamma_multiply(hot),
                                );
                            }
                            if resp.double_clicked() {
                                *open_invoice = Some(invoice.id);
                            }

                            let inner = Rect::from_min_max(
                                rect.min + Vec2::new(12.0, 0.0),
                                rect.max - Vec2::new(12.0, 0.0),
                            );
                            let mut row = ui.new_child(
                                egui::UiBuilder::new()
                                    .max_rect(inner)
                                    .layout(Layout::left_to_right(Align::Center)),
                            );
                            row.spacing_mut().item_spacing.x = 0.0;
                            row.spacing_mut().item_spacing.y = 0.0;
                            let row = &mut row;

                            cell(row, w[0], ROW_H, Align::Min, |ui| {
                                ui.label(
                                    RichText::new(&invoice.number)
                                        .size(14.0)
                                        .strong()
                                        .color(TEXT),
                                );
                            });
                            cell(row, w[1], ROW_H, Align::Min, |ui| {
                                ui.label(RichText::new(&invoice.date).size(13.5).color(MUTED));
                            });
                            cell(row, w[2], ROW_H, Align::Min, |ui| {
                                let max_chars = ((w[2] / 7.6) as usize).max(8);
                                let r = ui.add(
                                    egui::Button::new(
                                        RichText::new(truncate(&invoice.customer, max_chars))
                                            .size(14.0)
                                            .color(TEXT),
                                    )
                                    .frame(false),
                                );
                                if r.on_hover_text(format!(
                                    "{} — click to show only this customer",
                                    invoice.customer
                                ))
                                .clicked()
                                {
                                    state.customer = Some(invoice.customer.clone());
                                }
                            });
                            cell(row, w[3], ROW_H, Align::Min, |ui| {
                                let (text, fg, bg) = invoice.status();
                                badge(ui, text, fg, bg);
                                // tiny collected-progress bar under the badge
                                ui.add_space(3.0);
                                let (bar, _) =
                                    ui.allocate_exact_size(Vec2::new(46.0, 3.0), Sense::hover());
                                ui.painter().rect_filled(bar, CornerRadius::same(2), BORDER);
                                if invoice.total > 0 {
                                    let frac = (invoice.received as f32 / invoice.total as f32)
                                        .clamp(0.0, 1.0);
                                    ui.painter().rect_filled(
                                        Rect::from_min_size(
                                            bar.min,
                                            Vec2::new(bar.width() * frac, bar.height()),
                                        ),
                                        CornerRadius::same(2),
                                        fg,
                                    );
                                }
                            });
                            cell(row, w[4], ROW_H, Align::Max, |ui| {
                                ui.label(
                                    RichText::new(money(invoice.total)).size(13.5).color(TEXT),
                                );
                            });
                            cell(row, w[5], ROW_H, Align::Max, |ui| {
                                ui.label(
                                    RichText::new(money(invoice.received))
                                        .size(13.5)
                                        .color(GREEN),
                                );
                            });
                            cell(row, w[6], ROW_H, Align::Max, |ui| {
                                ui.label(
                                    RichText::new(money(invoice.balance()))
                                        .size(13.5)
                                        .strong()
                                        .color(if invoice.balance() > 0 { ORANGE } else { MUTED }),
                                );
                            });

                            row.allocate_ui_with_layout(
                                Vec2::new(ACTIONS_W, ROW_H),
                                Layout::right_to_left(Align::Center),
                                |ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    ui.spacing_mut().button_padding = Vec2::new(9.0, 5.0);
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("View invoice").size(13.0),
                                            )
                                            .min_size(Vec2::new(0.0, 30.0)),
                                        )
                                        .on_hover_text("Open invoice (or double-click the row)")
                                        .clicked()
                                    {
                                        *open_invoice = Some(invoice.id);
                                    }
                                    if ui
                                        .add(
                                            egui::Button::new(RichText::new("History").size(13.0))
                                                .min_size(Vec2::new(0.0, 30.0)),
                                        )
                                        .clicked()
                                    {
                                        *open_history = Some(invoice.id);
                                    }
                                    if invoice.balance() > 0
                                        && ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new("＋ Record payment")
                                                        .size(13.0)
                                                        .strong()
                                                        .color(Color32::WHITE),
                                                )
                                                .fill(PURPLE)
                                                .min_size(Vec2::new(0.0, 30.0)),
                                            )
                                            .clicked()
                                    {
                                        *open_payment = Some(invoice.clone());
                                    }
                                },
                            );
                        }
                    });
            });

        // ── Footer: counts, totals in view, CSV export
        ui.add_space(10.0);
        let view_total: i64 = filtered.iter().map(|i| i.total).sum();
        let view_received: i64 = filtered.iter().map(|i| i.received).sum();
        let view_balance: i64 = filtered.iter().map(|i| i.balance()).sum();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "Showing {} of {} invoices",
                    filtered.len(),
                    scoped.len()
                ))
                .size(12.5)
                .color(MUTED),
            );
            ui.label(RichText::new("·").size(12.5).color(FAINT));
            ui.label(
                RichText::new(format!("Invoiced {}", money(view_total)))
                    .size(12.5)
                    .color(MUTED),
            );
            ui.label(
                RichText::new(format!("Collected {}", money(view_received)))
                    .size(12.5)
                    .color(GREEN),
            );
            ui.label(
                RichText::new(format!("Outstanding {}", money(view_balance)))
                    .size(12.5)
                    .strong()
                    .color(if view_balance > 0 { ORANGE } else { GREEN }),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let just_copied = now - state.copied_at < 1.8;
                if just_copied {
                    ctx.request_repaint_after(std::time::Duration::from_millis(200));
                }
                let label = if just_copied {
                    "✓  Copied"
                } else {
                    "⧉  Copy as CSV"
                };
                let button = egui::Button::new(
                    RichText::new(label)
                        .size(13.0)
                        .strong()
                        .color(if just_copied { GREEN } else { PURPLE }),
                )
                .min_size(Vec2::new(0.0, 30.0));
                if ui
                    .add_enabled(!filtered.is_empty(), button)
                    .on_hover_text("Copy the invoices currently in view to the clipboard")
                    .clicked()
                {
                    ctx.copy_text(invoices_csv(&filtered));
                    state.copied_at = now;
                }
            });
        });
    });
}

// ───────────────────────── Dialogs ─────────────────────────
fn dialog_frame(ctx: &egui::Context, progress: f32) -> egui::Frame {
    egui::Frame::window(&ctx.style())
        .inner_margin(Margin::same(22))
        .corner_radius(CornerRadius::same(14))
        .multiply_with_opacity(progress)
}

fn payment_window(
    ctx: &egui::Context,
    p: &mut PaymentDraft,
    cancel: &mut bool,
    save: &mut bool,
    progress: f32,
) {
    egui::Window::new("record_payment_window")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, (1.0 - progress) * 18.0))
        .frame(dialog_frame(ctx, progress))
        .show(ctx, |ui| {
            ui.set_opacity(progress);
            apply_style(ui);
            ui.set_width(520.0);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Record payment")
                            .size(22.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(&p.invoice.customer)
                            .size(15.0)
                            .strong()
                            .color(PURPLE),
                    );
                    ui.label(
                        RichText::new(format!("{}  ·  {}", p.invoice.number, p.invoice.date))
                            .size(13.0)
                            .color(MUTED),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui.button("✕").clicked() {
                        *cancel = true;
                    }
                });
            });

            ui.add_space(14.0);
            ui.columns(3, |cols| {
                small_metric(&mut cols[0], "INVOICE TOTAL", &money(p.invoice.total), TEXT);
                small_metric(&mut cols[1], "RECEIVED", &money(p.invoice.received), GREEN);
                small_metric(
                    &mut cols[2],
                    "BALANCE DUE",
                    &money(p.invoice.balance()),
                    ORANGE,
                );
            });

            let balance = p.invoice.balance();
            let parsed = parse_rupees(&p.amount);
            let amount_err = if p.amount.trim().is_empty() {
                None
            } else {
                match parsed {
                    None => Some("Enter a valid amount, e.g. 1250 or 1250.50"),
                    Some(v) if v > balance => Some("Amount is more than the balance due"),
                    _ => None,
                }
            };
            let date_ok = valid_date(&p.date);

            ui.add_space(16.0);
            ui.label(caps("AMOUNT RECEIVED (₹)"));
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 132.0).max(120.0);
                ui.add_sized(
                    [width, 38.0],
                    egui::TextEdit::singleline(&mut p.amount)
                        .hint_text("e.g. 1250.00")
                        .font(FontId::proportional(17.0)),
                );
                if ui
                    .add_sized([120.0, 38.0], egui::Button::new("Full balance"))
                    .clicked()
                {
                    p.amount = rupees_string(balance);
                }
            });
            if let Some(err) = amount_err {
                ui.label(RichText::new(err).size(12.5).color(RED));
            }

            ui.add_space(12.0);
            ui.columns(2, |cols| {
                field(&mut cols[0], "PAYMENT DATE", &mut p.date, "YYYY-MM-DD");
                if !date_ok {
                    cols[0].label(
                        RichText::new("Use the format YYYY-MM-DD")
                            .size(12.5)
                            .color(RED),
                    );
                }
                field(
                    &mut cols[1],
                    "REFERENCE / TRANSACTION ID",
                    &mut p.reference,
                    "Optional UPI or bank ref",
                );
            });

            ui.add_space(12.0);
            ui.label(caps("PAYMENT MODE"));
            ui.horizontal(|ui| {
                for (i, mode) in MODES.iter().enumerate() {
                    let selected = p.mode == i;
                    let t = ui.ctx().animate_bool_with_time(
                        Id::new(("inv_mode_chip", i)),
                        selected,
                        0.15,
                    );
                    let button = egui::Button::new(
                        RichText::new(*mode)
                            .size(13.5)
                            .strong()
                            .color(mix(MUTED, PURPLE, t)),
                    )
                    .fill(mix(CARD, PURPLE_BG, t))
                    .stroke(Stroke::new(1.0_f32, mix(BORDER, PURPLE, t)))
                    .corner_radius(CornerRadius::same(8))
                    .min_size(Vec2::new(0.0, 34.0));
                    if ui.add(button).clicked() {
                        p.mode = i;
                    }
                }
            });

            ui.add_space(12.0);
            ui.label(caps("NOTES (OPTIONAL)"));
            ui.add(
                egui::TextEdit::multiline(&mut p.notes)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .hint_text("Collection note or receipt details"),
            );

            if !p.error.is_empty() {
                ui.add_space(8.0);
                ui.label(RichText::new(&p.error).size(13.0).strong().color(RED));
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            let valid = parsed.is_some_and(|v| v > 0 && v <= balance) && date_ok;
            let remaining = parsed.map_or(balance, |v| (balance - v).max(0));

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(caps("BALANCE AFTER PAYMENT"));
                    ui.label(
                        RichText::new(money(remaining))
                            .size(17.0)
                            .strong()
                            .color(if remaining == 0 { GREEN } else { ORANGE }),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            valid,
                            egui::Button::new(
                                RichText::new("Save receipt")
                                    .size(14.5)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(PURPLE)
                            .min_size(Vec2::new(140.0, 38.0)),
                        )
                        .clicked()
                    {
                        *save = true;
                    }
                    if ui
                        .add(egui::Button::new("Cancel").min_size(Vec2::new(90.0, 38.0)))
                        .clicked()
                    {
                        *cancel = true;
                    }
                });
            });
        });
}

fn history_window(
    ctx: &egui::Context,
    invoice: Option<&Invoice>,
    receipts: &[Receipt],
    close: &mut bool,
    progress: f32,
) {
    egui::Window::new("payment_history_window")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, (1.0 - progress) * 18.0))
        .frame(dialog_frame(ctx, progress))
        .show(ctx, |ui| {
            ui.set_opacity(progress);
            apply_style(ui);
            ui.set_width(500.0);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Payment history")
                            .size(22.0)
                            .strong()
                            .color(TEXT),
                    );
                    if let Some(invoice) = invoice {
                        ui.label(
                            RichText::new(&invoice.customer)
                                .size(15.0)
                                .strong()
                                .color(PURPLE),
                        );
                        ui.label(
                            RichText::new(format!("{}  ·  {}", invoice.number, invoice.date))
                                .size(13.0)
                                .color(MUTED),
                        );
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui.button("✕").clicked() {
                        *close = true;
                    }
                });
            });

            if let Some(invoice) = invoice {
                ui.add_space(14.0);
                ui.columns(3, |cols| {
                    small_metric(&mut cols[0], "INVOICE TOTAL", &money(invoice.total), TEXT);
                    small_metric(&mut cols[1], "RECEIVED", &money(invoice.received), GREEN);
                    small_metric(
                        &mut cols[2],
                        "BALANCE DUE",
                        &money(invoice.balance()),
                        ORANGE,
                    );
                });
            }

            ui.add_space(14.0);

            if receipts.is_empty() {
                ui.add_space(16.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("No payments recorded yet")
                            .size(15.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.label(
                        RichText::new("Use “Record payment” on the invoice row.")
                            .size(13.0)
                            .color(MUTED),
                    );
                });
                ui.add_space(16.0);
            } else {
                ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for (index, receipt) in receipts.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(format!("{}  ·  {}", receipt.date, receipt.mode))
                                        .size(14.0)
                                        .strong()
                                        .color(TEXT),
                                );
                                if !receipt.reference.is_empty() {
                                    ui.label(
                                        RichText::new(format!("Ref: {}", receipt.reference))
                                            .size(12.5)
                                            .color(MUTED),
                                    );
                                }
                                if !receipt.notes.is_empty() {
                                    ui.label(RichText::new(&receipt.notes).size(12.5).color(MUTED));
                                }
                            });
                            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                ui.label(
                                    RichText::new(money(receipt.amount))
                                        .size(15.0)
                                        .strong()
                                        .color(GREEN),
                                );
                            });
                        });
                        if index + 1 < receipts.len() {
                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);
                        }
                    }
                });
            }

            ui.add_space(14.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(egui::Button::new("Close").min_size(Vec2::new(100.0, 36.0)))
                    .clicked()
                {
                    *close = true;
                }
            });
        });
}

// ───────────────────────── Data loading ─────────────────────────
fn load(app: &AsterApp, state: &mut State) {
    state.year = app.selected_year;
    state.loaded = true;
    state.invoice_detail = None;
    state.error.clear();

    let Some(db) = app.db.as_ref() else {
        state.error = "Database connection is unavailable.".into();
        return;
    };

    let connection = match db.year_connection(state.year) {
        Ok(connection) => connection,
        Err(error) => {
            state.error = error;
            return;
        }
    };

    let result = connection
        .prepare(
            "SELECT id, invoice_no, invoice_date, customer_name_snap,
                    total_paise, received_paise
             FROM sales_invoices
             WHERE status = 'posted'
             ORDER BY invoice_date DESC, id DESC
             LIMIT 5000",
        )
        .map_err(|error| error.to_string())
        .and_then(|mut statement| {
            statement
                .query_map([], |row| {
                    Ok(Invoice {
                        id: row.get(0)?,
                        number: row.get(1)?,
                        date: row.get(2)?,
                        customer: row.get(3)?,
                        total: row.get(4)?,
                        received: row.get(5)?,
                    })
                })
                .map_err(|error| error.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())
        });

    match result {
        Ok(invoices) => state.invoices = invoices,
        Err(error) => state.error = format!("Could not read invoices: {error}"),
    }
}

fn load_history(app: &AsterApp, state: &mut State, invoice_id: i64) {
    state.history.clear();
    state.error.clear();

    let Some(db) = app.db.as_ref() else {
        state.error = "Database connection is unavailable.".into();
        return;
    };

    let connection = match db.year_connection(app.selected_year) {
        Ok(connection) => connection,
        Err(error) => {
            state.error = error;
            return;
        }
    };

    let result = connection
        .prepare(
            "SELECT payment_date, amount_paise, payment_mode, reference_no, notes
             FROM sales_payments
             WHERE invoice_id = ?1
             ORDER BY payment_date DESC, id DESC",
        )
        .and_then(|mut statement| {
            statement
                .query_map([invoice_id], |row| {
                    Ok(Receipt {
                        date: row.get(0)?,
                        amount: row.get(1)?,
                        mode: row.get(2)?,
                        reference: row.get(3)?,
                        notes: row.get(4)?,
                    })
                })
                .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        });

    match result {
        Ok(receipts) => state.history = receipts,
        Err(error) => state.error = format!("Could not load payment history: {error}"),
    }
}

fn load_invoice_detail(app: &AsterApp, state: &mut State, invoice_id: i64) {
    state.error.clear();
    state.invoice_detail = None;

    let result = app
        .db
        .as_ref()
        .ok_or_else(|| "Database connection is unavailable.".to_owned())
        .and_then(|db| db.sales_invoice_detail(app.selected_year, invoice_id));

    match result {
        Ok(detail) => state.invoice_detail = Some(detail),
        Err(error) => state.error = format!("Could not load invoice details: {error}"),
    }
}

fn invoice_detail_window(
    ctx: &egui::Context,
    invoice: &crate::database::SalesInvoiceDetail,
    close: &mut bool,
    progress: f32,
) {
    egui::Window::new("sales_invoice_detail_window")
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .order(egui::Order::Foreground)
        .default_size(Vec2::new(900.0, 680.0))
        .min_size(Vec2::new(620.0, 420.0))
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, (1.0 - progress) * 18.0))
        .frame(dialog_frame(ctx, progress))
        .show(ctx, |ui| {
            ui.set_opacity(progress);
            apply_style(ui);
            ui.set_min_width(580.0);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("INVOICE DETAILS")
                            .size(11.0)
                            .strong()
                            .color(PURPLE),
                    );
                    ui.label(
                        RichText::new(&invoice.invoice_no)
                            .size(23.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.label(
                        RichText::new(format!("{}  ·  {}", invoice.invoice_date, invoice.status))
                            .size(13.0)
                            .color(MUTED),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    if ui.button("✕ Close").clicked() {
                        *close = true;
                    }
                });
            });

            ui.add_space(12.0);
            egui::Frame::new()
                .fill(ROW_ALT)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(14))
                .show(ui, |ui| {
                    ui.label(RichText::new("BILL TO").size(10.5).strong().color(PURPLE));
                    ui.label(
                        RichText::new(&invoice.customer_name)
                            .size(16.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if !invoice.customer_gstin.is_empty() {
                            ui.label(
                                RichText::new(format!("GSTIN: {}", invoice.customer_gstin))
                                    .size(12.5)
                                    .color(MUTED),
                            );
                        }
                        if !invoice.customer_state_code.is_empty() {
                            let state_label = state_name(&invoice.customer_state_code)
                                .map(|name| format!("{} · {name}", invoice.customer_state_code))
                                .unwrap_or_else(|| invoice.customer_state_code.clone());
                            ui.label(
                                RichText::new(format!("State: {state_label}"))
                                    .size(12.5)
                                    .color(MUTED),
                            );
                        }
                    });
                });

            ui.add_space(14.0);
            ui.label(RichText::new("ITEMS").size(11.0).strong().color(PURPLE));
            ui.add_space(6.0);

            ScrollArea::vertical().max_height(330.0).show(ui, |ui| {
                egui::Grid::new("invoice_detail_items_grid")
                    .striped(true)
                    .min_col_width(38.0)
                    .show(ui, |ui| {
                        for heading in [
                            "#",
                            "Product / HSN",
                            "Batch allocations",
                            "Qty",
                            "Free",
                            "Rate",
                            "Taxable",
                        ] {
                            ui.label(RichText::new(heading).size(11.5).strong().color(MUTED));
                        }
                        ui.end_row();

                        for item in &invoice.items {
                            ui.label(item.line_no.to_string());
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&item.product_name).strong().color(TEXT));
                                ui.label(
                                    RichText::new(format!(
                                        "HSN {} · GST {:.2}%",
                                        item.hsn,
                                        item.gst_basis_points as f64 / 100.0
                                    ))
                                    .size(11.5)
                                    .color(MUTED),
                                );
                                if !item.manufacturer.is_empty() || !item.unit.is_empty() {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · {}",
                                            item.manufacturer, item.unit
                                        ))
                                        .size(11.5)
                                        .color(MUTED),
                                    );
                                }
                                ui.label(
                                    RichText::new(format!(
                                        "Discount {:.2}%",
                                        item.discount_basis_points as f64 / 100.0
                                    ))
                                    .size(11.5)
                                    .color(MUTED),
                                );
                            });
                            ui.vertical(|ui| {
                                if item.paid_batches.is_empty() && item.free_batches.is_empty() {
                                    ui.label(
                                        RichText::new("No batch allocation recorded")
                                            .size(11.5)
                                            .color(MUTED),
                                    );
                                }
                                for batch in &item.paid_batches {
                                    ui.label(
                                        RichText::new(format!(
                                            "Paid · {} · Exp {} · {:.3} · MRP {}",
                                            batch.batch_no,
                                            batch.expiry_date,
                                            batch.quantity_milli as f64 / 1000.0,
                                            money(batch.mrp_paise)
                                        ))
                                        .size(11.5)
                                        .color(TEXT),
                                    );
                                }
                                for batch in &item.free_batches {
                                    ui.label(
                                        RichText::new(format!(
                                            "Free · {} · Exp {} · {:.3} · MRP {}",
                                            batch.batch_no,
                                            batch.expiry_date,
                                            batch.quantity_milli as f64 / 1000.0,
                                            money(batch.mrp_paise)
                                        ))
                                        .size(11.5)
                                        .color(GREEN),
                                    );
                                }
                            });
                            ui.label(format!("{:.3}", item.quantity_milli as f64 / 1000.0));
                            ui.label(format!("{:.3}", item.free_quantity_milli as f64 / 1000.0));
                            ui.label(money(item.rate_paise));
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(money(item.taxable_paise))
                                        .strong()
                                        .color(TEXT),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "CGST {} · SGST {} · IGST {}",
                                        money(item.cgst_paise),
                                        money(item.sgst_paise),
                                        money(item.igst_paise)
                                    ))
                                    .size(10.5)
                                    .color(MUTED),
                                );
                            });
                            ui.end_row();
                        }
                    });
            });

            ui.add_space(12.0);
            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    if !invoice.payment_mode.is_empty() {
                        ui.label(
                            RichText::new(format!("Payment mode: {}", invoice.payment_mode))
                                .size(12.5)
                                .color(MUTED),
                        );
                    }
                    if !invoice.notes.is_empty() {
                        ui.label(
                            RichText::new(format!("Notes: {}", invoice.notes))
                                .size(12.5)
                                .color(MUTED),
                        );
                    }
                });
                columns[1].with_layout(Layout::right_to_left(Align::Min), |ui| {
                    ui.vertical(|ui| {
                        ui.label(format!(
                            "Taxable amount     {}",
                            money(invoice.taxable_paise)
                        ));
                        ui.label(format!(
                            "CGST                 {}",
                            money(invoice.cgst_paise)
                        ));
                        ui.label(format!(
                            "SGST                 {}",
                            money(invoice.sgst_paise)
                        ));
                        ui.label(format!(
                            "IGST                  {}",
                            money(invoice.igst_paise)
                        ));
                        ui.separator();
                        ui.label(
                            RichText::new(format!("GRAND TOTAL   {}", money(invoice.total_paise)))
                                .size(16.0)
                                .strong()
                                .color(TEXT),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Received          {}",
                                money(invoice.received_paise)
                            ))
                            .color(GREEN),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Balance due     {}",
                                money(
                                    invoice
                                        .total_paise
                                        .saturating_sub(invoice.received_paise)
                                        .max(0)
                                )
                            ))
                            .strong()
                            .color(ORANGE),
                        );
                    });
                });
            });
        });
}

// ───────────────────────── Charts ─────────────────────────
struct Seg {
    label: &'static str,
    value: f32,
    color: Color32,
    filter: Filter,
}

fn arc_points(center: Pos2, radius: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    let steps = (((a1 - a0).abs() * radius) / 3.0).ceil().max(2.0) as usize;
    (0..=steps)
        .map(|k| {
            let a = a0 + (a1 - a0) * k as f32 / steps as f32;
            center + Vec2::new(a.cos(), a.sin()) * radius
        })
        .collect()
}

/// Donut chart. `grow` (0→1) sweeps the segments in; hovering a segment
/// thickens it and shows its figures in the centre.
fn donut(ui: &mut Ui, rect: Rect, resp: &Response, segs: &[Seg], count: usize, grow: f32) {
    let ctx = ui.ctx().clone();
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let thickness = 24.0_f32;
    let radius = rect.width().min(rect.height()) / 2.0 - thickness / 2.0 - 8.0;
    let total: f32 = segs.iter().map(|s| s.value).sum();

    // background track
    painter.circle_stroke(center, radius, Stroke::new(thickness, PURPLE_BG));

    let mut hovered: Option<usize> = None;
    if total > 0.0 {
        let nonzero = segs.iter().filter(|s| s.value > 0.0).count();
        let gap = if nonzero > 1 { 0.06 * grow } else { 0.0 };

        if grow >= 0.999 {
            hovered = resp.hover_pos().and_then(|p| {
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
                    let sweep = s.value / total * TAU;
                    if s.value > 0.0 && ang >= acc && ang < acc + sweep {
                        return Some(i);
                    }
                    acc += sweep;
                }
                None
            });
        }

        let mut start = -PI / 2.0;
        for (i, seg) in segs.iter().enumerate() {
            let sweep = seg.value / total * TAU * grow;
            let extra = ctx.animate_value_with_time(
                Id::new(("inv_donut_seg", i)),
                if hovered == Some(i) { 5.0 } else { 0.0 },
                0.12,
            );
            if seg.value > 0.0 && sweep > 0.001 {
                let a0 = start + gap / 2.0;
                let a1 = (start + sweep - gap / 2.0).max(a0 + 0.001);
                painter.add(Shape::line(
                    arc_points(center, radius, a0, a1),
                    Stroke::new(thickness + extra, seg.color),
                ));
            }
            start += sweep;
        }
    }

    // centre text: total, or the hovered segment
    let (top, bottom, color) = match hovered {
        Some(i) => (
            (segs[i].value as usize).to_string(),
            segs[i].label.to_owned() + " invoices",
            segs[i].color,
        ),
        None => (count.to_string(), "invoices".to_owned(), TEXT),
    };
    painter.text(
        center - Vec2::new(0.0, 9.0),
        Align2::CENTER_CENTER,
        top,
        FontId::proportional(30.0),
        color,
    );
    painter.text(
        center + Vec2::new(0.0, 17.0),
        Align2::CENTER_CENTER,
        bottom,
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
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(14.0));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(14.0));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(12.0));
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
        ui.set_min_height(86.0);
        ui.horizontal(|ui| {
            dot(ui, accent);
            ui.label(caps(label));
        });
        ui.add_space(4.0);
        ui.label(RichText::new(value).size(24.0).strong().color(TEXT));
        ui.label(RichText::new(hint).size(12.5).color(MUTED));
    });
}

fn small_metric(ui: &mut Ui, label: &str, value: &str, color: Color32) {
    egui::Frame::new()
        .fill(PURPLE_BG)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(label).size(10.5).strong().color(MUTED));
            ui.label(RichText::new(value).size(15.0).strong().color(color));
        });
}

fn field(ui: &mut Ui, label: &str, value: &mut String, hint: &str) {
    ui.label(caps(label));
    ui.add_sized(
        [ui.available_width(), 34.0],
        egui::TextEdit::singleline(value).hint_text(hint),
    );
}

fn dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 5.0, color);
}

fn legend_item(ui: &mut Ui, color: Color32, label: &str) {
    ui.label(RichText::new(label).size(12.5).color(MUTED));
    dot(ui, color);
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

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() > max {
        let cut: String = text.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    } else {
        text.to_owned()
    }
}

// ───────────────────────── Dates / CSV ─────────────────────────
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = (if m <= 2 { y + 1 } else { y }) as i32;
    (y, m, d)
}

/// Earliest invoice date (ISO `YYYY-MM-DD`) a period includes, if it is limited.
fn period_cutoff(period: Period, today: (i32, u32, u32)) -> Option<String> {
    let (y, m, d) = today;
    let days_ago = |n: i64| {
        let (cy, cm, cd) = civil_from_days(days_from_civil(y, m, d) - n);
        format!("{cy:04}-{cm:02}-{cd:02}")
    };
    match period {
        Period::All => None,
        Period::ThisMonth => Some(format!("{y:04}-{m:02}-01")),
        Period::Last30 => Some(days_ago(30)),
        Period::Last90 => Some(days_ago(90)),
    }
}

fn csv_cell(value: &str) -> String {
    if value.contains(|c| matches!(c, ',' | '"' | '\n' | '\r')) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn invoices_csv(list: &[&Invoice]) -> String {
    let mut out = String::from("Invoice,Date,Customer,Status,Total,Received,Balance\n");
    for invoice in list {
        out.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            csv_cell(&invoice.number),
            invoice.date,
            csv_cell(&invoice.customer),
            invoice.status().0,
            rupees_string(invoice.total),
            rupees_string(invoice.received),
            rupees_string(invoice.balance()),
        ));
    }
    out
}

// ───────────────────────── Formatting / parsing ─────────────────────────
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

fn rupees_string(paise: i64) -> String {
    format!("{}.{:02}", paise / 100, paise % 100)
}

fn valid_date(input: &str) -> bool {
    let b = input.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    if !(digits(0..4) && digits(5..7) && digits(8..10)) {
        return false;
    }
    let month: u32 = input[5..7].parse().unwrap_or(0);
    let day: u32 = input[8..10].parse().unwrap_or(0);
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

fn parse_rupees(input: &str) -> Option<i64> {
    let cleaned = input.trim().replace(',', "");
    if cleaned.is_empty() || cleaned.starts_with('-') {
        return None;
    }

    let mut parts = cleaned.split('.');
    let whole = parts.next()?.parse::<i64>().ok()?;
    let fraction = parts.next().unwrap_or("");

    if parts.next().is_some()
        || fraction.len() > 2
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }

    let paise = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<i64>().ok()? * 10,
        2 => fraction.parse::<i64>().ok()?,
        _ => return None,
    };

    if whole < 0 {
        return None;
    }

    whole
        .checked_mul(100)?
        .checked_add(paise)
        .filter(|amount| *amount > 0)
}
