use crate::app::AsterApp;
use crate::utils::currency::format_inr;
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Layout, Margin, Pos2, Rect,
    Response, RichText, ScrollArea, Sense, Shape, Stroke, TextStyle, Ui, Vec2,
};
use rusqlite::params;
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
const BLUE: Color32 = Color32::from_rgb(54, 104, 178);

const MONTHS: [&str; 12] = [
    "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "Jan", "Feb", "Mar",
];

// ───────────────────────── Types ─────────────────────────
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

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum AccountFilter {
    #[default]
    All,
    Due,
    Settled,
    Inactive,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum SortBy {
    #[default]
    Name,
    Balance,
    Activity,
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

    fn activity(&self, kind: AccountType) -> i64 {
        match kind {
            AccountType::Customers => self.sales,
            AccountType::Suppliers => self.purchases,
        }
    }

    fn over_limit(&self, kind: AccountType) -> bool {
        kind == AccountType::Customers
            && self.credit_limit > 0
            && self.balance(kind) > self.credit_limit
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
    account_filter: AccountFilter,
    sort: SortBy,
    search: String,
    document_search: String,
    accounts: Vec<Account>,
    documents: Vec<Document>,
    selected: Option<i64>,
    loaded: bool,
    loaded_search: String,
    error: String,
    notice: String,
}

// ───────────────────────── Entry point ─────────────────────────
pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let state_id = Id::new("aster_accounts_workspace_v4");

    let mut state = ui
        .ctx()
        .data_mut(|data| data.get_temp::<State>(state_id).unwrap_or_default());

    let needs_reload =
        !state.loaded || state.year != app.selected_year || state.loaded_search != state.search;

    if needs_reload {
        load_accounts(app, &mut state);
    }

    ScrollArea::vertical()
        .id_salt("aster_accounts_full_page")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            apply_style(ui);

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
                draw_overview(ui, app, &mut state);
                ui.add_space(14.0);
                draw_workspace(ui, app, &mut state);
            }

            ui.add_space(22.0);
        });

    ui.ctx().data_mut(|data| data.insert_temp(state_id, state));
}

// ───────────────────────── Page sections ─────────────────────────
fn draw_header(ui: &mut Ui, app: &AsterApp, state: &mut State) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("FINANCIAL MANAGEMENT")
                    .size(11.0)
                    .strong()
                    .color(PURPLE),
            );
            ui.add_space(2.0);
            ui.label(RichText::new("Accounts").size(28.0).strong().color(TEXT));
            ui.add_space(2.0);
            ui.label(
                RichText::new("Customer receivables and supplier account activity")
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
                load_accounts(app, state);
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

fn draw_type_selector(ui: &mut Ui, state: &mut State) {
    egui::Frame::new()
        .fill(HEADER_BG)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for (label, kind) in [
                    ("Customers", AccountType::Customers),
                    ("Suppliers", AccountType::Suppliers),
                ] {
                    let selected = state.kind == kind;
                    let button = egui::Button::new(
                        RichText::new(label).size(14.5).strong().color(if selected {
                            PURPLE
                        } else {
                            MUTED
                        }),
                    )
                    .fill(if selected { CARD } else { Color32::TRANSPARENT })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if selected {
                            BORDER
                        } else {
                            Color32::TRANSPARENT
                        },
                    ))
                    .corner_radius(CornerRadius::same(9))
                    .min_size(Vec2::new(140.0, 36.0));

                    if ui.add(button).clicked() && !selected {
                        state.kind = kind;
                        state.selected = None;
                        state.filter = InvoiceFilter::All;
                        state.account_filter = AccountFilter::All;
                        state.document_search.clear();
                        state.search.clear();
                        state.notice.clear();
                        state.loaded = false;
                    }
                }
            });
        });
}

fn draw_metrics(ui: &mut Ui, state: &State) {
    let kind = state.kind;
    let count = state.accounts.len();
    let active = state.accounts.iter().filter(|a| a.active).count();
    let activity: i64 = state.accounts.iter().map(|a| a.activity(kind)).sum();
    let balance: i64 = state.accounts.iter().map(|a| a.balance(kind)).sum();
    let over = state.accounts.iter().filter(|a| a.over_limit(kind)).count();
    let with_balance = state
        .accounts
        .iter()
        .filter(|a| a.balance(kind) > 0)
        .count();

    ui.columns(4, |cols| {
        metric(
            &mut cols[0],
            "TOTAL ACCOUNTS",
            &count.to_string(),
            &format!("{active} active · {} inactive", count - active),
            PURPLE,
        );
        metric(
            &mut cols[1],
            if kind == AccountType::Customers {
                "POSTED SALES"
            } else {
                "POSTED PURCHASES"
            },
            &money(activity),
            "Current financial year",
            BLUE,
        );
        metric(
            &mut cols[2],
            if kind == AccountType::Customers {
                "RECEIVABLES"
            } else {
                "PAYABLES"
            },
            &money(balance),
            &format!("{with_balance} accounts with a balance"),
            ORANGE,
        );
        if kind == AccountType::Customers {
            metric(
                &mut cols[3],
                "OVER CREDIT LIMIT",
                &over.to_string(),
                if over > 0 {
                    "Needs attention"
                } else {
                    "All within limit"
                },
                if over > 0 { RED } else { GREEN },
            );
        } else {
            metric(
                &mut cols[3],
                "ACTIVE SUPPLIERS",
                &active.to_string(),
                "Currently trading",
                GREEN,
            );
        }
    });
}

