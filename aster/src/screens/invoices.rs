use crate::utils::currency::format_inr;
use crate::{app::AsterApp, date::local_date};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Id, Layout, Margin, Pos2, Rect, Response,
    RichText, ScrollArea, Sense, Shape, Stroke, TextStyle, Ui, Vec2,
};
use std::collections::BTreeMap;
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
const ORANGE: Color32 = Color32::from_rgb(190, 104, 8);
const ORANGE_BG: Color32 = Color32::from_rgb(253, 240, 221);
const RED: Color32 = Color32::from_rgb(190, 50, 50);
const RED_BG: Color32 = Color32::from_rgb(250, 229, 229);

const MODES: [&str; 4] = ["Cash", "UPI", "Bank transfer", "Cheque"];
const MONTHS: [&str; 12] = [
    "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "Jan", "Feb", "Mar",
];

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
    customer: Option<String>,
    invoices: Vec<Invoice>,
    payment: Option<PaymentDraft>,
    history_for: Option<i64>,
    history: Vec<Receipt>,
    error: String,
    message: String,
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

// ───────────────────────── Entry point ─────────────────────────
pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let state_id = Id::new("aster_invoice_register_state");
    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<State>(state_id).unwrap_or_default());

    if !state.loaded || state.year != app.selected_year {
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

    let mut open_payment: Option<Invoice> = None;
    let mut open_history: Option<i64> = None;

    ScrollArea::vertical()
        .id_salt("invoice_register_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            apply_style(ui);
            ui.add_space(6.0);

            header(ui, app, &mut state);
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
            ui.columns(4, |cols| {
                metric(
                    &mut cols[0],
                    "INVOICED",
                    &money(st.total),
                    "Posted invoice value",
                    PURPLE,
                );
                metric(
                    &mut cols[1],
                    "COLLECTED",
                    &money(st.received),
                    &format!("{:.1}% of invoiced", st.percent()),
                    GREEN,
                );
                metric(
                    &mut cols[2],
                    "OUTSTANDING",
                    &money(st.outstanding),
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

            ui.add_space(14.0);

            // Charts
            ui.columns(2, |cols| {
                donut_card(&mut cols[0], &mut state, &st);
                top_customers_card(&mut cols[1], &mut state, &scoped);
            });

            ui.add_space(14.0);
            monthly_card(ui, app.selected_year, &scoped);

            ui.add_space(14.0);
            register_card(
                ui,
                &mut state,
                &scoped,
                &customers,
                &mut open_payment,
                &mut open_history,
            );

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

    // Dialogs
    let ctx = ui.ctx().clone();
    let mut cancel = false;
    let mut save = false;
    if let Some(payment) = state.payment.as_mut() {
        payment_window(&ctx, payment, &mut cancel, &mut save);
    }
    if cancel {
        state.payment = None;
    }

    let mut close_history = false;
    if let Some(invoice_id) = state.history_for {
        let invoice = state.invoices.iter().find(|i| i.id == invoice_id).cloned();
        history_window(&ctx, invoice.as_ref(), &state.history, &mut close_history);
    }
    if close_history {
        state.history_for = None;
        state.history.clear();
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

    ui.ctx().data_mut(|d| d.insert_temp(state_id, state));
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

fn donut_card(ui: &mut Ui, state: &mut State, st: &Stats) {
    card(ui, |ui| {
        ui.set_min_height(250.0);
        section_title(
            ui,
            "Payment status",
            "Invoices by collection state · click to filter",
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
            donut(ui, rect, &resp, &segs, &count.to_string(), "invoices");

            ui.add_space(10.0);
            ui.vertical(|ui| {
                for seg in &segs {
                    ui.horizontal(|ui| {
                        dot(ui, seg.color);
                        let r = ui.add(
                            egui::Button::new(RichText::new(seg.label).size(14.0).color(TEXT))
                                .frame(false),
                        );
                        if r.on_hover_text("Filter the register").clicked() {
                            state.filter = seg.filter;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new((seg.value as usize).to_string())
                                    .size(14.0)
                                    .strong()
                                    .color(TEXT),
                            );
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
                    let w = rect.width() * (st.received as f32 / st.total as f32).clamp(0.0, 1.0);
                    ui.painter().rect_filled(
                        Rect::from_min_size(rect.min, Vec2::new(w, rect.height())),
                        CornerRadius::same(5),
                        GREEN,
                    );
                }
            });
        });
    });
}

fn top_customers_card(ui: &mut Ui, state: &mut State, scoped: &[Invoice]) {
    card(ui, |ui| {
        ui.set_min_height(250.0);
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
        for (name, amount) in &rows {
            let selected = state.customer.as_deref() == Some(name.as_str());
            ui.horizontal(|ui| {
                let text = RichText::new(truncate(name, 26))
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
                });
            });
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 8.0), Sense::hover());
            ui.painter()
                .rect_filled(rect, CornerRadius::same(4), PURPLE_BG);
            let w = rect.width() * (*amount as f32 / max as f32);
            ui.painter().rect_filled(
                Rect::from_min_size(rect.min, Vec2::new(w, rect.height())),
                CornerRadius::same(4),
                if selected { PURPLE } else { ORANGE },
            );
            ui.add_space(6.0);
        }
    });
}

fn monthly_card(ui: &mut Ui, year: i32, scoped: &[Invoice]) {
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

        let (rect, resp) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 240.0), Sense::hover());
        let painter = ui.painter_at(rect.expand(2.0));
        let plot = Rect::from_min_max(
            Pos2::new(rect.left() + 62.0, rect.top() + 8.0),
            Pos2::new(rect.right() - 4.0, rect.bottom() - 28.0),
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

        let bar_w = (group_w * 0.28).clamp(6.0, 22.0);
        for i in 0..12 {
            let cx = plot.left() + group_w * (i as f32 + 0.5);

            if hovered == Some(i) {
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(cx - group_w / 2.0 + 2.0, plot.top()),
                        Pos2::new(cx + group_w / 2.0 - 2.0, plot.bottom()),
                    ),
                    CornerRadius::same(6),
                    PURPLE_BG,
                );
            }

            for (k, (value, color)) in [(invoiced[i], PURPLE), (collected[i], GREEN)]
                .into_iter()
                .enumerate()
            {
                if value <= 0 {
                    continue;
                }
                let h = (plot.height() * value as f32 / top as f32).max(2.0);
                let x0 = if k == 0 { cx - bar_w - 1.5 } else { cx + 1.5 };
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(x0, plot.bottom() - h),
                        Pos2::new(x0 + bar_w, plot.bottom()),
                    ),
                    CornerRadius {
                        nw: 4,
                        ne: 4,
                        sw: 0,
                        se: 0,
                    },
                    color,
                );
            }

            painter.text(
                Pos2::new(cx, plot.bottom() + 8.0),
                Align2::CENTER_TOP,
                MONTHS[i],
                FontId::proportional(12.0),
                if hovered == Some(i) { TEXT } else { MUTED },
            );
        }

        if let Some(i) = hovered {
            let y = if i < 9 { year } else { year + 1 };
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
            });
        }
    });
}

