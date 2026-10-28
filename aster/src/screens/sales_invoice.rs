use eframe::egui::{self, Align, Color32, Id, Margin, RichText, Stroke, Vec2};

use crate::{
    app::AsterApp,
    database::{CustomerRecord, ProductRecord, SalesInvoiceDraft, SalesLineDraft},
};

const INK: Color32 = Color32::from_rgb(43, 39, 52);
const MUTED: Color32 = Color32::from_rgb(100, 94, 111);
const SOFT: Color32 = Color32::from_rgb(127, 120, 139);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const PURPLE_DARK: Color32 = Color32::from_rgb(74, 50, 120);
const TINT: Color32 = Color32::from_rgb(241, 237, 248);
const BORDER: Color32 = Color32::from_rgb(226, 223, 232);
//const CANVAS: Color32 = Color32::from_rgb(246, 245, 249);
const GREEN: Color32 = Color32::from_rgb(38, 110, 72);
const GREEN_BG: Color32 = Color32::from_rgb(232, 245, 238);
const RED: Color32 = Color32::from_rgb(176, 58, 58);
const AMBER: Color32 = Color32::from_rgb(200, 133, 30);

const PAGE_ID: &str = "aster_sales_invoice_state";

#[derive(Clone, Default)]
struct InvoiceLine {
    product_id: i64,
    name: String,
    brand: String,
    unit: String,
    hsn: String,
    gst_percent: f64,
    quantity: String,
    rate: String,
    discount_percent: String,
}

impl InvoiceLine {
    fn quantity(&self) -> f64 {
        self.quantity.trim().parse::<f64>().unwrap_or(0.0)
    }

    fn rate(&self) -> f64 {
        self.rate.trim().parse::<f64>().unwrap_or(0.0)
    }

    fn discount_percent(&self) -> f64 {
        self.discount_percent
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0)
            .clamp(0.0, 100.0)
    }

    fn gross(&self) -> f64 {
        (self.quantity() * self.rate()).max(0.0)
    }

    fn discount(&self) -> f64 {
        self.gross() * self.discount_percent() / 100.0
    }

    fn taxable(&self) -> f64 {
        (self.gross() - self.discount()).max(0.0)
    }

    fn tax(&self) -> f64 {
        self.taxable() * self.gst_percent.max(0.0) / 100.0
    }

    fn total(&self) -> f64 {
        self.taxable() + self.tax()
    }

    fn valid(&self) -> bool {
        self.product_id > 0
            && self.quantity() > 0.0
            && self.rate() >= 0.0
            && self.discount_percent() <= 100.0
    }
}

#[derive(Clone, Default)]
struct SalesState {
    year: i32,

    invoice_date: String,
    customer_search: String,
    product_search: String,

    customer_results: Vec<CustomerRecord>,
    product_results: Vec<ProductRecord>,

    selected_customer: Option<CustomerRecord>,
    lines: Vec<InvoiceLine>,

    payment_mode: usize,
    amount_received: String,
    overall_discount: String,
    notes: String,

    customer_loaded_for: String,
    product_loaded_for: String,

    customer_error: String,
    product_error: String,
    message: String,

    show_customer_results: bool,
    show_product_results: bool,
}

impl SalesState {
    fn reset(&mut self, date: &str, year: i32) {
        *self = Self {
            year,
            invoice_date: date.to_owned(),
            ..Default::default()
        };
    }

    fn subtotal(&self) -> f64 {
        self.lines.iter().map(InvoiceLine::gross).sum()
    }

    fn line_discount(&self) -> f64 {
        self.lines.iter().map(InvoiceLine::discount).sum()
    }

    fn overall_discount_value(&self) -> f64 {
        let percentage = self
            .overall_discount
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0)
            .clamp(0.0, 100.0);

