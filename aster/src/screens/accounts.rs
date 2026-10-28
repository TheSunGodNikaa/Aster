use crate::app::AsterApp;
use crate::utils::currency::format_inr;
use eframe::egui::{
    self, Align, Color32, CornerRadius, Id, Layout, Margin, RichText, ScrollArea, Sense, Stroke,
    Vec2,
};
use rusqlite::params;

const BG: Color32 = Color32::from_rgb(247, 246, 250);
const CARD: Color32 = Color32::WHITE;
const TEXT: Color32 = Color32::from_rgb(39, 35, 48);
const MUTED: Color32 = Color32::from_rgb(112, 106, 124);
const PURPLE: Color32 = Color32::from_rgb(99, 68, 151);
const LIGHT_PURPLE: Color32 = Color32::from_rgb(242, 238, 249);
const BORDER: Color32 = Color32::from_rgb(226, 222, 233);
const GREEN: Color32 = Color32::from_rgb(37, 125, 83);
const ORANGE: Color32 = Color32::from_rgb(174, 105, 19);
const RED: Color32 = Color32::from_rgb(177, 58, 58);
const BLUE: Color32 = Color32::from_rgb(65, 112, 181);

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum AccountType {
    #[default]
    Customers,
    Suppliers,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum InvoiceFilter {
    #[default]
    All,
    Outstanding,
    Paid,
}

#[derive(Clone, Default)]
struct Account {
    id: i64,
    name: String,
    phone: String,
    email: String,
    gstin: String,
    city: String,
    active: bool,
    opening: i64,
    credit_limit: i64,
    sales: i64,
    received: i64,
    purchases: i64,
}

impl Account {
    fn balance(&self, kind: AccountType) -> i64 {
        match kind {
            AccountType::Customers => self.opening + self.sales - self.received,
            AccountType::Suppliers => self.opening + self.purchases,
        }
    }
}

#[derive(Clone, Default)]
struct Document {
    id: i64,
    number: String,
    date: String,
    total: i64,
    received: i64,
    mode: String,
}

impl Document {
    fn due(&self) -> i64 {
        (self.total - self.received).max(0)
    }

    fn is_paid(&self) -> bool {
        self.due() == 0
    }
}

#[derive(Clone, Default)]
struct State {
    year: i32,
    kind: AccountType,
    filter: InvoiceFilter,
    search: String,
    document_search: String,
    accounts: Vec<Account>,
    documents: Vec<Document>,
    selected: Option<i64>,
    loaded: bool,
    loaded_search: String,
    error: String,
}

pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let state_id = Id::new("aster_accounts_workspace_v3");

    let mut state = ui
        .ctx()
        .data_mut(|data| data.get_temp::<State>(state_id).unwrap_or_default());

    let needs_reload =
        !state.loaded || state.year != app.selected_year || state.loaded_search != state.search;

    if needs_reload {
        load_accounts(app, &mut state);
    }

    // One page-level scroll area. No nested scroll regions, so the
    // scrollbar remains at the far right of the complete workspace.
    ScrollArea::vertical()
        .id_salt("aster_accounts_full_page")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.add_space(6.0);
            draw_header(ui, app, &mut state);
            ui.add_space(18.0);

            draw_type_selector(ui, &mut state);
            ui.add_space(14.0);

            if !state.error.is_empty() {
                draw_error(ui, app, &mut state);
            } else {
                draw_metrics(ui, &state);
                ui.add_space(14.0);
                draw_workspace(ui, app, &mut state);
            }

            ui.add_space(22.0);
        });

    ui.ctx().data_mut(|data| data.insert_temp(state_id, state));
}

fn draw_header(ui: &mut egui::Ui, app: &AsterApp, state: &mut State) {
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("FINANCIAL MANAGEMENT")
                    .size(10.0)
                    .strong()
                    .color(PURPLE),
            );

            ui.add_space(5.0);

            ui.label(RichText::new("Accounts").size(28.0).strong().color(TEXT));

            ui.add_space(4.0);

            ui.label(
                RichText::new("Customer receivables and supplier account activity")
                    .size(12.0)
                    .color(MUTED),
            );
        });

        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("↻  Refresh data").size(11.5).strong())
                        .fill(CARD)
                        .stroke(Stroke::new(1.0_f32, BORDER)),
                )
                .clicked()
            {
                load_accounts(app, state);
            }

            ui.add_space(8.0);

            ui.label(
                RichText::new(format!(
                    "FY {}–{}",
                    app.selected_year,
                    app.selected_year + 1
                ))
                .size(11.5)
                .color(MUTED),
            );
        });
    });
}