fn register_card(
    ui: &mut Ui,
    state: &mut State,
    scoped: &[Invoice],
    customers: &[String],
    open_payment: &mut Option<Invoice>,
    open_history: &mut Option<i64>,
) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                section_title(
                    ui,
                    "Invoice register",
                    "Search, pick a customer, filter by status and record payments",
                );
            });
        });
        ui.add_space(14.0);

        // Toolbar
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(caps("SEARCH"));
                ui.add_sized(
                    [300.0, 34.0],
                    egui::TextEdit::singleline(&mut state.search)
                        .hint_text("Invoice number or customer name"),
                );
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(caps("CUSTOMER"));
                let current = state
                    .customer
                    .clone()
                    .unwrap_or_else(|| "All customers".to_owned());
                egui::ComboBox::from_id_salt("invoice_customer_filter")
                    .width(280.0)
                    .selected_text(RichText::new(truncate(&current, 32)).size(14.0))
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

            let active =
                !state.search.is_empty() || state.customer.is_some() || state.filter != Filter::All;
            if active {
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(caps(" "));
                    if ui.button("Clear filters").clicked() {
                        state.search.clear();
                        state.customer = None;
                        state.filter = Filter::All;
                    }
                });
            }
        });

        // Search applied (status chips show counts for the current search)
        let needle = state.search.trim().to_lowercase();
        let searched: Vec<&Invoice> = scoped
            .iter()
            .filter(|i| {
                needle.is_empty()
                    || i.number.to_lowercase().contains(&needle)
                    || i.customer.to_lowercase().contains(&needle)
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
                let button = egui::Button::new(
                    RichText::new(format!("{label}   {n}"))
                        .size(13.0)
                        .strong()
                        .color(if selected { PURPLE } else { MUTED }),
                )
                .fill(if selected { PURPLE_BG } else { CARD })
                .stroke(Stroke::new(1.0_f32, if selected { PURPLE } else { BORDER }))
                .corner_radius(CornerRadius::same(16))
                .min_size(Vec2::new(0.0, 30.0));
                if ui.add(button).clicked() {
                    state.filter = filter;
                }
            }
        });

        let filtered: Vec<&Invoice> = searched
            .iter()
            .copied()
            .filter(|i| matches_filter(i, state.filter))
            .collect();

        ui.add_space(14.0);

        // Column widths
        let available = ui.available_width().max(700.0);
        let actions_w = 190.0;
        let rem = (available - actions_w).max(510.0);

        let w = [
            rem * 0.16,
            rem * 0.11,
            rem * 0.23,
            rem * 0.13,
            rem * 0.12,
            rem * 0.12,
            rem * 0.13,
        ];

        // Header row
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
                    for (i, (label, align)) in [
                        ("INVOICE", Align::Min),
                        ("DATE", Align::Min),
                        ("CUSTOMER", Align::Min),
                        ("STATUS", Align::Min),
                        ("TOTAL", Align::Max),
                        ("RECEIVED", Align::Max),
                        ("BALANCE", Align::Max),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        cell(ui, w[i], 30.0, align, |ui| {
                            ui.label(caps(label));
                        });
                    }
                    cell(ui, actions_w, 30.0, Align::Max, |ui| {
                        ui.label(caps("ACTIONS"));
                    });
                });
            });

        // Rows
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
                ScrollArea::vertical()
                    .id_salt("invoice_register_rows")
                    .max_height(460.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;

                        if filtered.is_empty() {
                            ui.add_space(30.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new("No matching invoices")
                                        .size(16.0)
                                        .strong()
                                        .color(TEXT),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(
                                        "Try a different search, customer or status filter.",
                                    )
                                    .size(13.0)
                                    .color(MUTED),
                                );
                            });
                            ui.add_space(30.0);
                        }

                        for (idx, invoice) in filtered.iter().enumerate() {
                            let bg = if idx % 2 == 0 { CARD } else { ROW_ALT };
                            egui::Frame::new()
                                .fill(bg)
                                .inner_margin(Margin::symmetric(12, 4))
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 0.0;

                                        cell(ui, w[0], 40.0, Align::Min, |ui| {
                                            ui.label(
                                                RichText::new(&invoice.number)
                                                    .size(14.0)
                                                    .strong()
                                                    .color(TEXT),
                                            );
                                        });
                                        cell(ui, w[1], 40.0, Align::Min, |ui| {
                                            ui.label(
                                                RichText::new(&invoice.date)
                                                    .size(13.5)
                                                    .color(MUTED),
                                            );
                                        });
                                        cell(ui, w[2], 40.0, Align::Min, |ui| {
                                            let r = ui.add(
                                                egui::Button::new(
                                                    RichText::new(truncate(&invoice.customer, 30))
                                                        .size(14.0)
                                                        .color(TEXT),
                                                )
                                                .frame(false),
                                            );
                                            if r.on_hover_text("Show only this customer").clicked()
                                            {
                                                state.customer = Some(invoice.customer.clone());
                                            }
                                        });
                                        cell(ui, w[3], 40.0, Align::Min, |ui| {
                                            let (text, fg, bg) = invoice.status();
                                            badge(ui, text, fg, bg);
                                        });
                                        cell(ui, w[4], 40.0, Align::Max, |ui| {
                                            ui.label(
                                                RichText::new(money(invoice.total))
                                                    .size(14.0)
                                                    .color(TEXT),
                                            );
                                        });
                                        cell(ui, w[5], 40.0, Align::Max, |ui| {
                                            ui.label(
                                                RichText::new(money(invoice.received))
                                                    .size(14.0)
                                                    .color(GREEN),
                                            );
                                        });
                                        cell(ui, w[6], 40.0, Align::Max, |ui| {
                                            ui.label(
                                                RichText::new(money(invoice.balance()))
                                                    .size(14.0)
                                                    .strong()
                                                    .color(if invoice.balance() > 0 {
                                                        ORANGE
                                                    } else {
                                                        MUTED
                                                    }),
                                            );
                                        });

                                        ui.allocate_ui_with_layout(
                                            Vec2::new(actions_w, 40.0),
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                ui.spacing_mut().item_spacing.x = 8.0;
                                                if ui
                                                    .add(
                                                        egui::Button::new(
                                                            RichText::new("History").size(13.0),
                                                        )
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
                                                    *open_payment = Some((*invoice).clone());
                                                }
                                            },
                                        );
                                    });
                                });
                        }
                    });
            });

        ui.add_space(8.0);
        let shown_balance: i64 = filtered.iter().map(|i| i.balance()).sum();
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
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("Outstanding in view: {}", money(shown_balance)))
                        .size(13.0)
                        .strong()
                        .color(if shown_balance > 0 { ORANGE } else { GREEN }),
                );
            });
        });
    });
}