        (self.subtotal() - self.line_discount()).max(0.0) * percentage / 100.0
    }

    fn taxable(&self) -> f64 {
        (self.subtotal() - self.line_discount() - self.overall_discount_value()).max(0.0)
    }

    fn tax(&self) -> f64 {
        let line_tax: f64 = self.lines.iter().map(InvoiceLine::tax).sum();

        let before_global_discount = (self.subtotal() - self.line_discount()).max(0.0);

        if before_global_discount <= 0.0 {
            return 0.0;
        }

        let global_discount_ratio = (self.taxable() / before_global_discount).clamp(0.0, 1.0);

        line_tax * global_discount_ratio
    }

    fn total(&self) -> f64 {
        self.taxable() + self.tax()
    }

    fn received(&self) -> f64 {
        self.amount_received
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0)
            .max(0.0)
    }

    fn balance(&self) -> f64 {
        (self.total() - self.received()).max(0.0)
    }

    fn change(&self) -> f64 {
        (self.received() - self.total()).max(0.0)
    }

    fn valid(&self) -> bool {
        self.selected_customer.is_some()
            && !self.lines.is_empty()
            && self.lines.iter().all(InvoiceLine::valid)
            && self
                .overall_discount
                .trim()
                .parse::<f64>()
                .unwrap_or(0.0)
                .is_finite()
            && self
                .overall_discount
                .trim()
                .parse::<f64>()
                .unwrap_or(0.0)
                .clamp(0.0, 100.0)
                <= 100.0
            && self.received() >= 0.0
    }
}

fn money(value: f64) -> String {
    format_currency(value)
}

fn section_title(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(15.0).strong().color(INK));

    if !subtitle.is_empty() {
        ui.add_space(3.0);
        ui.label(RichText::new(subtitle).size(12.0).color(MUTED));
    }

    ui.add_space(12.0);
}

fn card(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(12.0)
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            content(ui);
        });
}

fn small_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(11.5).strong().color(SOFT));
}

fn primary_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(
            RichText::new(label)
                .size(13.0)
                .strong()
                .color(Color32::WHITE),
        )
        .fill(if enabled {
            PURPLE
        } else {
            Color32::from_gray(190)
        })
        .corner_radius(8.0)
        .min_size(Vec2::new(132.0, 36.0)),
    )
}

fn secondary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).size(12.5).color(INK))
            .fill(Color32::from_rgb(250, 249, 252))
            .stroke(Stroke::new(1.0_f32, BORDER))
            .corner_radius(8.0)
            .min_size(Vec2::new(84.0, 34.0)),
    )
}

fn load_customers(app: &AsterApp, state: &mut SalesState) {
    let search = state.customer_search.trim().to_owned();

    if state.customer_loaded_for == search {
        return;
    }

    state.customer_loaded_for = search.clone();
    state.customer_error.clear();

    match app.db.as_ref() {
        Some(db) => match db.customers(state.year, &search) {
            Ok(rows) => {
                state.customer_results = rows.into_iter().filter(|row| row.active).collect();
            }
            Err(error) => {
                state.customer_results.clear();
                state.customer_error = error;
            }
        },
        None => {
            state.customer_results.clear();
            state.customer_error = "Database is unavailable.".to_owned();
        }
    }
}

fn load_products(app: &AsterApp, state: &mut SalesState) {
    let search = state.product_search.trim().to_owned();

    if state.product_loaded_for == search {
        return;
    }

    state.product_loaded_for = search.clone();
    state.product_error.clear();

    match app.db.as_ref() {
        Some(db) => match db.products(state.year, &search) {
            Ok(rows) => {
                state.product_results = rows.into_iter().filter(|row| row.active).collect();
            }
            Err(error) => {
                state.product_results.clear();
                state.product_error = error;
            }
        },
        None => {
            state.product_results.clear();
            state.product_error = "Database is unavailable.".to_owned();
        }
    }
}

fn format_currency(value: f64) -> String {
    let formatted = format!("{:.2}", value);
    let (integer, decimal) = formatted.split_once('.').unwrap_or((&formatted, "00"));
    let negative = integer.starts_with('-');
    let digits = integer.trim_start_matches('-');

    let mut grouped = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0
            && (digits.len() - i == 3 || (digits.len() - i > 3 && (digits.len() - i - 3) % 2 == 0))
        {
            grouped.push(',');
        }
        grouped.push(ch);
    }

    format!(
        "₹{}{}.{}",
        if negative { "-" } else { "" },
        grouped,
        decimal
    )
}