fn draw_overview(ui: &mut Ui, app: &AsterApp, state: &mut State) {
    ui.columns(2, |cols| {
        portfolio_card(&mut cols[0], state);
        top_accounts_card(&mut cols[1], app, state);
    });
}

fn portfolio_card(ui: &mut Ui, state: &State) {
    card(ui, |ui| {
        ui.set_min_height(240.0);
        let kind = state.kind;
        section_title(
            ui,
            if kind == AccountType::Customers {
                "Receivables breakdown"
            } else {
                "Payables breakdown"
            },
            "Across all listed accounts",
        );
        ui.add_space(12.0);

        let opening: i64 = state.accounts.iter().map(|a| a.opening.max(0)).sum();
        let sales: i64 = state.accounts.iter().map(|a| a.sales).sum();
        let received: i64 = state.accounts.iter().map(|a| a.received).sum();
        let purchases: i64 = state.accounts.iter().map(|a| a.purchases).sum();

        let (segs, top, bottom) = if kind == AccountType::Customers {
            let outstanding = (sales - received).max(0);
            let pct = if sales > 0 {
                received as f64 / sales as f64 * 100.0
            } else {
                0.0
            };
            (
                vec![
                    Seg {
                        label: "Received",
                        amount: received.max(0),
                        color: GREEN,
                    },
                    Seg {
                        label: "Outstanding sales",
                        amount: outstanding,
                        color: ORANGE,
                    },
                    Seg {
                        label: "Opening balances",
                        amount: opening,
                        color: BLUE,
                    },
                ],
                format!("{pct:.0}%"),
                "of sales collected".to_owned(),
            )
        } else {
            (
                vec![
                    Seg {
                        label: "Opening balances",
                        amount: opening,
                        color: BLUE,
                    },
                    Seg {
                        label: "Purchases",
                        amount: purchases.max(0),
                        color: PURPLE,
                    },
                ],
                compact(opening + purchases),
                "total payable".to_owned(),
            )
        };

        ui.horizontal(|ui| {
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(172.0), Sense::hover());
            donut(ui, rect, &resp, &segs, &top, &bottom);

            ui.add_space(10.0);
            ui.vertical(|ui| {
                for seg in &segs {
                    ui.horizontal(|ui| {
                        dot(ui, seg.color);
                        ui.label(RichText::new(seg.label).size(13.5).color(TEXT));
                    });
                    ui.label(
                        RichText::new(money(seg.amount))
                            .size(15.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.add_space(4.0);
                }
            });
        });
    });
}

fn top_accounts_card(ui: &mut Ui, app: &AsterApp, state: &mut State) {
    card(ui, |ui| {
        ui.set_min_height(240.0);
        let kind = state.kind;
        section_title(
            ui,
            if kind == AccountType::Customers {
                "Highest receivables"
            } else {
                "Highest payables"
            },
            "Top five balances · click to open the account",
        );
        ui.add_space(12.0);

        let mut rows: Vec<(i64, String, i64)> = state
            .accounts
            .iter()
            .filter(|a| a.balance(kind) > 0)
            .map(|a| (a.id, a.name.clone(), a.balance(kind)))
            .collect();
        rows.sort_by(|a, b| b.2.cmp(&a.2));
        rows.truncate(5);

        if rows.is_empty() {
            ui.add_space(34.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("No outstanding balances")
                        .size(14.5)
                        .strong()
                        .color(GREEN),
                );
                ui.label(
                    RichText::new("Every account is settled.")
                        .size(12.5)
                        .color(MUTED),
                );
            });
            return;
        }

        let max = rows.iter().map(|r| r.2).max().unwrap_or(1).max(1);
        for (id, name, amount) in &rows {
            let selected = state.selected == Some(*id);
            ui.horizontal(|ui| {
                let text = RichText::new(truncate(name, 26))
                    .size(14.0)
                    .color(if selected { PURPLE } else { TEXT });
                let text = if selected { text.strong() } else { text };
                let r = ui.add(egui::Button::new(text).frame(false));
                if r.on_hover_text(name.as_str()).clicked() && !selected {
                    state.selected = Some(*id);
                    state.filter = InvoiceFilter::All;
                    state.document_search.clear();
                    state.notice.clear();
                    load_documents(app, state);
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

fn draw_workspace(ui: &mut Ui, app: &AsterApp, state: &mut State) {
    let available = ui.available_width();
    let gap = 14.0;
    let left_width = (available * 0.33).clamp(310.0, 430.0);
    let right_width = (available - left_width - gap).max(360.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(left_width, 0.0),
            Layout::top_down(Align::Min),
            |ui| draw_account_list(ui, app, state),
        );

        ui.add_space(gap);

        ui.allocate_ui_with_layout(
            Vec2::new(right_width, 0.0),
            Layout::top_down(Align::Min),
            |ui| draw_account_detail(ui, app, state),
        );
    });
}

fn draw_account_list(ui: &mut Ui, app: &AsterApp, state: &mut State) {
    card(ui, |ui| {
        let kind = state.kind;
        section_title(
            ui,
            if kind == AccountType::Customers {
                "Customer accounts"
            } else {
                "Supplier accounts"
            },
            "Select an account to see its details and documents",
        );
        ui.add_space(12.0);

        ui.label(caps("SEARCH"));
        let search = ui.add_sized(
            [ui.available_width(), 34.0],
            egui::TextEdit::singleline(&mut state.search)
                .hint_text("Name, phone, email, GSTIN or city"),
        );
        if search.changed() {
            load_accounts(app, state);
        }

        ui.add_space(10.0);
        ui.label(caps("SORT BY"));
        egui::ComboBox::from_id_salt("accounts_sort")
            .width(ui.available_width())
            .selected_text(RichText::new(sort_label(state.sort, kind)).size(14.0))
            .show_ui(ui, |ui| {
                for s in [SortBy::Name, SortBy::Balance, SortBy::Activity] {
                    ui.selectable_value(&mut state.sort, s, sort_label(s, kind));
                }
            });

        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            for (label, filter) in [
                ("All", AccountFilter::All),
                ("With balance", AccountFilter::Due),
                ("Settled", AccountFilter::Settled),
                ("Inactive", AccountFilter::Inactive),
            ] {
                let n = state
                    .accounts
                    .iter()
                    .filter(|a| account_matches(a, filter, kind))
                    .count();
                if chip(ui, &format!("{label}  {n}"), state.account_filter == filter) {
                    state.account_filter = filter;
                }
            }
        });

        // Visible list
        let mut view: Vec<Account> = state
            .accounts
            .iter()
            .filter(|a| account_matches(a, state.account_filter, kind))
            .cloned()
            .collect();
        match state.sort {
            SortBy::Name => view.sort_by_key(|a| a.name.to_lowercase()),
            SortBy::Balance => view.sort_by(|a, b| b.balance(kind).cmp(&a.balance(kind))),
            SortBy::Activity => view.sort_by(|a, b| b.activity(kind).cmp(&a.activity(kind))),
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} of {} accounts",
                    view.len(),
                    state.accounts.len()
                ))
                .size(12.5)
                .color(MUTED),
            );
            if !state.search.is_empty() || state.account_filter != AccountFilter::All {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("Clear filters").clicked() {
                        state.search.clear();
                        state.account_filter = AccountFilter::All;
                        load_accounts(app, state);
                    }
                });
            }
        });
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        if view.is_empty() {
            empty(
                ui,
                "No accounts found",
                "Try another search or filter, or add an account in the master screen.",
            );
            return;
        }

        ScrollArea::vertical()
            .id_salt("accounts_list_scroll")
            .max_height(640.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                for account in &view {
                    let selected = state.selected == Some(account.id);
                    let response = account_item(ui, account, selected, kind);
                    if response.clicked() && !selected {
                        state.selected = Some(account.id);
                        state.filter = InvoiceFilter::All;
                        state.document_search.clear();
                        state.notice.clear();
                        load_documents(app, state);
                    }
                }
            });
    });
}