fn draw_type_selector(ui: &mut egui::Ui, state: &mut State) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("ACCOUNT TYPE")
                    .size(9.5)
                    .strong()
                    .color(MUTED),
            );

            ui.add_space(12.0);

            type_button(
                ui,
                "Customers",
                state.kind == AccountType::Customers,
                || {
                    state.kind = AccountType::Customers;
                    state.selected = None;
                    state.filter = InvoiceFilter::All;
                    state.document_search.clear();
                    state.loaded = false;
                },
            );

            type_button(
                ui,
                "Suppliers",
                state.kind == AccountType::Suppliers,
                || {
                    state.kind = AccountType::Suppliers;
                    state.selected = None;
                    state.filter = InvoiceFilter::All;
                    state.document_search.clear();
                    state.loaded = false;
                },
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new("Separate account and document views")
                        .size(10.0)
                        .color(MUTED),
                );
            });
        });
    });
}

fn draw_metrics(ui: &mut egui::Ui, state: &State) {
    let selected = state
        .accounts
        .iter()
        .find(|account| Some(account.id) == state.selected);

    let account_count = state.accounts.len();

    let activity: i64 = state
        .accounts
        .iter()
        .map(|account| match state.kind {
            AccountType::Customers => account.sales,
            AccountType::Suppliers => account.purchases,
        })
        .sum();

    let total_balance: i64 = state
        .accounts
        .iter()
        .map(|account| account.balance(state.kind))
        .sum();

    let selected_balance = selected
        .map(|account| account.balance(state.kind))
        .unwrap_or(0);

    let selected_name = selected
        .map(|account| account.name.as_str())
        .unwrap_or("No account selected");

    ui.columns(4, |columns| {
        metric(
            &mut columns[0],
            "TOTAL ACCOUNTS",
            &account_count.to_string(),
            "Matching records",
            PURPLE,
        );

        metric(
            &mut columns[1],
            if state.kind == AccountType::Customers {
                "POSTED SALES"
            } else {
                "POSTED PURCHASES"
            },
            &money(activity),
            "Current financial year",
            BLUE,
        );

        metric(
            &mut columns[2],
            if state.kind == AccountType::Customers {
                "ESTIMATED RECEIVABLES"
            } else {
                "BEFORE SUPPLIER PAYMENTS"
            },
            &money(total_balance),
            "Combined account balances",
            ORANGE,
        );

        metric(
            &mut columns[3],
            "SELECTED BALANCE",
            &money(selected_balance),
            selected_name,
            if selected_balance > 0 { ORANGE } else { GREEN },
        );
    });
}

fn draw_workspace(ui: &mut egui::Ui, app: &AsterApp, state: &mut State) {
    // Bento-style layout: account navigation at left, details at right.
    // Both panels grow naturally inside the one outer page scroll.
    let available = ui.available_width();
    let left_width = (available * 0.34).clamp(280.0, 410.0);
    let right_width = (available - left_width - 12.0).max(320.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(left_width, 0.0),
            Layout::top_down(Align::Min),
            |ui| draw_account_list(ui, app, state),
        );

        ui.add_space(12.0);

        ui.allocate_ui_with_layout(
            Vec2::new(right_width, 0.0),
            Layout::top_down(Align::Min),
            |ui| draw_account_detail(ui, app, state),
        );
    });
}

fn draw_account_list(ui: &mut egui::Ui, app: &AsterApp, state: &mut State) {
    card(ui, |ui| {
        section_title(
            ui,
            if state.kind == AccountType::Customers {
                "Customer accounts"
            } else {
                "Supplier accounts"
            },
            "Select an account to view its documents.",
        );

        ui.add_space(12.0);

        let search = ui.add(
            egui::TextEdit::singleline(&mut state.search)
                .hint_text("Search name, phone, GSTIN…")
                .desired_width(f32::INFINITY),
        );

        if search.changed() {
            load_accounts(app, state);
        }

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} accounts", state.accounts.len()))
                    .size(10.5)
                    .color(MUTED),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.small_button("Clear").clicked() {
                    state.search.clear();
                    load_accounts(app, state);
                }
            });
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(5.0);

        if state.accounts.is_empty() {
            empty(
                ui,
                "No accounts found",
                "Try another search or add an account in the master screen.",
            );
        } else {
            for account in state.accounts.clone() {
                let selected = state.selected == Some(account.id);

                let response = account_item(ui, &account, selected, state.kind);

                if response.clicked() && !selected {
                    state.selected = Some(account.id);
                    state.filter = InvoiceFilter::All;
                    state.document_search.clear();
                    load_documents(app, state);
                }

                ui.add_space(6.0);
            }
        }
    });
}