fn add_product(state: &mut SalesState, product: &ProductRecord) {
    if let Some(line) = state
        .lines
        .iter_mut()
        .find(|line| line.product_id == product.id)
    {
        let next = line.quantity() + 1.0;
        line.quantity = next.to_string();
        state.message = format!("Added another unit of {}.", product.name);
        return;
    }

    state.lines.push(InvoiceLine {
        product_id: product.id,
        name: product.name.clone(),
        brand: product.brand_name.clone(),
        unit: product.unit_name.clone(),
        hsn: product.hsn_code.clone(),
        gst_percent: product.gst_percent.max(0.0),
        quantity: "1".to_owned(),
        rate: String::new(),
        discount_percent: "0".to_owned(),
    });

    state.message = format!("{} added to the invoice.", product.name);
}

fn show_customer_section(ui: &mut egui::Ui, state: &mut SalesState) {
    section_title(
        ui,
        "Customer details",
        "Select the customer receiving this invoice.",
    );

    small_label(ui, "SEARCH CUSTOMER");
    ui.add_space(5.0);

    let response = ui.add(
        egui::TextEdit::singleline(&mut state.customer_search)
            .hint_text("Name, phone, GSTIN or city")
            .desired_width(f32::INFINITY),
    );

    if response.changed() {
        state.customer_loaded_for.clear();
        state.show_customer_results = true;
        state.message.clear();
    }

    if let Some(customer) = state.selected_customer.clone() {
        ui.add_space(10.0);

        egui::Frame::new()
            .fill(TINT)
            .corner_radius(9.0)
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&customer.name).size(14.0).strong().color(INK));

                        if !customer.phone.is_empty() {
                            ui.label(RichText::new(&customer.phone).size(12.0).color(MUTED));
                        }

                        if !customer.gstin.is_empty() {
                            ui.label(
                                RichText::new(format!("GSTIN: {}", customer.gstin))
                                    .size(11.0)
                                    .color(MUTED),
                            );
                        }

                        let address = [
                            customer.address_line1.as_str(),
                            customer.address_line2.as_str(),
                            customer.city.as_str(),
                            customer.pincode.as_str(),
                        ]
                        .into_iter()
                        .filter(|part| !part.trim().is_empty())
                        .collect::<Vec<_>>()
                        .join(", ");

                        if !address.is_empty() {
                            ui.label(RichText::new(address).size(11.5).color(MUTED));
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if secondary_button(ui, "Change").clicked() {
                            state.selected_customer = None;
                            state.show_customer_results = true;
                        }
                    });
                });
            });
    } else if state.show_customer_results {
        ui.add_space(6.0);

        if !state.customer_error.is_empty() {
            ui.colored_label(RED, &state.customer_error);
        } else if state.customer_results.is_empty() {
            ui.label(
                RichText::new("No matching active customers.")
                    .size(12.0)
                    .color(MUTED),
            );
        } else {
            egui::Frame::new()
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(8.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .show(ui, |ui| {
                            for customer in state.customer_results.iter() {
                                let label = if customer.phone.is_empty() {
                                    customer.name.clone()
                                } else {
                                    format!("{}  ·  {}", customer.name, customer.phone)
                                };

                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(label).size(12.5).color(INK),
                                        )
                                        .fill(Color32::TRANSPARENT)
                                        .corner_radius(5.0)
                                        .min_size(Vec2::new(ui.available_width(), 32.0)),
                                    )
                                    .clicked()
                                {
                                    state.selected_customer = Some(customer.clone());
                                    state.customer_search = customer.name.clone();
                                    state.show_customer_results = false;
                                    state.message.clear();
                                }
                            }
                        });
                });
        }
    }

    ui.add_space(12.0);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            small_label(ui, "INVOICE DATE");
            ui.add_space(4.0);
            ui.add(egui::TextEdit::singleline(&mut state.invoice_date).desired_width(140.0));
        });

        ui.add_space(16.0);

        ui.vertical(|ui| {
            small_label(ui, "FINANCIAL YEAR");
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("{}–{}", state.year, state.year + 1))
                    .size(13.0)
                    .strong()
                    .color(INK),
            );
        });
    });
}