fn draw_account_detail(ui: &mut Ui, app: &AsterApp, state: &mut State) {
    let Some(account) = state
        .accounts
        .iter()
        .find(|account| Some(account.id) == state.selected)
        .cloned()
    else {
        card(ui, |ui| {
            ui.add_space(50.0);
            empty(
                ui,
                "Select an account",
                "Profile, balance analytics and document history will appear here.",
            );
            ui.add_space(50.0);
        });
        return;
    };

    draw_profile(ui, &account, state.kind);
    ui.add_space(14.0);

    let available = ui.available_width();
    let gap = 14.0;
    let donut_w = ((available - gap) * 0.42).max(260.0);
    let chart_w = (available - donut_w - gap).max(260.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(donut_w, 0.0),
            Layout::top_down(Align::Min),
            |ui| draw_balance_chart(ui, &account, state.kind),
        );
        ui.add_space(gap);
        ui.allocate_ui_with_layout(
            Vec2::new(chart_w, 0.0),
            Layout::top_down(Align::Min),
            |ui| draw_monthly_chart(ui, state),
        );
    });

    ui.add_space(14.0);
    draw_documents(ui, app, state, &account);
}

fn draw_profile(ui: &mut Ui, account: &Account, kind: AccountType) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(56.0), Sense::hover());
            ui.painter().circle_filled(rect.center(), 28.0, PURPLE_BG);
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                initials(&account.name),
                FontId::proportional(19.0),
                PURPLE,
            );

            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(&account.name).size(22.0).strong().color(TEXT));
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    if account.active {
                        badge(ui, "Active", GREEN, GREEN_BG);
                    } else {
                        badge(ui, "Inactive", MUTED, HEADER_BG);
                    }
                    if account.over_limit(kind) {
                        badge(ui, "Over credit limit", RED, RED_BG);
                    }
                    if !account.city.trim().is_empty() {
                        ui.label(RichText::new(&account.city).size(13.0).color(MUTED));
                    }
                });
            });

            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                let balance = account.balance(kind);
                ui.vertical(|ui| {
                    ui.label(caps(if kind == AccountType::Customers {
                        "RECEIVABLE BALANCE"
                    } else {
                        "PAYABLE BALANCE"
                    }));
                    ui.label(
                        RichText::new(money(balance))
                            .size(24.0)
                            .strong()
                            .color(if balance > 0 { ORANGE } else { GREEN }),
                    );
                });
            });
        });

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(10.0);

        ui.columns(3, |cols| {
            detail(&mut cols[0], "PHONE", fallback(&account.phone));
            detail(&mut cols[1], "EMAIL", fallback(&account.email));
            detail(&mut cols[2], "GSTIN", fallback(&account.gstin));
            detail(&mut cols[0], "OPENING BALANCE", &money(account.opening));
            if kind == AccountType::Customers {
                detail(&mut cols[1], "POSTED SALES", &money(account.sales));
                detail(&mut cols[2], "RECEIVED", &money(account.received));
            } else {
                detail(&mut cols[1], "POSTED PURCHASES", &money(account.purchases));
                detail(&mut cols[2], "PAYMENTS", "Tracking not available");
            }
        });

        // Credit limit utilisation (customers only)
        if kind == AccountType::Customers {
            ui.add_space(4.0);
            ui.label(caps("CREDIT LIMIT USAGE"));
            ui.add_space(2.0);

            if account.credit_limit > 0 {
                let used = account.balance(kind).max(0);
                let ratio = used as f32 / account.credit_limit as f32;
                let color = if ratio > 1.0 {
                    RED
                } else if ratio > 0.7 {
                    ORANGE
                } else {
                    GREEN
                };
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 12.0), Sense::hover());
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(6), PURPLE_BG);
                ui.painter().rect_filled(
                    Rect::from_min_size(
                        rect.min,
                        Vec2::new(rect.width() * ratio.clamp(0.0, 1.0), rect.height()),
                    ),
                    CornerRadius::same(6),
                    color,
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!(
                        "{} used of {}  ·  {:.0}%",
                        money(used),
                        money(account.credit_limit),
                        ratio * 100.0
                    ))
                    .size(13.0)
                    .strong()
                    .color(color),
                );
            } else {
                ui.label(
                    RichText::new("No credit limit set for this customer")
                        .size(13.0)
                        .color(MUTED),
                );
            }
        }
    });
}

