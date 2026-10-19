use eframe::egui::{self, Align, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::app::AsterApp;

const INK: Color32 = Color32::from_rgb(43, 39, 52);
const MUTED: Color32 = Color32::from_rgb(100, 94, 111);
const SOFT: Color32 = Color32::from_rgb(127, 120, 139);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const BORDER: Color32 = Color32::from_rgb(226, 223, 232);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum WorkspacePage {
    Dashboard,
    Company,
    Customers,
    Suppliers,
    Products,
    Purchase,
    Sales,
    Stock,
    Accounts,
    Reports,
    Settings,
    UserPreferences,
}

#[derive(Clone, Copy)]
enum NavIcon {
    Dashboard,
    Building,
    Person,
    Box,
    Purchase,
    Sales,
    Stock,
    Accounts,
    Reports,
    Settings,
    Profile,
}

impl WorkspacePage {
    fn title(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Company => "Company settings",
            Self::Customers => "Customers",
            Self::Suppliers => "Suppliers",
            Self::Products => "Products and medicines",
            Self::Purchase => "Purchase",
            Self::Sales => "Sales invoice",
            Self::Stock => "Stock",
            Self::Accounts => "Accounts",
            Self::Reports => "Reports",
            Self::Settings => "System settings",
            Self::UserPreferences => "User preferences",
        }
    }

    fn icon(self) -> NavIcon {
        match self {
            Self::Dashboard => NavIcon::Dashboard,
            Self::Company => NavIcon::Building,
            Self::Customers | Self::Suppliers => NavIcon::Person,
            Self::Products => NavIcon::Box,
            Self::Purchase => NavIcon::Purchase,
            Self::Sales => NavIcon::Sales,
            Self::Stock => NavIcon::Stock,
            Self::Accounts => NavIcon::Accounts,
            Self::Reports => NavIcon::Reports,
            Self::Settings => NavIcon::Settings,
            Self::UserPreferences => NavIcon::Profile,
        }
    }
}