// ───────────────────────── Dialogs ─────────────────────────
fn payment_window(ctx: &egui::Context, p: &mut PaymentDraft, cancel: &mut bool, save: &mut bool) {
    egui::Window::new("record_payment_window")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(
            egui::Frame::window(&ctx.style())
                .inner_margin(Margin::same(22))
                .corner_radius(CornerRadius::same(14)),
        )
        .show(ctx, |ui| {
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
                    let button = egui::Button::new(
                        RichText::new(*mode).size(13.5).strong().color(if selected {
                            PURPLE
                        } else {
                            MUTED
                        }),
                    )
                    .fill(if selected { PURPLE_BG } else { CARD })
                    .stroke(Stroke::new(1.0_f32, if selected { PURPLE } else { BORDER }))
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
) {
    egui::Window::new("payment_history_window")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(
            egui::Frame::window(&ctx.style())
                .inner_margin(Margin::same(22))
                .corner_radius(CornerRadius::same(14)),
        )
        .show(ctx, |ui| {
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

fn donut(
    ui: &mut Ui,
    rect: Rect,
    resp: &Response,
    segs: &[Seg],
    center_top: &str,
    center_bottom: &str,
) {
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let thickness = 24.0_f32;
    let radius = rect.width().min(rect.height()) / 2.0 - thickness / 2.0 - 8.0;
    let total: f32 = segs.iter().map(|s| s.value).sum();

    if total <= 0.0 {
        painter.circle_stroke(center, radius, Stroke::new(thickness, PURPLE_BG));
    } else {
        let nonzero = segs.iter().filter(|s| s.value > 0.0).count();
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
                let sweep = s.value / total * TAU;
                if s.value > 0.0 && ang >= acc && ang < acc + sweep {
                    return Some(i);
                }
                acc += sweep;
            }
            None
        });

        let mut start = -PI / 2.0;
        for (i, seg) in segs.iter().enumerate() {
            if seg.value <= 0.0 {
                continue;
            }
            let sweep = seg.value / total * TAU;
            let width = if hovered == Some(i) {
                thickness + 5.0
            } else {
                thickness
            };
            if nonzero == 1 {
                painter.circle_stroke(center, radius, Stroke::new(width, seg.color));
            } else {
                let points =
                    arc_points(center, radius, start + gap / 2.0, start + sweep - gap / 2.0);
                painter.add(Shape::line(points, Stroke::new(width, seg.color)));
            }
            start += sweep;
        }

        if let Some(i) = hovered {
            let seg = &segs[i];
            resp.clone().on_hover_text(format!(
                "{}: {} invoices ({:.0}%)",
                seg.label,
                seg.value as usize,
                seg.value / total * 100.0
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