fn show_product_picker(ui: &mut egui::Ui, state: &mut SalesState, products: &[ProductRecord]) {
    section_title(
        ui,
        "Add products",
        "Search medicines and add them to the invoice.",
    );

    let response = ui.add(
        egui::TextEdit::singleline(&mut state.product_search)
            .hint_text("Medicine, brand, generic name or barcode")
            .desired_width(f32::INFINITY),
    );

    if response.changed() {
        state.product_loaded_for.clear();
        state.show_product_results = true;
        state.message.clear();
    }

    if !state.product_error.is_empty() {
        ui.add_space(6.0);
        ui.colored_label(RED, &state.product_error);
        return;
    }

    if !state.show_product_results {
        return;
    }

    ui.add_space(7.0);

    if products.is_empty() {
        ui.label(
            RichText::new("No matching active products.")
                .size(12.0)
                .color(MUTED),
        );
        return;
    }

    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(9.0)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(210.0)
                .show(ui, |ui| {
                    for product in products.iter() {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(&product.name).size(12.5).strong().color(INK),
                                );

                                let details = [
                                    product.brand_name.as_str(),
                                    product.generic_name.as_str(),
                                    product.unit_name.as_str(),
                                ]
                                .into_iter()
                                .filter(|s| !s.trim().is_empty())
                                .collect::<Vec<_>>()
                                .join(" · ");

                                if !details.is_empty() {
                                    ui.label(RichText::new(details).size(10.5).color(MUTED));
                                }

                                ui.label(
                                    RichText::new(format!(
                                        "GST {}%  ·  HSN {}",
                                        product.gst_percent,
                                        if product.hsn_code.is_empty() {
                                            "—"
                                        } else {
                                            &product.hsn_code
                                        }
                                    ))
                                    .size(10.5)
                                    .color(SOFT),
                                );
                            });

                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("+ Add").strong().color(PURPLE),
                                        )
                                        .fill(TINT)
                                        .corner_radius(7.0),
                                    )
                                    .clicked()
                                {
                                    add_product(state, product);
                                    state.show_product_results = false;
                                }
                            });
                        });

                        ui.add_space(5.0);
                        ui.painter().hline(
                            ui.min_rect().x_range(),
                            ui.cursor().top(),
                            Stroke::new(1.0_f32, BORDER),
                        );
                        ui.add_space(5.0);
                    }
                });
        });
}