fn draw_account_detail(ui: &mut egui::Ui, app: &AsterApp, state: &mut State) {
    let Some(account) = state
        .accounts
        .iter()
        .find(|account| Some(account.id) == state.selected)
        .cloned()
    else {
        card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add_space(36.0);
            empty(
                ui,
                "Select an account",
                "Account details, balance analytics and document history will appear here.",
            );
            ui.add_space(36.0);
        });
        return;
    };

    let available = ui.available_width();
    let gap = 12.0;
    let profile_width = ((available - gap) * 0.58).max(0.0);
    let chart_width = (available - profile_width - gap).max(0.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(profile_width, 0.0),
            Layout::top_down(Align::Min),
            |ui| {
                draw_profile(ui, &account, state.kind);
            },
        );

        ui.add_space(gap);

        ui.allocate_ui_with_layout(
            Vec2::new(chart_width, 0.0),
            Layout::top_down(Align::Min),
            |ui| {
                draw_balance_chart(ui, &account, state);
            },
        );
    });

    ui.add_space(12.0);
    draw_documents(ui, app, state);
}

fn draw_profile(ui: &mut egui::Ui, account: &Account, kind: AccountType) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(46.0, 46.0), Sense::hover());

            ui.painter()
                .circle_filled(rect.center(), 23.0, LIGHT_PURPLE);

            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                initials(&account.name),
                egui::FontId::proportional(15.0),
                PURPLE,
            );

            ui.add_space(7.0);

            ui.vertical(|ui| {
                ui.label(RichText::new(&account.name).size(18.0).strong().color(TEXT));

                ui.add_space(4.0);

                ui.label(
                    RichText::new(format!(
                        "{}  ·  {}",
                        if account.active { "Active" } else { "Inactive" },
                        if account.phone.is_empty() {
                            "No phone recorded"
                        } else {
                            &account.phone
                        }
                    ))
                    .size(11.0)
                    .color(MUTED),
                );
            });
        });

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(9.0);

        ui.columns(2, |columns| {
            detail(&mut columns[0], "GSTIN", fallback(&account.gstin));
            detail(&mut columns[1], "City", fallback(&account.city));
            detail(&mut columns[0], "Email", fallback(&account.email));
            detail(&mut columns[1], "Opening balance", &money(account.opening));
            if kind == AccountType::Customers {
                detail(
                    &mut columns[0],
                    "Credit limit",
                    &money(account.credit_limit),
                );
                detail(&mut columns[1], "Posted sales", &money(account.sales));
            } else {
                detail(
                    &mut columns[0],
                    "Posted purchases",
                    &money(account.purchases),
                );
                detail(&mut columns[1], "Payment tracking", "Not implemented");
            }
        });
    });
}