impl AsterApp {
    pub(crate) fn show_main(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("main_header")
            .exact_height(78.0)
            .frame(egui::Frame::new().fill(Color32::WHITE))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new("ASTER")
                            .font(crate::theme::brand_font(26.0))
                            .color(PURPLE),
                    );
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("MEDICAL WHOLESALE")
                            .size(9.5)
                            .strong()
                            .color(SOFT),
                    );
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(10.0);
                    let tabs_width = (ui.available_width() - 236.0).max(120.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(tabs_width, 60.0),
                        egui::Layout::left_to_right(Align::Center),
                        |ui| show_open_tabs(ui, self),
                    );
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if profile_button(ui, self).clicked() {
                            self.open_page(WorkspacePage::UserPreferences);
                        }
                        ui.add_space(20.0);
                        ui.add_space(8.0);
                    });
                });
            });

        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(34.0)
            .frame(egui::Frame::new().fill(Color32::WHITE))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(17.0);
                    ui.painter().circle_filled(
                        ui.cursor().left_center() + Vec2::new(4.0, 0.0),
                        3.5,
                        Color32::from_rgb(45, 146, 94),
                    );
                    ui.add_space(11.0);
                    ui.label(RichText::new("Database connected").size(12.5).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("Ready").size(12.5).color(MUTED));
                        ui.add_space(17.0);
                    });
                });
            });

        let sidebar_progress = ctx.animate_bool(
            egui::Id::new("aster_sidebar_animation"),
            !self.sidebar_collapsed,
        );
        if sidebar_progress > 0.0 && sidebar_progress < 1.0 {
            ctx.request_repaint();
        }
        let sidebar_width = 72.0 + sidebar_progress * 166.0;
        let compact_navigation = sidebar_progress < 0.55;
        egui::SidePanel::left("navigation")
            .resizable(false)
            .exact_width(sidebar_width)
            .frame(
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.0_f32, BORDER)),
            )
            .show(ctx, |ui| {
                ui.add_space(15.0);
                if compact_navigation {
                    ui.vertical_centered(|ui| {
                        if collapse_button(ui, true).clicked() {
                            self.sidebar_collapsed = false;
                        }
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.add_space(19.0);
                        ui.label(RichText::new("NAVIGATION").size(11.0).strong().color(SOFT));
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            ui.add_space(11.0);
                            if collapse_button(ui, false).clicked() {
                                self.sidebar_collapsed = true;
                            }
                        });
                    });
                }
                ui.add_space(15.0);

                nav_group(ui, "WORKSPACE", compact_navigation);
                nav_item(
                    ui,
                    &mut self.workspace_page,
                    WorkspacePage::Dashboard,
                    "Overview",
                    compact_navigation,
                );
                nav_group(ui, "MASTERS", compact_navigation);
                nav_item(
                    ui,
                    &mut self.workspace_page,
                    WorkspacePage::Company,
                    "Company settings",
                    compact_navigation,
                );
                nav_item(
                    ui,
                    &mut self.workspace_page,
                    WorkspacePage::Customers,
                    "Customers",
                    compact_navigation,
                );
                nav_item(
                    ui,
                    &mut self.workspace_page,
                    WorkspacePage::Suppliers,
                    "Suppliers",
                    compact_navigation,
                );
                nav_item(
                    ui,
                    &mut self.workspace_page,
                    WorkspacePage::Products,
                    "Products / Medicines",
                    compact_navigation,
                );
                nav_group(ui, "TRANSACTIONS", compact_navigation);
                nav_item(
                    ui,
                    &mut self.workspace_page,
                    WorkspacePage::Purchase,
                    "Purchase",
                    compact_navigation,
                );
                nav_item(
                    ui,
                    &mut self.workspace_page,
                    WorkspacePage::Sales,
                    "Sales invoice",
                    compact_navigation,
                );
                nav_item(
                    ui,
                    &mut self.workspace_page,
                    WorkspacePage::Stock,
                    "Stock",
                    compact_navigation,
                );
                nav_group(ui, "ACCOUNTS & REPORTS", compact_navigation);
                nav_item(
                    ui,
                    &mut self.workspace_page,
                    WorkspacePage::Accounts,
                    "Accounts",
                    compact_navigation,
                );
                nav_item(
                    ui,
                    &mut self.workspace_page,
                    WorkspacePage::Reports,
                    "Reports",
                    compact_navigation,
                );
                nav_group(ui, "SYSTEM", compact_navigation);
                nav_item(
                    ui,
                    &mut self.workspace_page,
                    WorkspacePage::Settings,
                    "Settings",
                    compact_navigation,
                );
                self.open_page(self.workspace_page);
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(246, 245, 249))
                    .inner_margin(egui::Margin::same(30)),
            )
            .show(ctx, |ui| {
                let available = ui.available_rect_before_wrap();
                let width = available.width().min(1500.0);
                let bounds = Rect::from_min_size(
                    Pos2::new(available.center().x - width / 2.0, available.top()),
                    Vec2::new(width, available.height()),
                );
                let mut content = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(bounds)
                        .layout(egui::Layout::top_down(Align::Min)),
                );
                match self.workspace_page {
                    WorkspacePage::Dashboard => show_dashboard(&mut content),
                    WorkspacePage::UserPreferences => show_user_preferences(&mut content, self),
                    page => show_module_intro(&mut content, page),
                }
            });
    }
}

fn show_open_tabs(ui: &mut egui::Ui, app: &mut AsterApp) {
    let open_pages = app.open_pages.clone();
    let mut close_page = None;
    egui::ScrollArea::horizontal()
        .id_salt("aster_open_workspace_tabs")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for page in open_pages {
                    let selected = app.workspace_page == page;
                    let activation = ui
                        .ctx()
                        .animate_bool(egui::Id::new(("aster_open_tab", page)), selected);
                    if activation > 0.0 && activation < 1.0 {
                        ui.ctx().request_repaint();
                    }
                    let fill = if selected {
                        Color32::from_rgb(
                            (248.0 - activation * 10.0) as u8,
                            (247.0 - activation * 13.0) as u8,
                            (251.0 - activation * 5.0) as u8,
                        )
                    } else {
                        Color32::from_rgb(250, 249, 252)
                    };
                    let stroke = if selected {
                        Stroke::new(1.0_f32, Color32::from_rgb(207, 195, 226))
                    } else {
                        Stroke::new(1.0_f32, Color32::from_rgb(235, 232, 241))
                    };
                    egui::Frame::new()
                        .fill(fill)
                        .stroke(stroke)
                        .corner_radius(7.0)
                        .inner_margin(egui::Margin::symmetric(9, 5))
                        .show(ui, |ui| {
                            if ui
                                .add(
                                    egui::Label::new(
                                        RichText::new(page.title()).size(12.5).color(if selected {
                                            PURPLE
                                        } else {
                                            MUTED
                                        }),
                                    )
                                    .sense(Sense::click()),
                                )
                                .clicked()
                            {
                                app.workspace_page = page;
                            }
                            if page != WorkspacePage::Dashboard {
                                let close = ui
                                    .add(
                                        egui::Label::new(RichText::new("×").size(15.0).color(SOFT))
                                            .sense(Sense::click()),
                                    )
                                    .on_hover_text("Close tab");
                                if close.clicked() {
                                    close_page = Some(page);
                                }
                            }
                        });
                    ui.add_space(4.0);
                }
            });
        });
    if let Some(page) = close_page {
        app.close_page(page);
    }
}

