use eframe::egui::{
    self, Align, Color32, CornerRadius, Id, Layout, Margin, RichText, ScrollArea, Sense, Stroke,
    Vec2,
};
use crate::utils::currency::format_inr;
use crate::{app::AsterApp, date::local_date};

const TEXT: Color32 = Color32::from_rgb(39, 35, 48);
const MUTED: Color32 = Color32::from_rgb(112, 106, 124);
const PURPLE: Color32 = Color32::from_rgb(99, 68, 151);
const PURPLE_BG: Color32 = Color32::from_rgb(242, 238, 249);
const CARD: Color32 = Color32::WHITE;
const BORDER: Color32 = Color32::from_rgb(226, 222, 233);
const GREEN: Color32 = Color32::from_rgb(37, 125, 83);
const ORANGE: Color32 = Color32::from_rgb(174, 105, 19);
const RED: Color32 = Color32::from_rgb(177, 58, 58);

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Filter {
    #[default]
    All,
    Pending,
    Unpaid,
    Partial,
    Paid,
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

    fn status(&self) -> &'static str {
        if self.balance() == 0 {
            "Paid"
        } else if self.received > 0 {
            "Partially paid"
        } else {
            "Unpaid"
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
    invoices: Vec<Invoice>,
    payment: Option<PaymentDraft>,
    history_for: Option<i64>,
    history: Vec<Receipt>,
    error: String,
    message: String,
}

pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let state_id = Id::new("aster_invoice_register_state");
    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<State>(state_id).unwrap_or_default());

    if !state.loaded || state.year != app.selected_year {
        load(app, &mut state);
    }

    ScrollArea::vertical()
        .id_salt("invoice_register_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.add_space(4.0);

            // Page heading
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("TRANSACTIONS  /  RECEIVABLES")
                            .size(10.0)
                            .strong()
                            .color(PURPLE),
                    );
                    ui.add_space(3.0);
                    ui.label(
                        RichText::new("Invoice Management")
                            .size(26.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.label(
                        RichText::new("Track balances, record collections and review receipts.")
                            .size(11.5)
                            .color(MUTED),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("↻  Refresh").clicked() {
                        load(app, &mut state);
                    }
                    ui.label(
                        RichText::new(format!(
                            "FY {}–{}",
                            app.selected_year,
                            app.selected_year + 1
                        ))
                        .size(10.5)
                        .color(MUTED),
                    );
                });
            });

            ui.add_space(16.0);

            // Summary metrics
            let total: i64 = state.invoices.iter().map(|i| i.total).sum();
            let received: i64 = state.invoices.iter().map(|i| i.received).sum();
            let outstanding: i64 = state.invoices.iter().map(Invoice::balance).sum();
            let unpaid = state
                .invoices
                .iter()
                .filter(|i| i.received == 0 && i.balance() > 0)
                .count();
            let pending = state.invoices.iter().filter(|i| i.balance() > 0).count();

            ui.columns(4, |cols| {
                metric(&mut cols[0], "INVOICED", &money(total), "Posted invoice value", PURPLE);
                metric(&mut cols[1], "COLLECTED", &money(received), "Amount received", GREEN);
                metric(
                    &mut cols[2],
                    "OUTSTANDING",
                    &money(outstanding),
                    "Balance to collect",
                    if outstanding > 0 { ORANGE } else { GREEN },
                );
                metric(&mut cols[3], "PENDING INVOICES", &pending.to_string(), &format!("{unpaid} unpaid"), RED);
            });

            ui.add_space(12.0);

            // Collection overview
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Collection overview").size(14.0).strong().color(TEXT));
                        ui.label(
                            RichText::new("Collected versus outstanding on posted invoices")
                                .size(10.5)
                                .color(MUTED),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let percent = if total > 0 {
                            received as f64 / total as f64 * 100.0
                        } else {
                            0.0
                        };
                        ui.label(
                            RichText::new(format!("{percent:.1}% collected"))
                                .size(12.0)
                                .strong()
                                .color(GREEN),
                        );
                    });
                });

                ui.add_space(10.0);
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 12.0), Sense::hover());
                ui.painter().rect_filled(rect, CornerRadius::same(6), PURPLE_BG);

                if total > 0 {
                    let width =
                        rect.width() * (received as f32 / total as f32).clamp(0.0, 1.0_f32);
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(rect.min, Vec2::new(width, rect.height())),
                        CornerRadius::same(6),
                        GREEN,
                    );
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    legend(ui, GREEN, "Collected", &money(received));
                    ui.add_space(16.0);
                    legend(ui, ORANGE, "Outstanding", &money(outstanding));
                });
            });

            ui.add_space(12.0);

            if !state.error.is_empty() {
                card(ui, |ui| {
                    ui.label(RichText::new("Could not load invoice data").strong().color(RED));
                    ui.label(RichText::new(&state.error).size(11.0).color(MUTED));
                    if ui.button("Retry").clicked() {
                        load(app, &mut state);
                    }
                });
            } else {
                // Invoice register
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Invoice register").size(15.0).strong().color(TEXT));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{} invoices", state.invoices.len()))
                                    .size(10.5)
                                    .color(MUTED),
                            );
                        });
                    });

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [ui.available_width().min(300.0), 30.0],
                            egui::TextEdit::singleline(&mut state.search)
                                .hint_text("⌕  Search invoice or customer…"),
                        );
                        ui.add_space(6.0);

                        for (label, filter) in [
                            ("All", Filter::All),
                            ("Pending", Filter::Pending),
                            ("Unpaid", Filter::Unpaid),
                            ("Partial", Filter::Partial),
                            ("Paid", Filter::Paid),
                        ] {
                            let selected = state.filter == filter;
                            let button = egui::Button::new(
                                RichText::new(label)
                                    .size(10.0)
                                    .strong()
                                    .color(if selected { PURPLE } else { MUTED }),
                            )
                            .fill(if selected { PURPLE_BG } else { CARD })
                            .stroke(Stroke::new(
                                1.0_f32,
                                if selected { PURPLE } else { BORDER },
                            ))
                            .corner_radius(CornerRadius::same(7));

                            if ui.add(button).clicked() {
                                state.filter = filter;
                            }
                        }
                    });

                    ui.add_space(12.0);

                    // Column headings
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("INVOICE / DATE").size(9.0).strong().color(MUTED));
                        ui.add_space(24.0);
                        ui.label(RichText::new("CUSTOMER").size(9.0).strong().color(MUTED));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new("BALANCE · RECEIVED / TOTAL")
                                    .size(9.0)
                                    .strong()
                                    .color(MUTED),
                            );
                        });
                    });
                    ui.add_space(5.0);
                    ui.separator();

                    let search = state.search.trim().to_lowercase();
                    let filtered: Vec<Invoice> = state
                        .invoices
                        .iter()
                        .filter(|invoice| {
                            let matches_search = search.is_empty()
                                || invoice.number.to_lowercase().contains(&search)
                                || invoice.customer.to_lowercase().contains(&search);

                            let matches_filter = match state.filter {
                                Filter::All => true,
                                Filter::Pending => invoice.balance() > 0,
                                Filter::Unpaid => {
                                    invoice.received == 0 && invoice.balance() > 0
                                }
                                Filter::Partial => {
                                    invoice.received > 0 && invoice.balance() > 0
                                }
                                Filter::Paid => invoice.balance() == 0,
                            };

                            matches_search && matches_filter
                        })
                        .cloned()
                        .collect();

                    let mut open_payment: Option<Invoice> = None;
                    let mut open_history: Option<i64> = None;

                    ScrollArea::vertical()
                        .id_salt("invoice_register_rows")
                        .max_height(360.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if filtered.is_empty() {
                                ui.add_space(22.0);
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new("No matching invoices")
                                            .strong()
                                            .color(TEXT),
                                    );
                                    ui.label(
                                        RichText::new("Try another search or payment filter.")
                                            .size(10.5)
                                            .color(MUTED),
                                    );
                                });
                                ui.add_space(22.0);
                            }

                            for invoice in &filtered {
                                ui.add_space(4.0);
                                egui::Frame::new()
                                    .fill(CARD)
                                    .stroke(Stroke::new(1.0_f32, BORDER))
                                    .corner_radius(CornerRadius::same(8))
                                    .inner_margin(Margin::symmetric(9, 8))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.allocate_ui_with_layout(
                                                Vec2::new(135.0, 42.0),
                                                Layout::top_down(Align::Min),
                                                |ui| {
                                                    ui.label(
                                                        RichText::new(&invoice.number)
                                                            .size(10.5)
                                                            .strong()
                                                            .color(TEXT),
                                                    );
                                                    ui.label(
                                                        RichText::new(&invoice.date)
                                                            .size(9.5)
                                                            .color(MUTED),
                                                    );
                                                },
                                            );

                                            ui.allocate_ui_with_layout(
                                                Vec2::new((ui.available_width() * 0.28).max(90.0), 42.0),
                                                Layout::top_down(Align::Min),
                                                |ui| {
                                                    ui.label(
                                                        RichText::new(&invoice.customer)
                                                            .size(10.5)
                                                            .color(TEXT),
                                                    );
                                                    let color = match invoice.status() {
                                                        "Paid" => GREEN,
                                                        "Partially paid" => ORANGE,
                                                        _ => RED,
                                                    };
                                                    ui.label(
                                                        RichText::new(invoice.status().to_uppercase())
                                                            .size(8.0)
                                                            .strong()
                                                            .color(color),
                                                    );
                                                },
                                            );

                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    ui.vertical(|ui| {
                                                        ui.label(
                                                            RichText::new(money(invoice.balance()))
                                                                .size(11.0)
                                                                .strong()
                                                                .color(if invoice.balance() > 0 {
                                                                    ORANGE
                                                                } else {
                                                                    GREEN
                                                                }),
                                                        );
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "{} received / {}",
                                                                money(invoice.received),
                                                                money(invoice.total)
                                                            ))
                                                            .size(9.0)
                                                            .color(MUTED),
                                                        );
                                                    });

                                                    if ui.small_button("History").clicked() {
                                                        open_history = Some(invoice.id);
                                                    }

                                                    if invoice.balance() > 0
                                                        && ui.small_button("＋ Payment").clicked()
                                                    {
                                                        open_payment = Some(invoice.clone());
                                                    }
                                                },
                                            );
                                        });
                                    });
                            }
                        });

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
                });
            }

            // Payment form
            if let Some(payment) = state.payment.as_mut() {
                ui.add_space(12.0);
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Record payment").size(15.0).strong().color(TEXT));
                            ui.label(
                                RichText::new(format!(
                                    "{}  ·  {}",
                                    payment.invoice.number, payment.invoice.customer
                                ))
                                .size(10.5)
                                .color(MUTED),
                            );
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Cancel").clicked() {
                                payment.error = "__CANCEL__".into();
                            }
                        });
                    });

                    ui.add_space(10.0);
                    ui.columns(3, |cols| {
                        small_metric(&mut cols[0], "INVOICE TOTAL", &money(payment.invoice.total));
                        small_metric(&mut cols[1], "RECEIVED", &money(payment.invoice.received));
                        small_metric(&mut cols[2], "BALANCE DUE", &money(payment.invoice.balance()));
                    });

                    ui.add_space(10.0);
                    ui.columns(2, |cols| {
                        field(&mut cols[0], "AMOUNT RECEIVED (₹)", &mut payment.amount, "e.g. 1250.00");
                        field(&mut cols[1], "PAYMENT DATE", &mut payment.date, "YYYY-MM-DD");

                        cols[0].add_space(8.0);
                        cols[0].label(RichText::new("PAYMENT MODE").size(9.0).strong().color(MUTED));
                        egui::ComboBox::from_id_salt("invoice_payment_mode")
                            .selected_text(
                                ["Cash", "UPI", "Bank transfer", "Cheque"]
                                    .get(payment.mode)
                                    .copied()
                                    .unwrap_or("Cash"),
                            )
                            .width(cols[0].available_width())
                            .show_ui(&mut cols[0], |ui| {
                                for (index, mode) in
                                    ["Cash", "UPI", "Bank transfer", "Cheque"].iter().enumerate()
                                {
                                    ui.selectable_value(&mut payment.mode, index, *mode);
                                }
                            });

                        cols[1].add_space(8.0);
                        field(
                            &mut cols[1],
                            "REFERENCE / TRANSACTION ID",
                            &mut payment.reference,
                            "Optional UPI or bank reference",
                        );
                    });

                    ui.add_space(8.0);
                    ui.label(RichText::new("NOTES (OPTIONAL)").size(9.0).strong().color(MUTED));
                    ui.add(
                        egui::TextEdit::multiline(&mut payment.notes)
                            .desired_rows(2)
                            .desired_width(f32::INFINITY)
                            .hint_text("Collection note or receipt details"),
                    );

                    if !payment.error.is_empty() && payment.error != "__CANCEL__" && payment.error != "__SAVE__" {
                        ui.add_space(5.0);
                        ui.label(RichText::new(&payment.error).size(10.5).color(RED));
                    }

                    ui.add_space(8.0);
                    let parsed = parse_rupees(&payment.amount);
                    let remaining = parsed
                        .map(|value| payment.invoice.balance().saturating_sub(value))
                        .unwrap_or(payment.invoice.balance());

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Balance after payment: {}",
                                money(remaining.max(0))
                            ))
                            .size(11.0)
                            .strong()
                            .color(if remaining <= 0 { GREEN } else { ORANGE }),
                        );

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let valid = parsed.is_some_and(|value| {
                                value > 0 && value <= payment.invoice.balance()
                            });

                            if ui
                                .add_enabled(
                                    valid,
                                    egui::Button::new(
                                        RichText::new("Save receipt")
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .fill(PURPLE)
                                    .corner_radius(CornerRadius::same(7)),
                                )
                                .clicked()
                            {
                                payment.error = "__SAVE__".into();
                            }
                        });
                    });
                });
            }

            // Receipt history
            if state.history_for.is_some() {
                ui.add_space(12.0);
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Payment history").size(14.0).strong().color(TEXT));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                state.history_for = None;
                                state.history.clear();
                            }
                        });
                    });

                    ui.add_space(8.0);

                    if state.history.is_empty() {
                        ui.label(
                            RichText::new("No payment receipts recorded for this invoice.")
                                .size(10.5)
                                .color(MUTED),
                        );
                    } else {
                        for (index, receipt) in state.history.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(format!("{}  ·  {}", receipt.date, receipt.mode))
                                            .size(10.5)
                                            .strong()
                                            .color(TEXT),
                                    );
                                    if !receipt.reference.is_empty() {
                                        ui.label(
                                            RichText::new(format!("Reference: {}", receipt.reference))
                                                .size(9.5)
                                                .color(MUTED),
                                        );
                                    }
                                    if !receipt.notes.is_empty() {
                                        ui.label(
                                            RichText::new(&receipt.notes).size(9.5).color(MUTED),
                                        );
                                    }
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(money(receipt.amount))
                                            .size(11.0)
                                            .strong()
                                            .color(GREEN),
                                    );
                                });
                            });
                            if index + 1 < state.history.len() {
                                ui.separator();
                            }
                        }
                    }
                });
            }

            if !state.message.is_empty() {
                ui.add_space(8.0);
                ui.label(RichText::new(&state.message).size(11.0).strong().color(GREEN));
            }

            ui.add_space(14.0);
        });

    // Handle form actions after UI borrows have ended.
    let action = state
        .payment
        .as_ref()
        .map(|payment| payment.error.clone())
        .unwrap_or_default();

    if action == "__CANCEL__" {
        state.payment = None;
    } else if action == "__SAVE__" {
        if let Some(payment) = state.payment.clone() {
            let amount = parse_rupees(&payment.amount).unwrap_or(0);
            let mode = ["Cash", "UPI", "Bank transfer", "Cheque"]
                .get(payment.mode)
                .copied()
                .unwrap_or("Cash");

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

fn card(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(11))
        .inner_margin(Margin::same(13))
        .show(ui, content);
}

fn metric(ui: &mut egui::Ui, label: &str, value: &str, hint: &str, accent: Color32) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(7.0), Sense::hover());
            ui.painter().circle_filled(rect.center(), 3.5, accent);
            ui.label(RichText::new(label).size(9.0).strong().color(MUTED));
        });
        ui.add_space(6.0);
        ui.label(RichText::new(value).size(18.0).strong().color(TEXT));
        ui.add_space(2.0);
        ui.label(RichText::new(hint).size(9.0).color(MUTED));
    });
}

fn small_metric(ui: &mut egui::Ui, label: &str, value: &str) {
    egui::Frame::new()
        .fill(PURPLE_BG)
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::same(9))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(8.5).strong().color(MUTED));
            ui.label(RichText::new(value).size(13.0).strong().color(TEXT));
        });
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    ui.label(RichText::new(label).size(9.0).strong().color(MUTED));
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(f32::INFINITY),
    );
}

fn legend(ui: &mut egui::Ui, color: Color32, label: &str, value: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(7.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 3.5, color);
        ui.label(
            RichText::new(format!("{label}  {value}"))
                .size(10.0)
                .color(MUTED),
        );
    });
}

fn money(paise: i64) -> String {
    format_inr(paise as f64 / 100.0)
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

    whole.checked_mul(100)?.checked_add(paise).filter(|amount| *amount > 0)
}