fn draw_balance_chart(ui: &mut egui::Ui, account: &Account, state: &State) {
    card(ui, |ui| {
        section_title(
            ui,
            "Balance distribution",
            "Financial breakdown for the selected account",
        );

        ui.add_space(14.0);

        let (first_label, first_value, second_label, second_value) =
            match state.kind {
                AccountType::Customers => (
                    "Received",
                    account.received.max(0),
                    "Outstanding",
                    (account.sales - account.received).max(0),
                ),
                AccountType::Suppliers => (
                    "Opening balance",
                    account.opening.max(0),
                    "Purchases",
                    account.purchases.max(0),
                ),
            };

        let total = first_value.saturating_add(second_value);
        let ratio = if total > 0 {
            first_value as f32 / total as f32
        } else {
            0.0
        };

        ui.horizontal_top(|ui| {
            let chart_size = 170.0;
            let (rect, _) = ui.allocate_exact_size(
                Vec2::splat(chart_size),
                Sense::hover(),
            );

            let painter = ui.painter_at(rect);
            let center = rect.center();
            let radius = 61.0;
            let thickness = 16.0_f32;

            painter.circle_stroke(
                center,
                radius,
                Stroke::new(thickness, BG),
            );

            let animated_ratio = ui.ctx().animate_value_with_time(
                Id::new(("account_donut", account.id, state.kind as u8)),
                ratio,
                0.4,
            );

            let start_angle = -std::f32::consts::FRAC_PI_2;
            let first_angle =
                std::f32::consts::TAU * animated_ratio.clamp(0.0, 1.0_f32);
            let second_angle = std::f32::consts::TAU - first_angle;

            for (start, sweep, dark, light) in [
                (
                    start_angle,
                    first_angle,
                    Color32::from_rgb(99, 68, 151),
                    Color32::from_rgb(165, 108, 242),
                ),
                (
                    start_angle + first_angle,
                    second_angle,
                    Color32::from_rgb(28, 112, 72),
                    Color32::from_rgb(72, 201, 135),
                ),
            ] {
                if sweep <= 0.001 {
                    continue;
                }

                let segments = ((sweep * 45.0).ceil() as usize).max(2);

                let points: Vec<egui::Pos2> = (0..=segments)
                    .map(|i| {
                        let angle = start + sweep * i as f32 / segments as f32;
                        center + egui::vec2(angle.cos(), angle.sin()) * radius
                    })
                    .collect();

                for (i, pair) in points.windows(2).enumerate() {
                    let t = i as f32 / segments as f32;

                    let color = gradient_color(dark, light, t);

                    painter.line_segment(
                        [pair[0], pair[1]],
                        Stroke::new(thickness, color),
                    );
                }
            }

            painter.text(
                center + egui::vec2(0.0, -8.0),
                egui::Align2::CENTER_CENTER,
                "TOTAL",
                egui::FontId::proportional(9.0),
                MUTED,
            );

            painter.text(
                center + egui::vec2(0.0, 11.0),
                egui::Align2::CENTER_CENTER,
                money(total),
                egui::FontId::proportional(12.0),
                TEXT,
            );

            ui.add_space(12.0);

            ui.vertical(|ui| {
                ui.add_space(15.0);

                legend(ui, Color32::from_rgb(165, 108, 242), first_label);
                ui.label(
                    RichText::new(money(first_value))
                        .size(14.0)
                        .strong()
                        .color(TEXT),
                );

                ui.add_space(13.0);

                legend(ui, Color32::from_rgb(72, 201, 135), second_label);
                ui.label(
                    RichText::new(money(second_value))
                        .size(14.0)
                        .strong()
                        .color(TEXT),
                );

                ui.add_space(12.0);

                let percentage = ratio as f64 * 100.0;
                let percentage_label = match state.kind {
                    AccountType::Customers => {
                        format!("{percentage:.1}% received")
                    }
                    AccountType::Suppliers => {
                        format!("{percentage:.1}% opening balance")
                    }
                };

                ui.label(
                    RichText::new(percentage_label)
                        .size(10.0)
                        .color(MUTED),
                );
            });
        });

        ui.add_space(10.0);

        let bar_width = ui.available_width();
        let (bar_rect, _) = ui.allocate_exact_size(
            Vec2::new(bar_width, 12.0),
            Sense::hover(),
        );

        ui.painter().rect_filled(
            bar_rect,
            CornerRadius::same(6),
            BG,
        );

        if total > 0 {
            let animated = ui.ctx().animate_value_with_time(
                Id::new(("account_balance_bar", account.id, state.kind as u8)),
                ratio,
                0.45,
            );

            let filled_width =
                bar_rect.width() * animated.clamp(0.0, 1.0_f32);

            let segments = 40;
            for i in 0..segments {
                let x0 = bar_rect.left()
                    + filled_width * i as f32 / segments as f32;
                let x1 = bar_rect.left()
                    + filled_width * (i + 1) as f32 / segments as f32;

                if x1 <= x0 {
                    continue;
                }

                let t = i as f32 / segments as f32;
                let color = match state.kind {
                    AccountType::Customers => {
                        gradient_color(
                            Color32::from_rgb(99, 68, 151),
                            Color32::from_rgb(165, 108, 242),
                            t,
                        )
                    }
                    AccountType::Suppliers => {
                        gradient_color(
                            Color32::from_rgb(66, 113, 190),
                            Color32::from_rgb(155, 100, 190),
                            t,
                        )
                    }
                };

                let segment = egui::Rect::from_min_max(
                    egui::pos2(x0, bar_rect.top()),
                    egui::pos2(x1 + 0.5, bar_rect.bottom()),
                );

                ui.painter().rect_filled(
                    segment,
                    CornerRadius::same(3),
                    color,
                );
            }
        }

        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(first_label)
                    .size(9.5)
                    .color(MUTED),
            );

            ui.with_layout(
                Layout::right_to_left(Align::Center),
                |ui| {
                    ui.label(
                        RichText::new(second_label)
                            .size(9.5)
                            .color(MUTED),
                    );
                },
            );
        });
    });
}