fn draw_balance_chart(ui: &mut Ui, account: &Account, kind: AccountType) {
    card(ui, |ui| {
        ui.set_min_height(290.0);
        section_title(
            ui,
            "Balance distribution",
            "Breakdown for the selected account",
        );
        ui.add_space(10.0);

        let (segs, top, bottom): (Vec<Seg>, String, String) = match kind {
            AccountType::Customers => {
                let outstanding = (account.sales - account.received).max(0);
                let pct = if account.sales > 0 {
                    account.received as f64 / account.sales as f64 * 100.0
                } else {
                    0.0
                };
                (
                    vec![
                        Seg {
                            label: "Received",
                            amount: account.received.max(0),
                            color: GREEN,
                        },
                        Seg {
                            label: "Outstanding",
                            amount: outstanding,
                            color: ORANGE,
                        },
                        Seg {
                            label: "Opening balance",
                            amount: account.opening.max(0),
                            color: BLUE,
                        },
                    ],
                    format!("{pct:.0}%"),
                    "collected".to_owned(),
                )
            }
            AccountType::Suppliers => (
                vec![
                    Seg {
                        label: "Opening balance",
                        amount: account.opening.max(0),
                        color: BLUE,
                    },
                    Seg {
                        label: "Purchases",
                        amount: account.purchases.max(0),
                        color: PURPLE,
                    },
                ],
                compact(account.opening.max(0) + account.purchases.max(0)),
                "total".to_owned(),
            ),
        };

        ui.vertical_centered(|ui| {
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(160.0), Sense::hover());
            donut(ui, rect, &resp, &segs, &top, &bottom);
        });
        ui.add_space(8.0);

        for seg in &segs {
            ui.horizontal(|ui| {
                dot(ui, seg.color);
                ui.label(RichText::new(seg.label).size(13.0).color(MUTED));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(money(seg.amount))
                            .size(13.5)
                            .strong()
                            .color(TEXT),
                    );
                });
            });
        }
    });
}