fn show_invoice_lines(ui: &mut egui::Ui, state: &mut SalesState) {
    section_title(
        ui,
        "Invoice items",
        "Enter quantities, selling rates and item discounts.",
    );

    let width = ui.available_width();

    let columns = [
        ("PRODUCT", 0.28),
        ("QTY", 0.09),
        ("RATE ₹", 0.13),
        ("DISC %", 0.10),
        ("GST", 0.08),
        ("AMOUNT", 0.19),
        ("", 0.06),
    ];

    egui::Frame::new()
        .fill(Color32::from_rgb(249, 248, 252))
        .corner_radius(7.0)
        .inner_margin(Margin::symmetric(8, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;

                for (label, fraction) in columns {
                    ui.allocate_ui_with_layout(
                        Vec2::new((width * fraction - 8.0).max(30.0), 20.0),
                        egui::Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.label(RichText::new(label).size(9.5).strong().color(SOFT));
                        },
                    );
                }
            });
        });

    if state.lines.is_empty() {
        egui::Frame::new()
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0_f32, BORDER))
            .corner_radius(9.0)
            .inner_margin(Margin::same(28))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("No items added yet")
                            .size(15.0)
                            .strong()
                            .color(INK),
                    );

                    ui.add_space(4.0);

                    ui.label(
                        RichText::new("Search for a medicine above to begin building the invoice.")
                            .size(12.0)
                            .color(MUTED),
                    );
                });
            });

        return;
    }

    let mut remove_index = None;

    for (index, line) in state.lines.iter_mut().enumerate() {
        egui::Frame::new()
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0_f32, BORDER))
            .corner_radius(8.0)
            .inner_margin(Margin::symmetric(8, 9))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;

                    ui.allocate_ui_with_layout(
                        Vec2::new((width * 0.28 - 8.0).max(60.0), 52.0),
                        egui::Layout::top_down(Align::Min),
                        |ui| {
                            ui.label(RichText::new(&line.name).size(11.5).strong().color(INK));

                            ui.label(
                                RichText::new(format!("HSN: {}", line.hsn))
                                    .size(10.5)
                                    .color(MUTED),
                            );

                            if !line.brand.is_empty() {
                                ui.label(RichText::new(&line.brand).size(10.0).color(MUTED));
                            }

                            if !line.unit.is_empty() {
                                ui.label(RichText::new(&line.unit).size(9.5).color(SOFT));
                            }
                        },
                    );

                    ui.allocate_ui_with_layout(
                        Vec2::new((width * 0.09 - 8.0).max(38.0), 34.0),
                        egui::Layout::top_down(Align::Center),
                        |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut line.quantity)
                                    .id(Id::new(("sales_qty", line.product_id)))
                                    .desired_width(f32::INFINITY),
                            );
                        },
                    );

                    ui.allocate_ui_with_layout(
                        Vec2::new((width * 0.13 - 8.0).max(48.0), 34.0),
                        egui::Layout::top_down(Align::Center),
                        |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut line.rate)
                                    .id(Id::new(("sales_rate", line.product_id)))
                                    .hint_text("0.00")
                                    .desired_width(f32::INFINITY),
                            );
                        },
                    );

                    ui.allocate_ui_with_layout(
                        Vec2::new((width * 0.10 - 8.0).max(40.0), 34.0),
                        egui::Layout::top_down(Align::Center),
                        |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut line.discount_percent)
                                    .id(Id::new(("sales_disc", line.product_id)))
                                    .desired_width(f32::INFINITY),
                            );
                        },
                    );

                    ui.allocate_ui_with_layout(
                        Vec2::new((width * 0.08 - 8.0).max(32.0), 34.0),
                        egui::Layout::top_down(Align::Center),
                        |ui| {
                            ui.label(
                                RichText::new(format!("{:.1}%", line.gst_percent))
                                    .size(10.5)
                                    .color(MUTED),
                            );
                        },
                    );

                    ui.allocate_ui_with_layout(
                        Vec2::new((width * 0.19 - 8.0).max(55.0), 34.0),
                        egui::Layout::top_down(Align::Center),
                        |ui| {
                            ui.label(
                                RichText::new(money(line.total()))
                                    .size(11.5)
                                    .strong()
                                    .color(INK),
                            );
                        },
                    );

                    ui.allocate_ui_with_layout(
                        Vec2::new((width * 0.06 - 8.0).max(24.0), 34.0),
                        egui::Layout::top_down(Align::Center),
                        |ui| {
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("×").size(18.0).color(RED))
                                        .fill(Color32::from_rgb(253, 242, 242))
                                        .corner_radius(6.0),
                                )
                                .clicked()
                            {
                                remove_index = Some(index);
                            }
                        },
                    );
                });

                if line.quantity.trim().parse::<f64>().unwrap_or(0.0) <= 0.0 {
                    ui.colored_label(
                        RED,
                        RichText::new("Quantity must be greater than zero.").size(10.0),
                    );
                }

                if line.rate.trim().parse::<f64>().unwrap_or(-1.0) < 0.0 {
                    ui.colored_label(
                        AMBER,
                        RichText::new("Enter a valid selling rate.").size(10.0),
                    );
                }
            });

        ui.add_space(5.0);
    }

    if let Some(index) = remove_index {
        let name = state.lines[index].name.clone();
        state.lines.remove(index);
        state.message = format!("{name} removed from the invoice.");
    }
}