fn profile_button(ui: &mut egui::Ui, app: &AsterApp) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(204.0, 58.0), Sense::click());
    let fill = if response.hovered() {
        Color32::from_rgb(248, 246, 251)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 8.0, fill);
    let avatar = Pos2::new(rect.left() + 23.0, rect.center().y);
    ui.painter()
        .circle_filled(avatar, 15.0, Color32::from_rgb(235, 228, 247));
    let initials = app
        .display_name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    ui.painter().text(
        avatar,
        egui::Align2::CENTER_CENTER,
        if initials.is_empty() { "U" } else { &initials },
        FontId::proportional(12.0),
        PURPLE,
    );
    let name = if app.display_name.is_empty() {
        &app.current_user
    } else {
        &app.display_name
    };
    ui.painter().text(
        Pos2::new(rect.left() + 47.0, rect.top() + 13.0),
        egui::Align2::LEFT_TOP,
        name,
        FontId::proportional(13.5),
        INK,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 47.0, rect.top() + 31.0),
        egui::Align2::LEFT_TOP,
        format!("{}  ·  {}", app.current_user, app.date_text),
        FontId::proportional(10.5),
        MUTED,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 47.0, rect.top() + 44.0),
        egui::Align2::LEFT_TOP,
        format!("FY {} – {}", app.selected_year, app.selected_year + 1),
        FontId::proportional(10.5),
        PURPLE,
    );
    response.on_hover_text("Open user preferences")
}

fn collapse_button(ui: &mut egui::Ui, collapsed: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
    let fill = if response.hovered() {
        Color32::from_rgb(242, 239, 247)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    let center = rect.center();
    let color = Color32::from_rgb(100, 91, 116);
    let stroke = Stroke::new(1.6_f32, color);
    if collapsed {
        ui.painter().line_segment(
            [center + Vec2::new(-3.0, -5.0), center + Vec2::new(2.0, 0.0)],
            stroke,
        );
        ui.painter().line_segment(
            [center + Vec2::new(2.0, 0.0), center + Vec2::new(-3.0, 5.0)],
            stroke,
        );
    } else {
        ui.painter().line_segment(
            [center + Vec2::new(3.0, -5.0), center + Vec2::new(-2.0, 0.0)],
            stroke,
        );
        ui.painter().line_segment(
            [center + Vec2::new(-2.0, 0.0), center + Vec2::new(3.0, 5.0)],
            stroke,
        );
    }
    response.on_hover_text(if collapsed {
        "Expand navigation"
    } else {
        "Collapse navigation"
    })
}

fn nav_group(ui: &mut egui::Ui, label: &str, collapsed: bool) {
    if collapsed {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(5.0);
    } else {
        ui.add_space(11.0);
        ui.label(RichText::new(label).size(11.0).strong().color(SOFT));
        ui.add_space(5.0);
    }
}

fn nav_item(
    ui: &mut egui::Ui,
    current: &mut WorkspacePage,
    page: WorkspacePage,
    label: &str,
    collapsed: bool,
) {
    if collapsed {
        ui.horizontal_centered(|ui| {
            draw_nav_item(ui, current, page, label, true, 48.0);
        });
    } else {
        draw_nav_item(ui, current, page, label, false, ui.available_width());
    }
}

fn draw_nav_item(
    ui: &mut egui::Ui,
    current: &mut WorkspacePage,
    page: WorkspacePage,
    label: &str,
    collapsed: bool,
    width: f32,
) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 40.0), Sense::click());
    let selected = *current == page;
    if selected || response.hovered() {
        let fill = if selected {
            Color32::from_rgb(241, 237, 248)
        } else {
            Color32::from_rgb(248, 247, 250)
        };
        ui.painter().rect_filled(rect, 7.0, fill);
    }
    let color = if selected { PURPLE } else { MUTED };
    let icon_center = if collapsed {
        rect.center()
    } else {
        Pos2::new(rect.left() + 23.0, rect.center().y)
    };
    draw_icon(ui.painter(), icon_center, page.icon(), color);
    if !collapsed {
        ui.painter().text(
            Pos2::new(rect.left() + 44.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(14.5),
            color,
        );
    }
    if response.clicked() {
        *current = page;
    }
    if collapsed {
        response.on_hover_text(label);
    }
}