fn gradient_color(start: Color32, end: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0_f32);

    let channel = |a: u8, b: u8| -> u8 {
        (a as f32 + (b as f32 - a as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };

    Color32::from_rgb(
        channel(start.r(), end.r()),
        channel(start.g(), end.g()),
        channel(start.b(), end.b()),
    )
}

fn draw_documents(ui: &mut egui::Ui, app: &AsterApp, state: &mut State) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                section_title(
                    ui,
                    if state.kind == AccountType::Customers {
                        "Sales invoices"
                    } else {
                        "Purchase documents"
                    },
                    "Document activity for the selected account",
                );
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .small_button("↻")
                    .on_hover_text("Refresh documents")
                    .clicked()
                {
                    load_documents(app, state);
                }
            });
        });

        ui.add_space(12.0);

        ui.horizontal(|ui| {
            filter_button(ui, "All", state.filter == InvoiceFilter::All, || {
                state.filter = InvoiceFilter::All
            });

            if state.kind == AccountType::Customers {
                filter_button(
                    ui,
                    "Outstanding",
                    state.filter == InvoiceFilter::Outstanding,
                    || state.filter = InvoiceFilter::Outstanding,
                );

                filter_button(ui, "Paid", state.filter == InvoiceFilter::Paid, || {
                    state.filter = InvoiceFilter::Paid
                });
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} documents", filtered_documents(state).len()))
                        .size(10.0)
                        .color(MUTED),
                );
            });
        });

        ui.add_space(9.0);

        ui.add(
            egui::TextEdit::singleline(&mut state.document_search)
                .hint_text("Filter by invoice number or date…")
                .desired_width(f32::INFINITY),
        );

        ui.add_space(12.0);

        let documents = filtered_documents(state);

        if documents.is_empty() {
            empty(
                ui,
                "No matching documents",
                "Try another filter or confirm the document was posted in this financial year.",
            );
        } else {
            document_table_header(ui, state.kind);

            for document in documents {
                document_row(ui, &document, state.kind);
            }
        }

        ui.add_space(7.0);

        ui.label(
            RichText::new("Only posted documents are included. Reversed documents are excluded.")
                .size(9.5)
                .color(MUTED),
        );
    });
}

fn filtered_documents(state: &State) -> Vec<Document> {
    let query = state.document_search.trim().to_lowercase();

    state
        .documents
        .iter()
        .filter(|document| {
            let matches_query = query.is_empty()
                || document.number.to_lowercase().contains(&query)
                || document.date.to_lowercase().contains(&query);

            let matches_filter = match state.filter {
                InvoiceFilter::All => true,
                InvoiceFilter::Outstanding => !document.is_paid(),
                InvoiceFilter::Paid => document.is_paid(),
            };

            matches_query && matches_filter
        })
        .cloned()
        .collect()
}