fn show_totals(ui: &mut egui::Ui, state: &mut SalesState) {
    section_title(
        ui,
        "Invoice summary",
        "Review the amounts before completing the sale.",
    );

    let summary_row = |ui: &mut egui::Ui, label: &str, amount: f64| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).size(12.0).color(MUTED));

            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(money(amount)).size(12.0).color(INK));
            });
        });

        ui.add_space(7.0);
    };

    summary_row(ui, "Subtotal", state.subtotal());
    summary_row(ui, "Item discounts", state.line_discount());

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Additional discount (%)")
                .size(11.5)
                .color(MUTED),
        );

        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.add(egui::TextEdit::singleline(&mut state.overall_discount).desired_width(70.0));
        });
    });

    ui.add_space(8.0);

    summary_row(ui, "Additional discount", state.overall_discount_value());

    summary_row(ui, "Taxable amount", state.taxable());
    summary_row(ui, "GST", state.tax());

    ui.add_space(4.0);

    egui::Frame::new()
        .fill(TINT)
        .corner_radius(9.0)
        .inner_margin(Margin::same(13))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("TOTAL PAYABLE")
                        .size(11.0)
                        .strong()
                        .color(PURPLE),
                );

                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(money(state.total()))
                            .size(21.0)
                            .strong()
                            .color(PURPLE_DARK),
                    );
                });
            });
        });

    ui.add_space(15.0);

    section_title(ui, "Payment", "Record the payment details for this sale.");

    small_label(ui, "PAYMENT MODE");
    ui.add_space(5.0);

    egui::ComboBox::from_id_salt("sales_payment_mode")
        .selected_text(
            ["Cash", "UPI", "Bank transfer", "Cheque", "Credit"]
                .get(state.payment_mode)
                .copied()
                .unwrap_or("Cash"),
        )
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for (index, mode) in ["Cash", "UPI", "Bank transfer", "Cheque", "Credit"]
                .iter()
                .enumerate()
            {
                ui.selectable_value(&mut state.payment_mode, index, *mode);
            }
        });

    ui.add_space(10.0);

    small_label(ui, "AMOUNT RECEIVED (₹)");
    ui.add_space(5.0);

    ui.add(
        egui::TextEdit::singleline(&mut state.amount_received)
            .hint_text("0.00")
            .desired_width(f32::INFINITY),
    );

    ui.add_space(10.0);

    let received = state.received();

    ui.horizontal(|ui| {
        ui.label(RichText::new("Received").size(11.5).color(MUTED));

        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(money(received)).size(12.0).color(INK));
        });
    });

    ui.horizontal(|ui| {
        ui.label(RichText::new("Balance due").size(11.5).color(MUTED));

        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(money(state.balance()))
                    .size(12.0)
                    .strong()
                    .color(if state.balance() > 0.0 { AMBER } else { GREEN }),
            );
        });
    });

    if state.change() > 0.0 {
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Change to return").size(11.5).color(MUTED));

            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(money(state.change()))
                        .size(12.0)
                        .strong()
                        .color(GREEN),
                );
            });
        });
    }
}

// AsterApp stores the working session date as DD/MM/YYYY, while invoice
// records require ISO YYYY-MM-DD. Convert at the UI boundary.

fn working_date_indian(date_text: &str) -> String {
    let parts: Vec<&str> = date_text.trim().split('/').collect();

    if parts.len() == 3 {
        if let (Ok(day), Ok(month), Ok(year)) = (
            parts[0].parse::<u32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            let leap_year = year % 400 == 0 || (year % 4 == 0 && year % 100 != 0);
            let max_day = match month {
                2 if leap_year => 29,
                2 => 28,
                4 | 6 | 9 | 11 => 30,
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                _ => 0,
            };

            if year > 0 && day > 0 && day <= max_day {
                return format!("{day:02}/{month:02}/{year:04}");
            }
        }
    }

    let (day, month, year) = crate::date::local_date();
    format!("{day:02}/{month:02}/{year:04}")
}

fn indian_date_to_iso(date_text: &str) -> Result<String, String> {
    let text = date_text.trim();

    if text.len() != 10
        || text.as_bytes()[2] != b'/'
        || text.as_bytes()[5] != b'/'
        || !text.as_bytes()[..2].iter().all(u8::is_ascii_digit)
        || !text.as_bytes()[3..5].iter().all(u8::is_ascii_digit)
        || !text.as_bytes()[6..].iter().all(u8::is_ascii_digit)
    {
        return Err("Enter the date in DD/MM/YYYY format.".to_string());
    }

    let day: u32 = text[0..2].parse().map_err(|_| "Invalid day.".to_string())?;
    let month: u32 = text[3..5]
        .parse()
        .map_err(|_| "Invalid month.".to_string())?;
    let year: u32 = text[6..10]
        .parse()
        .map_err(|_| "Invalid year.".to_string())?;

    if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
        return Err("Enter a valid calendar date.".to_string());
    }

    let leap_year = year % 400 == 0 || (year % 4 == 0 && year % 100 != 0);

    let days_in_month = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };

    if day == 0 || day > days_in_month {
        return Err("Enter a valid calendar date.".to_string());
    }

    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