fn draw_icon(painter: &egui::Painter, center: Pos2, icon: NavIcon, color: Color32) {
    let stroke = Stroke::new(1.65_f32, color);
    let line = |a: Vec2, b: Vec2| painter.line_segment([center + a, center + b], stroke);
    match icon {
        NavIcon::Dashboard => {
            for offset in [(-3.5, -3.5), (3.5, -3.5), (-3.5, 3.5), (3.5, 3.5)] {
                painter.circle_filled(center + Vec2::new(offset.0, offset.1), 1.8, color);
            }
        }
        NavIcon::Building => {
            painter.rect_stroke(
                Rect::from_center_size(center, Vec2::new(12.0, 14.0)),
                1.5,
                stroke,
                egui::StrokeKind::Inside,
            );
            for x in [-3.0, 3.0] {
                for y in [-3.0, 1.0] {
                    painter.circle_filled(center + Vec2::new(x, y), 0.8, color);
                }
            }
            line(Vec2::new(0.0, 7.0), Vec2::new(0.0, 3.0));
        }
        NavIcon::Person => {
            painter.circle_stroke(center + Vec2::new(0.0, -3.0), 3.0, stroke);
            painter.line_segment(
                [center + Vec2::new(-6.0, 6.0), center + Vec2::new(-4.0, 2.0)],
                stroke,
            );
            painter.line_segment(
                [center + Vec2::new(-4.0, 2.0), center + Vec2::new(4.0, 2.0)],
                stroke,
            );
            painter.line_segment(
                [center + Vec2::new(4.0, 2.0), center + Vec2::new(6.0, 6.0)],
                stroke,
            );
        }
        NavIcon::Box => {
            painter.rect_stroke(
                Rect::from_center_size(center, Vec2::new(13.0, 12.0)),
                2.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            line(Vec2::new(-6.0, -2.0), Vec2::new(6.0, -2.0));
            line(Vec2::new(0.0, -5.0), Vec2::new(0.0, -2.0));
        }
        NavIcon::Purchase => {
            line(Vec2::new(-6.0, -3.5), Vec2::new(5.0, -3.5));
            line(Vec2::new(5.0, -3.5), Vec2::new(1.5, -7.0));
            line(Vec2::new(5.0, -3.5), Vec2::new(1.5, 0.0));
            line(Vec2::new(6.0, 3.5), Vec2::new(-5.0, 3.5));
            line(Vec2::new(-5.0, 3.5), Vec2::new(-1.5, 0.0));
            line(Vec2::new(-5.0, 3.5), Vec2::new(-1.5, 7.0));
        }
        NavIcon::Sales => {
            painter.rect_stroke(
                Rect::from_center_size(center, Vec2::new(13.0, 15.0)),
                2.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            for y in [-3.0, 1.0, 5.0] {
                line(Vec2::new(-3.0, y), Vec2::new(3.0, y));
            }
        }
        NavIcon::Stock => {
            for y in [-4.0, 0.0, 4.0] {
                line(Vec2::new(-6.0, y), Vec2::new(6.0, y));
            }
        }
        NavIcon::Accounts => {
            painter.circle_stroke(center, 7.0, stroke);
            line(Vec2::new(0.0, -4.0), Vec2::new(0.0, 4.0));
            line(Vec2::new(-2.5, -2.0), Vec2::new(2.5, -2.0));
            line(Vec2::new(-2.5, 2.0), Vec2::new(2.5, 2.0));
        }
        NavIcon::Reports => {
            painter.line_segment(
                [center + Vec2::new(-6.0, 6.0), center + Vec2::new(6.0, 6.0)],
                stroke,
            );
            for (x, height) in [(-4.0, 5.0), (0.0, 9.0), (4.0, 7.0)] {
                line(Vec2::new(x, 5.0), Vec2::new(x, 5.0 - height));
            }
        }
        NavIcon::Settings => {
            painter.circle_stroke(center, 5.0, stroke);
            painter.circle_filled(center, 1.5, color);
            for (a, b) in [
                (Vec2::new(0.0, -8.0), Vec2::new(0.0, -6.5)),
                (Vec2::new(0.0, 8.0), Vec2::new(0.0, 6.5)),
                (Vec2::new(-8.0, 0.0), Vec2::new(-6.5, 0.0)),
                (Vec2::new(8.0, 0.0), Vec2::new(6.5, 0.0)),
            ] {
                line(a, b);
            }
        }
        NavIcon::Profile => {
            painter.circle_stroke(center + Vec2::new(0.0, -3.0), 3.0, stroke);
            painter.line_segment(
                [center + Vec2::new(-6.0, 6.0), center + Vec2::new(-4.0, 2.0)],
                stroke,
            );
            painter.line_segment(
                [center + Vec2::new(-4.0, 2.0), center + Vec2::new(4.0, 2.0)],
                stroke,
            );
            painter.line_segment(
                [center + Vec2::new(4.0, 2.0), center + Vec2::new(6.0, 6.0)],
                stroke,
            );
        }
    }
}

fn show_dashboard(ui: &mut egui::Ui) {
    ui.label(
        RichText::new("BUSINESS OVERVIEW")
            .size(11.0)
            .strong()
            .color(SOFT),
    );
    ui.add_space(5.0);
    ui.label(RichText::new("Dashboard").size(30.0).strong().color(INK));
    ui.add_space(2.0);
    ui.label(
        RichText::new("A clear view of your billing and stock activity.")
            .size(15.0)
            .color(MUTED),
    );
    ui.add_space(24.0);

    let metrics = [
        ("Today's sales", "₹—", "Invoice totals appear here"),
        ("Stock alerts", "—", "Low stock items"),
        ("Receivables", "₹—", "Outstanding balances"),
        ("Expiring batches", "—", "Upcoming expiry dates"),
    ];
    ui.columns(4, |columns| {
        for (column, (title, value, note)) in columns.iter_mut().zip(metrics) {
            egui::Frame::new()
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(10.0)
                .inner_margin(18.0)
                .show(column, |ui| {
                    ui.set_min_height(112.0);
                    ui.label(RichText::new(title).size(13.0).strong().color(MUTED));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(value)
                            .font(FontId::proportional(29.0))
                            .strong()
                            .color(INK),
                    );
                    ui.add_space(2.0);
                    ui.label(RichText::new(note).size(12.5).color(SOFT));
                });
        }
    });

    ui.add_space(22.0);
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(10.0)
        .inner_margin(22.0)
        .show(ui, |ui| {
            ui.label(
                RichText::new("Recent activity")
                    .size(17.0)
                    .strong()
                    .color(INK),
            );
            ui.add_space(3.0);
            ui.label(
                RichText::new("Invoices and stock updates will appear here.")
                    .size(14.0)
                    .color(MUTED),
            );
            ui.add_space(18.0);
            ui.separator();
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), 170.0),
                egui::Layout::top_down(Align::Center),
                |ui| {
                    ui.add_space(42.0);
                    ui.label(
                        RichText::new("No activity yet")
                            .size(15.0)
                            .strong()
                            .color(MUTED),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Choose a section from the left to get started.")
                            .size(13.0)
                            .color(SOFT),
                    );
                },
            );
        });
}