fn load_accounts(app: &AsterApp, state: &mut State) {
    state.year = app.selected_year;
    state.loaded_search = state.search.clone();
    state.loaded = true;
    state.error.clear();

    let Some(db) = app.db.as_ref() else {
        state.accounts.clear();
        state.documents.clear();
        state.error = "Database connection is unavailable.".into();
        return;
    };

    let conn = match db.year_connection(state.year) {
        Ok(connection) => connection,
        Err(error) => {
            state.error = error;
            return;
        }
    };

    let pattern = format!("%{}%", state.search.trim());

    let result: Result<Vec<Account>, String> = match state.kind {
        AccountType::Customers => conn
            .prepare(
                "SELECT
                    c.id,
                    c.name,
                    c.phone,
                    c.email,
                    c.gstin,
                    c.city,
                    c.active,
                    c.opening_balance_paise,
                    c.credit_limit_paise,
                    COALESCE(SUM(
                        CASE WHEN s.status = 'posted'
                        THEN s.total_paise ELSE 0 END
                    ), 0),
                    COALESCE(SUM(
                        CASE WHEN s.status = 'posted'
                        THEN s.received_paise ELSE 0 END
                    ), 0),
                    0
                FROM customers c
                LEFT JOIN sales_invoices s ON s.customer_id = c.id
                WHERE
                    c.name LIKE ?1 COLLATE NOCASE
                    OR c.phone LIKE ?1 COLLATE NOCASE
                    OR c.email LIKE ?1 COLLATE NOCASE
                    OR c.gstin LIKE ?1 COLLATE NOCASE
                    OR c.city LIKE ?1 COLLATE NOCASE
                GROUP BY c.id
                ORDER BY c.name COLLATE NOCASE
                LIMIT 1000",
            )
            .map_err(|error| error.to_string())
            .and_then(|mut statement| {
                statement
                    .query_map(params![pattern], |row| {
                        Ok(Account {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            phone: row.get(2)?,
                            email: row.get(3)?,
                            gstin: row.get(4)?,
                            city: row.get(5)?,
                            active: row.get::<_, i64>(6)? != 0,
                            opening: row.get(7)?,
                            credit_limit: row.get(8)?,
                            sales: row.get(9)?,
                            received: row.get(10)?,
                            purchases: row.get(11)?,
                        })
                    })
                    .map_err(|error| error.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| error.to_string())
            }),

        AccountType::Suppliers => conn
            .prepare(
                "SELECT
                    s.id,
                    s.name,
                    s.phone,
                    s.email,
                    s.gstin,
                    s.city,
                    s.active,
                    s.opening_balance_paise,
                    0,
                    0,
                    0,
                    COALESCE(SUM(
                        CASE WHEN p.status = 'posted'
                        THEN p.total_paise ELSE 0 END
                    ), 0)
                FROM suppliers s
                LEFT JOIN purchase_documents p ON p.supplier_id = s.id
                WHERE
                    s.name LIKE ?1 COLLATE NOCASE
                    OR s.phone LIKE ?1 COLLATE NOCASE
                    OR s.email LIKE ?1 COLLATE NOCASE
                    OR s.gstin LIKE ?1 COLLATE NOCASE
                    OR s.city LIKE ?1 COLLATE NOCASE
                GROUP BY s.id
                ORDER BY s.name COLLATE NOCASE
                LIMIT 1000",
            )
            .map_err(|error| error.to_string())
            .and_then(|mut statement| {
                statement
                    .query_map(params![pattern], |row| {
                        Ok(Account {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            phone: row.get(2)?,
                            email: row.get(3)?,
                            gstin: row.get(4)?,
                            city: row.get(5)?,
                            active: row.get::<_, i64>(6)? != 0,
                            opening: row.get(7)?,
                            credit_limit: row.get(8)?,
                            sales: row.get(9)?,
                            received: row.get(10)?,
                            purchases: row.get(11)?,
                        })
                    })
                    .map_err(|error| error.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| error.to_string())
            }),
    };

    match result {
        Ok(accounts) => {
            state.accounts = accounts;

            if !state
                .accounts
                .iter()
                .any(|account| Some(account.id) == state.selected)
            {
                state.selected = state.accounts.first().map(|account| account.id);
            }

            load_documents(app, state);
        }
        Err(error) => {
            state.accounts.clear();
            state.documents.clear();
            state.error = format!("Account query failed: {error}");
        }
    }
}