pub(crate) fn show(ui: &mut egui::Ui, app: &mut AsterApp) {
    let id = Id::new(PAGE_ID);

    let mut state = ui
        .ctx()
        .data(|data| data.get_temp::<SalesState>(id))
        .unwrap_or_default();

    if state.year != app.selected_year {
        state.reset(&working_date_indian(&app.date_text), app.selected_year);
    }

    if state.invoice_date.is_empty() {
        state.invoice_date = working_date_indian(&app.date_text);
    }

    load_customers(app, &mut state);
    load_products(app, &mut state);

    ui.label(
        RichText::new("TRANSACTIONS  /  SALES")
            .size(10.5)
            .strong()
            .color(SOFT),
    );

    ui.add_space(5.0);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("Sales invoice")
                    .size(27.0)
                    .strong()
                    .color(INK),
            );

            ui.add_space(3.0);

            ui.label(
                RichText::new("Create a customer bill and review its items, taxes and payment.")
                    .size(13.0)
                    .color(MUTED),
            );
        });

        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            let db_connected = app.db.is_some();

            egui::Frame::new()
                .fill(if db_connected {
                    GREEN_BG
                } else {
                    Color32::from_rgb(253, 238, 238)
                })
                .corner_radius(20.0)
                .inner_margin(Margin::symmetric(10, 5))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(if db_connected {
                            "●  Database connected"
                        } else {
                            "●  Database unavailable"
                        })
                        .size(10.5)
                        .strong()
                        .color(if db_connected { GREEN } else { RED }),
                    );
                });
        });
    });

    ui.add_space(18.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;

        for (label, value, color) in [
            ("ITEMS", state.lines.len().to_string(), PURPLE),
            ("SUBTOTAL", money(state.subtotal()), INK),
            (
                "DISCOUNTS",
                money(state.line_discount() + state.overall_discount_value()),
                AMBER,
            ),
            ("TOTAL", money(state.total()), GREEN),
        ] {
            ui.allocate_ui_with_layout(
                Vec2::new((ui.available_width() / 4.0 - 8.0).max(100.0), 68.0),
                egui::Layout::top_down(Align::Min),
                |ui| {
                    egui::Frame::new()
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .corner_radius(10.0)
                        .inner_margin(Margin::same(12))
                        .show(ui, |ui| {
                            ui.label(RichText::new(label).size(9.5).strong().color(SOFT));

                            ui.add_space(4.0);

                            ui.label(RichText::new(value).size(17.0).strong().color(color));
                        });
                },
            );
        }
    });

    ui.add_space(14.0);

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;

        let left_width = (ui.available_width() * 0.67).max(300.0);
        let right_width = (ui.available_width() - left_width - 12.0).max(250.0);

        ui.allocate_ui_with_layout(
            Vec2::new(left_width, 0.0),
            egui::Layout::top_down(Align::Min),
            |ui| {
                card(ui, |ui| {
                    show_customer_section(ui, &mut state);
                });

                ui.add_space(12.0);

                card(ui, |ui| {

                    let product_results = state.product_results.clone();
                    show_product_picker(
                        ui,
                        &mut state,
                        &product_results,
                    );
                });

                ui.add_space(12.0);

                card(ui, |ui| {
                    show_invoice_lines(ui, &mut state);
                });

                ui.add_space(12.0);

                card(ui, |ui| {
                    section_title(
                        ui,
                        "Invoice notes",
                        "Optional notes for this transaction.",
                    );

                    ui.add(
                        egui::TextEdit::multiline(&mut state.notes)
                            .hint_text("Delivery instructions or other remarks…")
                            .desired_rows(2)
                            .desired_width(f32::INFINITY),
                    );
                });
            },
        );

        ui.allocate_ui_with_layout(
            Vec2::new(right_width, 0.0),
            egui::Layout::top_down(Align::Min),
            |ui| {
                card(ui, |ui| {
                    show_totals(ui, &mut state);
                });

                ui.add_space(12.0);

                card(ui, |ui| {
                    section_title(
                        ui,
                        "Invoice actions",
                        "Review the invoice before posting.",
                    );

                    ui.label(RichText::new("Posting saves the invoice, payment and batch stock movements in one database transaction.").size(11.5).color(MUTED));
                    ui.add_space(12.0);
                    let ready = state.valid();
                    if primary_button(ui, "Post invoice", ready).clicked() {
                        if let (Some(db), Some(customer)) = (&app.db, &state.selected_customer) {
                            let parse_money = |value: &str| -> Result<i64, String> {
                                if value.trim().is_empty() { return Ok(0); }
                                let v = value.trim().parse::<f64>().map_err(|_| "Enter a valid monetary amount.".to_owned())?;
                                if !v.is_finite() || v < 0.0 || v > (i64::MAX as f64 / 100.0) { return Err("Monetary amount is out of range.".into()); }
                                Ok((v * 100.0).round() as i64)
                            };
                            let lines = state.lines.iter().map(|line| {
                                let quantity = line.quantity();
                                let rate = parse_money(&line.rate)?;
                                if !quantity.is_finite() || quantity <= 0.0 || quantity > (i64::MAX as f64 / 1000.0) { return Err("Enter a valid positive quantity.".to_owned()); }
                                Ok(SalesLineDraft { product_id: line.product_id, quantity_milli: (quantity * 1000.0).round() as i64, rate_paise: rate, discount_basis_points: (line.discount_percent() * 100.0).round() as i64 })
                            }).collect::<Result<Vec<_>, String>>();
                            let overall_discount = if state.overall_discount.trim().is_empty() { Ok(0.0) } else { state.overall_discount.trim().parse::<f64>().map_err(|_| "Enter a valid overall discount.".to_owned()) };
                            let received = parse_money(&state.amount_received);
                            match (lines, overall_discount, received) {
                                (Ok(lines), Ok(discount), Ok(received)) => {
                                    let payment_mode = ["Cash", "UPI", "Card", "Bank transfer", "Credit"]
                                        .get(state.payment_mode)
                                        .copied()
                                        .unwrap_or("Cash");

                                    match indian_date_to_iso(&state.invoice_date) {
                                        Ok(invoice_date) => {
                                            let draft = SalesInvoiceDraft {
                                                customer_id: customer.id,
                                                invoice_date,
                                                overall_discount_basis_points: (discount * 100.0).round() as i64,
                                                amount_received_paise: received,
                                                payment_mode: payment_mode.to_owned(),
                                                notes: state.notes.clone(),
                                                lines,
                                            };

                                            match db.post_sales_invoice(state.year, &draft) {
                                                Ok(invoice_no) => {
                                                    state.message = format!(
                                                        "Invoice {invoice_no} posted successfully. Stock and payment were recorded."
                                                    );
                                                    state.lines.clear();
                                                    state.amount_received.clear();
                                                    state.overall_discount.clear();
                                                    state.notes.clear();
                                                }
                                                Err(error) => state.message = error,
                                            }
                                        }
                                        Err(error) => state.message = error,
                                    }
                                }
                                (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => {
                                    state.message = error;
                                }
                            }

                        } else { state.message = "Database is not available or the customer selection is missing.".into(); }
                    }

                    if !state.message.is_empty() {
                        ui.add_space(6.0);
                        ui.label(RichText::new(&state.message).size(11.5).color(if state.message.contains("posted successfully") { GREEN } else { RED }));
                    }
                    ui.add_space(6.0);

                    if secondary_button(ui, "Clear draft").clicked() {
                        state.reset(&working_date_indian(&app.date_text), app.selected_year);
                    }

                    ui.add_space(10.0);

                    ui.label(
                        RichText::new(if ready {
                            "Draft validation: ready for database integration."
                        } else {
                            "Select a customer, add items and enter valid quantities and rates."
                        })
                        .size(10.5)
                        .color(if ready { GREEN } else { MUTED }),
                    );

                    ui.add_space(6.0);

                    ui.label(
                        RichText::new("Draft only · not saved")
                            .size(10.5)
                            .strong()
                            .color(AMBER),
                    );
                });
            },
        );
    });

    if !state.message.is_empty() {
        ui.add_space(8.0);

        ui.label(RichText::new(&state.message).size(11.5).color(PURPLE));
    }

    ui.add_space(16.0);

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("ASTER  ·  FY {}–{}", state.year, state.year + 1,))
                .size(10.5)
                .color(SOFT),
        );

        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new("Amounts shown in INR").size(10.5).color(SOFT));
        });
    });

    ui.ctx().data_mut(|data| {
        data.insert_temp(id, state);
    });
}