fn show_user_preferences(ui: &mut egui::Ui, app: &mut AsterApp) {
    ui.label(RichText::new("ACCOUNT").size(11.0).strong().color(SOFT));
    ui.add_space(5.0);
    ui.label(
        RichText::new("User preferences")
            .size(30.0)
            .strong()
            .color(INK),
    );
    ui.add_space(3.0);
    ui.label(
        RichText::new("Manage your profile, workspace preferences, and sign-in security.")
            .size(15.0)
            .color(MUTED),
    );
    ui.add_space(22.0);

    ui.columns(2, |columns| {
        let left = &mut columns[0];
        section_card(
            left,
            "Profile",
            "How you appear in the ASTER workspace",
            |ui| {
                preference_field(ui, "Display name", &mut app.display_name, "Your name");
                preference_field(
                    ui,
                    "Email address",
                    &mut app.profile_email,
                    "name@company.com",
                );
                ui.add_space(8.0);
                if primary_action(ui, "Save profile").clicked() {
                    if app.display_name.trim().is_empty() {
                        app.profile_message = "Enter a display name before saving.".into();
                    } else {
                        app.profile_message =
                            "Profile changes are applied for this session.".into();
                    }
                }
            },
        );
        left.add_space(18.0);
        section_card(left, "Workspace", "Personal display choices", |ui| {
            ui.checkbox(&mut app.compact_density, "Use compact spacing");
            ui.add_space(8.0);
            ui.checkbox(&mut app.show_tooltips, "Show navigation tooltips");
            ui.add_space(8.0);
            ui.checkbox(&mut app.reduced_motion, "Reduce interface motion");
            ui.add_space(12.0);
            ui.label(
                RichText::new("These preferences apply while this session is open.")
                    .size(12.0)
                    .color(SOFT),
            );
        });

        let right = &mut columns[1];
        section_card(
            right,
            "Security",
            "Change the password for this account",
            |ui| {
                password_field(ui, "Current password", &mut app.current_password);
                password_field(ui, "New password", &mut app.new_password);
                password_field(ui, "Confirm password", &mut app.confirm_new_password);
                ui.add_space(8.0);
                if primary_action(ui, "Update password").clicked() {
                    if app.new_password.len() < 10 {
                        app.profile_message =
                            "Use a new password with at least 10 characters.".into();
                    } else if app.new_password != app.confirm_new_password {
                        app.profile_message = "The new passwords do not match.".into();
                    } else if let Some(db) = &app.db {
                        match db.change_password(
                            &app.current_user,
                            &app.current_password,
                            &app.new_password,
                        ) {
                            Ok(()) => {
                                app.current_password.clear();
                                app.new_password.clear();
                                app.confirm_new_password.clear();
                                app.profile_message = "Password updated successfully.".into();
                            }
                            Err(error) => app.profile_message = error,
                        }
                    } else {
                        app.profile_message = "Database is unavailable.".into();
                    }
                }
            },
        );
        right.add_space(18.0);
        section_card(right, "Account", "Your signed-in access", |ui| {
            preference_readonly(ui, "User name", &app.current_user);
            preference_readonly(ui, "Role", "Administrator");
            preference_readonly(ui, "Status", "Active");
        });
    });

    if !app.profile_message.is_empty() {
        ui.add_space(18.0);
        ui.label(
            RichText::new(&app.profile_message)
                .size(13.0)
                .color(Color32::from_rgb(78, 112, 85)),
        );
    }
}