fn load_documents(app: &AsterApp, state: &mut State) {
    state.documents.clear();

    let (Some(db), Some(account_id)) = (app.db.as_ref(), state.selected) else {
        return;
    };

    let conn = match db.year_connection(state.year) {
        Ok(connection) => connection,
        Err(error) => {
            state.error = error;
            return;
        }
    };

    let result: Result<Vec<Document>, rusqlite::Error> = match state.kind {
        AccountType::Customers => conn
            .prepare(
                "SELECT
                    id,
                    invoice_no,
                    invoice_date,
                    total_paise,
                    received_paise,
                    payment_mode
                FROM sales_invoices
                WHERE customer_id = ?1 AND status = 'posted'
                ORDER BY invoice_date DESC, id DESC
                LIMIT 1000",
            )
            .and_then(|mut statement| {
                statement
                    .query_map(params![account_id], |row| {
                        Ok(Document {
                            id: row.get(0)?,
                            number: row.get(1)?,
                            date: row.get(2)?,
                            total: row.get(3)?,
                            received: row.get(4)?,
                            mode: row.get(5)?,
                        })
                    })?
                    .collect()
            }),

        AccountType::Suppliers => conn
            .prepare(
                "SELECT
                    id,
                    purchase_no,
                    invoice_date,
                    total_paise,
                    0,
                    'Purchase'
                FROM purchase_documents
                WHERE supplier_id = ?1 AND status = 'posted'
                ORDER BY invoice_date DESC, id DESC
                LIMIT 1000",
            )
            .and_then(|mut statement| {
                statement
                    .query_map(params![account_id], |row| {
                        Ok(Document {
                            id: row.get(0)?,
                            number: row.get(1)?,
                            date: row.get(2)?,
                            total: row.get(3)?,
                            received: row.get(4)?,
                            mode: row.get(5)?,
                        })
                    })?
                    .collect()
            }),
    };

    match result {
        Ok(documents) => state.documents = documents,
        Err(error) => {
            state.error = format!("Document query failed: {error}");
        }
    }
}

fn card(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width();

    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(15))
        .show(ui, |inner| {
            inner.set_min_width((width - 30.0).max(0.0));
            contents(inner);
        });
}
fn section_title(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(14.0).strong().color(TEXT));

    ui.add_space(4.0);

    ui.label(RichText::new(subtitle).size(10.5).color(MUTED));
}

fn metric(ui: &mut egui::Ui, label: &str, value: &str, subtitle: &str, accent: Color32) {
    card(ui, |ui| {
        ui.label(RichText::new(label).size(9.0).strong().color(MUTED));

        ui.add_space(9.0);

        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(4.0, 25.0), Sense::hover());

            ui.painter()
                .rect_filled(rect, CornerRadius::same(2), accent);

            ui.add_space(5.0);

            ui.label(RichText::new(value).size(19.0).strong().color(TEXT));
        });

        ui.add_space(6.0);

        ui.label(RichText::new(subtitle).size(9.5).color(MUTED));
    });
}

fn type_button(ui: &mut egui::Ui, label: &str, selected: bool, action: impl FnOnce()) {
    let response = ui.add(
        egui::Button::new(RichText::new(label).size(11.5).strong().color(if selected {
            PURPLE
        } else {
            MUTED
        }))
        .fill(if selected { LIGHT_PURPLE } else { CARD })
        .stroke(Stroke::new(1.0_f32, if selected { PURPLE } else { BORDER }))
        .corner_radius(CornerRadius::same(7)),
    );

    if response.clicked() {
        action();
    }
}

fn filter_button(ui: &mut egui::Ui, label: &str, selected: bool, action: impl FnOnce()) {
    let response = ui.add(
        egui::Button::new(RichText::new(label).size(10.0).color(if selected {
            PURPLE
        } else {
            MUTED
        }))
        .fill(if selected { LIGHT_PURPLE } else { CARD })
        .stroke(Stroke::new(1.0_f32, if selected { PURPLE } else { BORDER }))
        .corner_radius(CornerRadius::same(6)),
    );

    if response.clicked() {
        action();
    }
}

fn account_item(
    ui: &mut egui::Ui,
    account: &Account,
    selected: bool,
    kind: AccountType,
) -> egui::Response {
    let response = egui::Frame::new()
        .fill(if selected { LIGHT_PURPLE } else { CARD })
        .stroke(Stroke::new(1.0_f32, if selected { PURPLE } else { BORDER }))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());

                ui.painter()
                    .circle_filled(rect.center(), 17.0, if selected { PURPLE } else { BG });

                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    initials(&account.name),
                    egui::FontId::proportional(11.0),
                    if selected { Color32::WHITE } else { PURPLE },
                );

                ui.add_space(6.0);

                ui.vertical(|ui| {
                    ui.label(RichText::new(&account.name).size(11.5).strong().color(TEXT));

                    ui.add_space(3.0);

                    ui.label(
                        RichText::new(fallback(&account.phone))
                            .size(10.0)
                            .color(MUTED),
                    );

                    ui.add_space(4.0);

                    ui.label(
                        RichText::new(money(account.balance(kind)))
                            .size(12.0)
                            .strong()
                            .color(if account.balance(kind) > 0 {
                                ORANGE
                            } else {
                                GREEN
                            }),
                    );
                });
            });
        })
        .response;

    response.interact(Sense::click())
}