fn draw_monthly_chart(ui: &mut Ui, state: &State) {
    card(ui, |ui| {
        ui.set_min_height(290.0);
        let customers = state.kind == AccountType::Customers;

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                section_title(
                    ui,
                    if customers {
                        "Monthly sales"
                    } else {
                        "Monthly purchases"
                    },
                    "By document month across the financial year",
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if customers {
                    legend_item(ui, GREEN, "Received");
                    ui.add_space(8.0);
                }
                legend_item(ui, PURPLE, if customers { "Invoiced" } else { "Purchases" });
            });
        });
        ui.add_space(10.0);

        if state.documents.is_empty() {
            ui.add_space(60.0);
            empty(
                ui,
                "No documents yet",
                "Posted documents will be charted here.",
            );
            return;
        }

        let mut totals = [0_i64; 12];
        let mut received = [0_i64; 12];
        for doc in &state.documents {
            if let Some(m) = doc.date.get(5..7).and_then(|m| m.parse::<usize>().ok()) {
                if (1..=12).contains(&m) {
                    let idx = (m + 8) % 12;
                    totals[idx] += doc.total;
                    received[idx] += doc.received;
                }
            }
        }

        let max = totals
            .iter()
            .chain(received.iter())
            .copied()
            .max()
            .unwrap_or(0);
        let top = nice_max(max);

        let (rect, resp) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 205.0), Sense::hover());
        let painter = ui.painter_at(rect.expand(2.0));
        let plot = Rect::from_min_max(
            Pos2::new(rect.left() + 56.0, rect.top() + 6.0),
            Pos2::new(rect.right() - 2.0, rect.bottom() - 26.0),
        );
        let group_w = plot.width() / 12.0;

        let hovered = resp.hover_pos().and_then(|p| {
            if plot.expand2(Vec2::new(0.0, 10.0)).contains(p) {
                Some((((p.x - plot.left()) / group_w) as usize).min(11))
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

        let bar_w = (group_w * if customers { 0.28 } else { 0.5 }).clamp(6.0, 22.0);
        for i in 0..12 {
            let cx = plot.left() + group_w * (i as f32 + 0.5);

            if hovered == Some(i) {
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(cx - group_w / 2.0 + 1.0, plot.top()),
                        Pos2::new(cx + group_w / 2.0 - 1.0, plot.bottom()),
                    ),
                    CornerRadius::same(6),
                    PURPLE_BG,
                );
            }

            let bars: Vec<(i64, Color32, f32)> = if customers {
                vec![
                    (totals[i], PURPLE, cx - bar_w - 1.5),
                    (received[i], GREEN, cx + 1.5),
                ]
            } else {
                vec![(totals[i], PURPLE, cx - bar_w / 2.0)]
            };
            for (value, color, x0) in bars {
                if value <= 0 {
                    continue;
                }
                let h = (plot.height() * value as f32 / top as f32).max(2.0);
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
                Pos2::new(cx, plot.bottom() + 7.0),
                Align2::CENTER_TOP,
                MONTHS[i],
                FontId::proportional(11.5),
                if hovered == Some(i) { TEXT } else { MUTED },
            );
        }

        if let Some(i) = hovered {
            let year = if i < 9 { state.year } else { state.year + 1 };
            resp.on_hover_ui_at_pointer(|ui| {
                ui.label(
                    RichText::new(format!("{} {}", MONTHS[i], year))
                        .size(14.0)
                        .strong(),
                );
                ui.label(
                    RichText::new(format!(
                        "{}: {}",
                        if customers { "Invoiced" } else { "Purchases" },
                        money(totals[i])
                    ))
                    .size(13.0)
                    .color(PURPLE),
                );
                if customers {
                    ui.label(
                        RichText::new(format!("Received: {}", money(received[i])))
                            .size(13.0)
                            .color(GREEN),
                    );
                    ui.label(
                        RichText::new(format!("Due: {}", money((totals[i] - received[i]).max(0))))
                            .size(13.0)
                            .color(ORANGE),
                    );
                }
            });
        }
    });
}