fn section_card(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    content: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(10.0)
        .inner_margin(20.0)
        .show(ui, |ui| {
            ui.label(RichText::new(title).size(17.0).strong().color(INK));
            ui.add_space(3.0);
            ui.label(RichText::new(subtitle).size(12.5).color(MUTED));
            ui.add_space(16.0);
            content(ui);
        });
}

fn preference_field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    ui.label(RichText::new(label).size(12.0).strong().color(MUTED));
    ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(ui.available_width())
            .hint_text(hint),
    );
    ui.add_space(9.0);
}

fn password_field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(RichText::new(label).size(12.0).strong().color(MUTED));
    ui.add(
        egui::TextEdit::singleline(value)
            .password(true)
            .desired_width(ui.available_width()),
    );
    ui.add_space(9.0);
}

fn preference_readonly(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.0).color(MUTED));
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).size(13.0).strong().color(INK));
        });
    });
    ui.add_space(7.0);
}

fn primary_action(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(12.5)
                .strong()
                .color(Color32::WHITE),
        )
        .fill(PURPLE)
        .corner_radius(6.0)
        .min_size(Vec2::new(120.0, 31.0)),
    )
}

fn show_module_intro(ui: &mut egui::Ui, page: WorkspacePage) {
    ui.label(RichText::new("WORKSPACE").size(11.0).strong().color(SOFT));
    ui.add_space(5.0);
    ui.label(RichText::new(page.title()).size(30.0).strong().color(INK));
    ui.add_space(22.0);
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(10.0)
        .inner_margin(28.0)
        .show(ui, |ui| {
            ui.set_min_height(190.0);
            ui.vertical_centered(|ui| {
                ui.add_space(35.0);
                ui.label(
                    RichText::new(format!("{} workspace", page.title()))
                        .size(20.0)
                        .strong()
                        .color(INK),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "This screen is being prepared for a later implementation phase.",
                    )
                    .size(14.0)
                    .color(MUTED),
                );
            });
        });
}