fn chart_bar(
    ui: &mut egui::Ui,
    label: &str,
    amount: i64,
    maximum: i64,
    color: Color32,
    account_name: &str,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(10.5).color(MUTED));

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(money(amount)).size(11.0).strong().color(TEXT));
        });
    });

    ui.add_space(6.0);

    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 10.0), Sense::hover());

    ui.painter().rect_filled(rect, CornerRadius::same(5), BG);

    let target = amount.max(0) as f32 / maximum.max(1) as f32;

    let animated = ui.ctx().animate_value_with_time(
        Id::new(("aster_account_chart", account_name, label)),
        target,
        0.25,
    );

    let bar = egui::Rect::from_min_size(
        rect.min,
        Vec2::new(rect.width() * animated.clamp(0.0, 1.0_f32), rect.height()),
    );

    ui.painter().rect_filled(bar, CornerRadius::same(5), color);
}

fn legend(ui: &mut egui::Ui, color: Color32, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(7.0), Sense::hover());

        ui.painter().circle_filled(rect.center(), 3.5, color);

        ui.label(RichText::new(label).size(9.0).color(MUTED));
    });
}

fn document_table_header(ui: &mut egui::Ui, kind: AccountType) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("DOCUMENT / DATE")
                .size(9.0)
                .strong()
                .color(MUTED),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(if kind == AccountType::Customers {
                    "TOTAL / RECEIVED / DUE"
                } else {
                    "PURCHASE TOTAL"
                })
                .size(9.0)
                .strong()
                .color(MUTED),
            );
        });
    });

    ui.add_space(7.0);
    ui.separator();
    ui.add_space(5.0);
}

fn document_row(ui: &mut egui::Ui, document: &Document, kind: AccountType) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(&document.number)
                    .size(11.0)
                    .strong()
                    .color(TEXT),
            );

            ui.add_space(3.0);

            ui.label(
                RichText::new(format!("{} · {}", document.date, document.mode))
                    .size(9.5)
                    .color(MUTED),
            );
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(money(document.total))
                        .size(11.0)
                        .strong()
                        .color(TEXT),
                );

                if kind == AccountType::Customers {
                    ui.add_space(3.0);

                    let due = document.due();

                    ui.label(
                        RichText::new(format!(
                            "Received {} · Due {}",
                            money(document.received),
                            money(due)
                        ))
                        .size(9.5)
                        .color(if due > 0 { ORANGE } else { GREEN }),
                    );
                }
            });
        });
    });

    ui.add_space(7.0);
    ui.separator();
    ui.add_space(5.0);
}

fn detail(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(10.0).color(MUTED));

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add(
                egui::Label::new(RichText::new(value).size(10.0).strong().color(TEXT)).truncate(),
            );
        });
    });

    ui.add_space(8.0);
}

fn draw_error(ui: &mut egui::Ui, app: &AsterApp, state: &mut State) {
    card(ui, |ui| {
        ui.label(
            RichText::new("Accounts could not be loaded")
                .size(15.0)
                .strong()
                .color(RED),
        );

        ui.add_space(6.0);

        ui.label(RichText::new(&state.error).size(11.0).color(MUTED));

        ui.add_space(12.0);

        if ui.button("Retry").clicked() {
            load_accounts(app, state);
        }
    });
}

fn empty(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(15.0);
        ui.label(RichText::new(title).size(12.0).strong().color(TEXT));
        ui.add_space(5.0);
        ui.label(RichText::new(subtitle).size(10.0).color(MUTED));
        ui.add_space(15.0);
    });
}

fn money(paise: i64) -> String {
    format_inr(paise as f64 / 100.0)
}

fn fallback(value: &str) -> &str {
    if value.trim().is_empty() {
        "—"
    } else {
        value
    }
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}