fn draw_documents(ui: &mut Ui, app: &AsterApp, state: &mut State, account: &Account) {
    card(ui, |ui| {
        let customers = state.kind == AccountType::Customers;

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                section_title(
                    ui,
                    if customers {
                        "Sales invoices"
                    } else {
                        "Purchase documents"
                    },
                    "Posted documents for the selected account",
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(egui::Button::new("↻  Reload").min_size(Vec2::new(0.0, 32.0)))
                    .on_hover_text("Reload documents")
                    .clicked()
                {
                    load_documents(app, state);
                }
                if ui
                    .add(egui::Button::new("⎘  Copy statement").min_size(Vec2::new(0.0, 32.0)))
                    .on_hover_text("Copy a text statement to the clipboard")
                    .clicked()
                {
                    let text = statement_text(account, state.kind, &state.documents);
                    ui.ctx().copy_text(text);
                    state.notice = "Statement copied to clipboard".to_owned();
                }
            });
        });

        if !state.notice.is_empty() {
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!("✓  {}", state.notice))
                    .size(13.0)
                    .strong()
                    .color(GREEN),
            );
        }

        ui.add_space(12.0);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(caps("SEARCH"));
                ui.add_sized(
                    [300.0, 34.0],
                    egui::TextEdit::singleline(&mut state.document_search)
                        .hint_text("Document number or date"),
                );
            });

            if customers {
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(caps("STATUS"));
                    ui.horizontal(|ui| {
                        let all = state.documents.len();
                        let out = state.documents.iter().filter(|d| !d.is_paid()).count();
                        let paid = all - out;
                        for (label, n, filter) in [
                            ("All", all, InvoiceFilter::All),
                            ("Outstanding", out, InvoiceFilter::Outstanding),
                            ("Paid", paid, InvoiceFilter::Paid),
                        ] {
                            if chip(ui, &format!("{label}  {n}"), state.filter == filter) {
                                state.filter = filter;
                            }
                        }
                    });
                });
            }
        });

        ui.add_space(14.0);

        let documents = filtered_documents(state);

        if documents.is_empty() {
            empty(
                ui,
                "No matching documents",
                "Try another filter, or confirm the document was posted in this financial year.",
            );
        } else {
            let inner = (ui.available_width() - 24.0).max(560.0);
            let widths: Vec<f32> = if customers {
                [0.17, 0.14, 0.14, 0.13, 0.14, 0.14, 0.14]
                    .iter()
                    .map(|f| inner * f)
                    .collect()
            } else {
                [0.40, 0.30, 0.30].iter().map(|f| inner * f).collect()
            };

            // Header
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
                        let heads: Vec<(&str, Align)> = if customers {
                            vec![
                                ("DOCUMENT", Align::Min),
                                ("DATE", Align::Min),
                                ("MODE", Align::Min),
                                ("STATUS", Align::Min),
                                ("TOTAL", Align::Max),
                                ("RECEIVED", Align::Max),
                                ("DUE", Align::Max),
                            ]
                        } else {
                            vec![
                                ("DOCUMENT", Align::Min),
                                ("DATE", Align::Min),
                                ("TOTAL", Align::Max),
                            ]
                        };
                        for (i, (label, align)) in heads.into_iter().enumerate() {
                            cell(ui, widths[i], 30.0, align, |ui| {
                                ui.label(caps(label));
                            });
                        }
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
                        .id_salt("account_documents_rows")
                        .max_height(420.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            for (idx, doc) in documents.iter().enumerate() {
                                egui::Frame::new()
                                    .fill(if idx % 2 == 0 { CARD } else { ROW_ALT })
                                    .inner_margin(Margin::symmetric(12, 4))
                                    .show(ui, |ui| {
                                        ui.set_min_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 0.0;
                                            document_row(ui, doc, customers, &widths);
                                        });
                                    });
                            }
                        });
                });

            // Totals
            let total: i64 = documents.iter().map(|d| d.total).sum();
            let received: i64 = documents.iter().map(|d| d.received).sum();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "Showing {} of {} documents",
                        documents.len(),
                        state.documents.len()
                    ))
                    .size(12.5)
                    .color(MUTED),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if customers {
                        let due = (total - received).max(0);
                        ui.label(
                            RichText::new(format!("Due {}", money(due)))
                                .size(13.0)
                                .strong()
                                .color(if due > 0 { ORANGE } else { GREEN }),
                        );
                        ui.label(
                            RichText::new(format!("Received {}  ·", money(received)))
                                .size(13.0)
                                .color(GREEN),
                        );
                    }
                    ui.label(
                        RichText::new(format!("Total {}  ·", money(total)))
                            .size(13.0)
                            .strong()
                            .color(TEXT),
                    );
                });
            });
        }

        ui.add_space(8.0);
        ui.label(
            RichText::new("Only posted documents are included. Reversed documents are excluded.")
                .size(12.0)
                .color(MUTED),
        );
    });
}

fn document_row(ui: &mut Ui, doc: &Document, customers: bool, w: &[f32]) {
    cell(ui, w[0], 40.0, Align::Min, |ui| {
        ui.label(RichText::new(&doc.number).size(14.0).strong().color(TEXT));
    });
    cell(ui, w[1], 40.0, Align::Min, |ui| {
        ui.label(RichText::new(&doc.date).size(13.5).color(MUTED));
    });

    if customers {
        cell(ui, w[2], 40.0, Align::Min, |ui| {
            ui.label(RichText::new(&doc.mode).size(13.5).color(MUTED));
        });
        cell(ui, w[3], 40.0, Align::Min, |ui| {
            if doc.is_paid() {
                badge(ui, "Paid", GREEN, GREEN_BG);
            } else if doc.received > 0 {
                badge(ui, "Partial", ORANGE, ORANGE_BG);
            } else {
                badge(ui, "Unpaid", RED, RED_BG);
            }
        });
        cell(ui, w[4], 40.0, Align::Max, |ui| {
            ui.label(RichText::new(money(doc.total)).size(14.0).color(TEXT));
        });
        cell(ui, w[5], 40.0, Align::Max, |ui| {
            ui.label(RichText::new(money(doc.received)).size(14.0).color(GREEN));
        });
        cell(ui, w[6], 40.0, Align::Max, |ui| {
            let due = doc.due();
            ui.label(
                RichText::new(money(due))
                    .size(14.0)
                    .strong()
                    .color(if due > 0 { ORANGE } else { MUTED }),
            );
        });
    } else {
        cell(ui, w[2], 40.0, Align::Max, |ui| {
            ui.label(
                RichText::new(money(doc.total))
                    .size(14.0)
                    .strong()
                    .color(TEXT),
            );
        });
    }
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

fn account_matches(account: &Account, filter: AccountFilter, kind: AccountType) -> bool {
    match filter {
        AccountFilter::All => true,
        AccountFilter::Due => account.balance(kind) > 0,
        AccountFilter::Settled => account.balance(kind) <= 0,
        AccountFilter::Inactive => !account.active,
    }
}

fn sort_label(sort: SortBy, kind: AccountType) -> &'static str {
    match (sort, kind) {
        (SortBy::Name, _) => "Name  (A–Z)",
        (SortBy::Balance, _) => "Balance  (high to low)",
        (SortBy::Activity, AccountType::Customers) => "Sales  (high to low)",
        (SortBy::Activity, AccountType::Suppliers) => "Purchases  (high to low)",
    }
}

fn statement_text(account: &Account, kind: AccountType, documents: &[Document]) -> String {
    let mut out = String::new();
    out.push_str(&format!("Account statement: {}\n", account.name));
    if !account.gstin.trim().is_empty() {
        out.push_str(&format!("GSTIN: {}\n", account.gstin));
    }
    out.push_str(&format!("Opening balance: {}\n", money(account.opening)));
    out.push_str(&format!(
        "Closing balance: {}\n\n",
        money(account.balance(kind))
    ));
    for doc in documents {
        if kind == AccountType::Customers {
            out.push_str(&format!(
                "{}  {}  total {}  received {}  due {}\n",
                doc.date,
                doc.number,
                money(doc.total),
                money(doc.received),
                money(doc.due())
            ));
        } else {
            out.push_str(&format!(
                "{}  {}  total {}\n",
                doc.date,
                doc.number,
                money(doc.total)
            ));
        }
    }
    out
}

// ───────────────────────── Data loading ─────────────────────────
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

// ───────────────────────── Charts ─────────────────────────
struct Seg {
    label: &'static str,
    amount: i64,
    color: Color32,
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
    let painter = ui.painter_at(rect.expand(6.0));
    let center = rect.center();
    let thickness = 22.0_f32;
    let radius = rect.width() / 2.0 - thickness / 2.0 - 4.0;
    let total: f32 = segs.iter().map(|s| s.amount as f32).sum();

    if total <= 0.0 {
        painter.circle_stroke(center, radius, Stroke::new(thickness, PURPLE_BG));
    } else {
        let nonzero = segs.iter().filter(|s| s.amount > 0).count();
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
                let sweep = s.amount as f32 / total * TAU;
                if s.amount > 0 && ang >= acc && ang < acc + sweep {
                    return Some(i);
                }
                acc += sweep;
            }
            None
        });

        let mut start = -PI / 2.0;
        for (i, seg) in segs.iter().enumerate() {
            if seg.amount <= 0 {
                continue;
            }
            let sweep = seg.amount as f32 / total * TAU;
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
                "{}: {} ({:.0}%)",
                seg.label,
                money(seg.amount),
                seg.amount as f32 / total * 100.0
            ));
        }
    }

    painter.text(
        center - Vec2::new(0.0, 9.0),
        Align2::CENTER_CENTER,
        center_top,
        FontId::proportional(26.0),
        TEXT,
    );
    painter.text(
        center + Vec2::new(0.0, 14.0),
        Align2::CENTER_CENTER,
        center_bottom,
        FontId::proportional(12.0),
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

fn detail(ui: &mut Ui, label: &str, value: &str) {
    ui.label(caps(label));
    ui.add(egui::Label::new(RichText::new(value).size(14.5).color(TEXT)).truncate());
    ui.add_space(8.0);
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

fn chip(ui: &mut Ui, label: &str, selected: bool) -> bool {
    ui.add(
        egui::Button::new(RichText::new(label).size(13.0).strong().color(if selected {
            PURPLE
        } else {
            MUTED
        }))
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

fn account_item(ui: &mut Ui, account: &Account, selected: bool, kind: AccountType) -> Response {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 66.0), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);

    let fill = if selected {
        PURPLE_BG
    } else if resp.hovered() {
        ROW_ALT
    } else {
        CARD
    };
    let border = if selected { PURPLE } else { BORDER };

    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(10), border);
    painter.rect_filled(rect.shrink(1.0), CornerRadius::same(9), fill);

    // Avatar
    let avatar = Pos2::new(rect.left() + 32.0, rect.center().y);
    painter.circle_filled(avatar, 19.0, if selected { PURPLE } else { PURPLE_BG });
    painter.text(
        avatar,
        Align2::CENTER_CENTER,
        initials(&account.name),
        FontId::proportional(13.0),
        if selected { Color32::WHITE } else { PURPLE },
    );

    // Text
    let balance = account.balance(kind);
    let max_chars = ((rect.width() - 190.0) / 8.0).max(8.0) as usize;
    let text_x = rect.left() + 60.0;

    painter.text(
        Pos2::new(text_x, rect.top() + 21.0),
        Align2::LEFT_CENTER,
        truncate(&account.name, max_chars),
        FontId::proportional(14.5),
        TEXT,
    );

    let sub = {
        let mut parts: Vec<&str> = Vec::new();
        if !account.phone.trim().is_empty() {
            parts.push(account.phone.as_str());
        }
        if !account.city.trim().is_empty() {
            parts.push(account.city.as_str());
        }
        if !account.active {
            parts.push("Inactive");
        }
        if parts.is_empty() {
            "—".to_owned()
        } else {
            parts.join("  ·  ")
        }
    };
    painter.text(
        Pos2::new(text_x, rect.top() + 44.0),
        Align2::LEFT_CENTER,
        truncate(&sub, max_chars + 4),
        FontId::proportional(12.5),
        MUTED,
    );

    painter.text(
        Pos2::new(rect.right() - 14.0, rect.top() + 21.0),
        Align2::RIGHT_CENTER,
        money(balance),
        FontId::proportional(14.5),
        if balance > 0 { ORANGE } else { GREEN },
    );

    let (status, color) = if account.over_limit(kind) {
        ("Over limit", RED)
    } else if balance > 0 {
        (
            if kind == AccountType::Customers {
                "Receivable"
            } else {
                "Payable"
            },
            ORANGE,
        )
    } else {
        ("Settled", GREEN)
    };
    painter.text(
        Pos2::new(rect.right() - 14.0, rect.top() + 44.0),
        Align2::RIGHT_CENTER,
        status,
        FontId::proportional(12.0),
        color,
    );

    resp
}

fn draw_error(ui: &mut Ui, app: &AsterApp, state: &mut State) {
    card(ui, |ui| {
        ui.label(
            RichText::new("Accounts could not be loaded")
                .size(17.0)
                .strong()
                .color(RED),
        );
        ui.add_space(4.0);
        ui.label(RichText::new(&state.error).size(13.0).color(MUTED));
        ui.add_space(10.0);
        if ui
            .add(egui::Button::new("Retry").min_size(Vec2::new(90.0, 34.0)))
            .clicked()
        {
            load_accounts(app, state);
        }
    });
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

// ───────────────────────── Formatting ─────────────────────────
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
