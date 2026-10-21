use std::time::Duration;

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Id, Margin, Pos2, Rect, RichText, Sense, Stroke,
    Vec2,
};

use crate::app::AsterApp;

const INK: Color32 = Color32::from_rgb(43, 39, 52);
const MUTED: Color32 = Color32::from_rgb(100, 94, 111);
const SOFT: Color32 = Color32::from_rgb(127, 120, 139);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const PURPLE_DEEP: Color32 = Color32::from_rgb(74, 50, 120);
const PURPLE_TINT: Color32 = Color32::from_rgb(241, 237, 248);
const BORDER: Color32 = Color32::from_rgb(226, 223, 232);
const CANVAS: Color32 = Color32::from_rgb(246, 245, 249);
const SUCCESS: Color32 = Color32::from_rgb(45, 146, 94);
const SUCCESS_TEXT: Color32 = Color32::from_rgb(38, 110, 72);
const SUCCESS_TINT: Color32 = Color32::from_rgb(232, 245, 238);
const DANGER: Color32 = Color32::from_rgb(176, 58, 58);
const AMBER: Color32 = Color32::from_rgb(200, 133, 30);

const HEADER_HEIGHT: f32 = 56.0;
const STATUS_HEIGHT: f32 = 30.0;
const SIDEBAR_COLLAPSED: f32 = 64.0;
const SIDEBAR_EXPANDED: f32 = 240.0;
const CONTENT_MAX_WIDTH: f32 = 1400.0;

const TAB_MIN_WIDTH: f32 = 104.0;
const TAB_MAX_WIDTH: f32 = 208.0;
const TAB_GAP: f32 = 2.0;

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
    Truck,
    Box,
    Purchase,
    Sales,
    Stock,
    Accounts,
    Reports,
    Settings,
    Profile,
}

const NAV: &[(&str, &[(WorkspacePage, &str)])] = &[
    ("WORKSPACE", &[(WorkspacePage::Dashboard, "Overview")]),
    (
        "MASTERS",
        &[
            (WorkspacePage::Company, "Company settings"),
            (WorkspacePage::Customers, "Customers"),
            (WorkspacePage::Suppliers, "Suppliers"),
            (WorkspacePage::Products, "Products / Medicines"),
        ],
    ),
    (
        "TRANSACTIONS",
        &[
            (WorkspacePage::Purchase, "Purchase"),
            (WorkspacePage::Sales, "Sales invoice"),
            (WorkspacePage::Stock, "Stock"),
        ],
    ),
    (
        "ACCOUNTS & REPORTS",
        &[
            (WorkspacePage::Accounts, "Accounts"),
            (WorkspacePage::Reports, "Reports"),
        ],
    ),
    ("SYSTEM", &[(WorkspacePage::Settings, "Settings")]),
];

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
            Self::Customers => NavIcon::Person,
            Self::Suppliers => NavIcon::Truck,
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

// ------------------------------------------------------------------ helpers

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// Global animation duration (0 when the user prefers reduced motion).
fn anim(ctx: &egui::Context) -> f32 {
    ctx.data(|d| d.get_temp::<f32>(Id::new("aster_anim_time")))
        .unwrap_or(0.14)
}

fn initials_of(app: &AsterApp) -> String {
    let initials = app
        .display_name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    if initials.is_empty() {
        app.current_user
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "U".to_string())
    } else {
        initials
    }
}

fn message_is_ok(message: &str) -> bool {
    message.contains("successfully") || message.contains("saved") || message.contains("applied")
}

/// Fades and slides content in once when a page is first shown.
fn reveal(ui: &mut egui::Ui, key: &'static str, index: usize, content: impl FnOnce(&mut egui::Ui)) {
    let ctx = ui.ctx().clone();
    let now = ui.input(|i| i.time);
    let start = ui.data_mut(|d| *d.get_temp_mut_or_insert_with(Id::new(key), || now));
    let t = if anim(&ctx) == 0.0 {
        1.0
    } else {
        (((now - start) - index as f64 * 0.06) / 0.32).clamp(0.0, 1.0) as f32
    };
    let eased = 1.0 - (1.0 - t).powi(3);
    if t < 1.0 {
        ctx.request_repaint();
    }
    ui.vertical(|ui| {
        ui.set_opacity(eased);
        ui.add_space((1.0 - eased) * 10.0);
        content(ui);
    });
}

impl AsterApp {
    pub(crate) fn show_main(&mut self, ctx: &egui::Context) {
        ctx.data_mut(|d| {
            d.insert_temp(
                Id::new("aster_anim_time"),
                if self.reduced_motion {
                    0.0_f32
                } else {
                    0.14_f32
                },
            )
        });

        // Keyboard shortcuts: Ctrl+W closes the tab, Ctrl+Tab cycles tabs.
        let (close_current, next_tab) = ctx.input(|i| {
            (
                i.modifiers.command && i.key_pressed(egui::Key::W),
                i.modifiers.ctrl && i.key_pressed(egui::Key::Tab),
            )
        });
        if close_current {
            self.close_page(self.workspace_page);
        }
        if next_tab && !self.open_pages.is_empty() {
            let index = self
                .open_pages
                .iter()
                .position(|p| *p == self.workspace_page)
                .unwrap_or(0);
            self.workspace_page = self.open_pages[(index + 1) % self.open_pages.len()];
        }

        // Restart entrance animations whenever a page is left.
        if self.workspace_page != WorkspacePage::UserPreferences {
            ctx.data_mut(|d| d.remove::<f64>(Id::new("aster_prefs_enter")));
        }
        if self.workspace_page != WorkspacePage::Dashboard {
            ctx.data_mut(|d| d.remove::<f64>(Id::new("aster_dash_enter")));
        }
        if self.workspace_page != WorkspacePage::Company {
            ctx.data_mut(|d| d.remove::<f64>(Id::new("aster_company_enter")));
        }

        // ---------------------------------------------------------------- header
        egui::TopBottomPanel::top("main_header")
            .exact_height(HEADER_HEIGHT)
            .frame(egui::Frame::new().fill(Color32::WHITE))
            .show(ctx, |ui| {
                let full = ui.max_rect();
                ui.painter().hline(
                    full.x_range(),
                    full.bottom() - 0.5,
                    Stroke::new(1.0_f32, BORDER),
                );

                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(22.0);

                    ui.allocate_ui_with_layout(
                        Vec2::new(210.0, HEADER_HEIGHT),
                        egui::Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            if let Some(logo) = &self.logo {
                                ui.add(egui::Image::new(logo).fit_to_exact_size(Vec2::splat(36.0)));
                            }
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new("ASTER")
                                        .font(crate::theme::brand_font(24.0))
                                        .color(PURPLE),
                                );
                                ui.label(
                                    RichText::new("MEDICAL WHOLESALE")
                                        .size(9.0)
                                        .strong()
                                        .color(SOFT),
                                );
                            });
                        },
                    );

                    ui.add_space(8.0);
                    let (div, _) = ui.allocate_exact_size(Vec2::new(1.0, 26.0), Sense::hover());
                    ui.painter().rect_filled(div, 0.0, BORDER);
                    ui.add_space(12.0);

                    let tabs_width = (ui.available_width() - 232.0).max(120.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(tabs_width, HEADER_HEIGHT),
                        egui::Layout::left_to_right(Align::Min),
                        |ui| show_open_tabs(ui, self),
                    );

                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(16.0);
                        if profile_button(ui, self).clicked() {
                            self.open_page(WorkspacePage::UserPreferences);
                        }
                    });
                });
            });

        // ------------------------------------------------------------ status bar
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(STATUS_HEIGHT)
            .frame(egui::Frame::new().fill(Color32::WHITE))
            .show(ctx, |ui| {
                let full = ui.max_rect();
                ui.painter().hline(
                    full.x_range(),
                    full.top() + 0.5,
                    Stroke::new(1.0_f32, BORDER),
                );
                let connected = self.db.is_some();
                ui.horizontal_centered(|ui| {
                    ui.add_space(18.0);
                    let (dot, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                    ui.painter().circle_filled(
                        dot.center(),
                        3.5,
                        if connected { SUCCESS } else { DANGER },
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(if connected {
                            "Database connected"
                        } else {
                            "Database unavailable"
                        })
                        .size(12.0)
                        .color(MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(18.0);
                        ui.label(
                            RichText::new(format!(
                                "FY {}–{}   ·   {}",
                                self.selected_year,
                                self.selected_year + 1,
                                self.date_text
                            ))
                            .size(12.0)
                            .color(MUTED),
                        );
                    });
                });
            });

        // --------------------------------------------------------------- sidebar
        let sidebar_target = !self.sidebar_collapsed;
        let sidebar_progress = if self.reduced_motion {
            if sidebar_target { 1.0 } else { 0.0 }
        } else {
            ctx.animate_bool_with_time(
                egui::Id::new("aster_sidebar_animation"),
                sidebar_target,
                0.2,
            )
        };
        if sidebar_progress > 0.0 && sidebar_progress < 1.0 {
            ctx.request_repaint();
        }
        let sidebar_width =
            SIDEBAR_COLLAPSED + sidebar_progress * (SIDEBAR_EXPANDED - SIDEBAR_COLLAPSED);
        let compact_navigation = sidebar_progress < 0.55;
        let tooltips = self.show_tooltips;

        egui::SidePanel::left("navigation")
            .resizable(false)
            .exact_width(sidebar_width)
            .frame(
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.0_f32, BORDER)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.add_space(14.0);

                if compact_navigation {
                    ui.vertical_centered(|ui| {
                        if collapse_button(ui, true).clicked() {
                            self.sidebar_collapsed = false;
                        }
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.add_space(22.0);
                        ui.label(RichText::new("NAVIGATION").size(11.0).strong().color(SOFT));
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            ui.add_space(12.0);
                            if collapse_button(ui, false).clicked() {
                                self.sidebar_collapsed = true;
                            }
                        });
                    });
                }
                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for (group, items) in NAV {
                            nav_group(ui, group, compact_navigation);
                            for (page, label) in items.iter() {
                                nav_item(
                                    ui,
                                    &mut self.workspace_page,
                                    *page,
                                    label,
                                    compact_navigation,
                                    tooltips,
                                );
                            }
                        }
                        ui.add_space(12.0);
                    });
            });

        if !self.open_pages.contains(&self.workspace_page) {
            self.open_page(self.workspace_page);
            ctx.request_repaint();
        }

        // --------------------------------------------------------------- content
        let margin = if self.compact_density {
            Margin::symmetric(20, 16)
        } else {
            Margin::symmetric(32, 26)
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(CANVAS).inner_margin(margin))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let available = ui.available_width();
                        let width = available.min(CONTENT_MAX_WIDTH);
                        let pad = ((available - width) / 2.0).max(0.0);
                        ui.horizontal_top(|ui| {
                            ui.add_space(pad);
                            ui.allocate_ui_with_layout(
                                Vec2::new(width, 0.0),
                                egui::Layout::top_down(Align::Min),
                                |ui| {
                                    match self.workspace_page {
                                        WorkspacePage::Dashboard => show_dashboard(ui, self),
                                        WorkspacePage::UserPreferences => {
                                            show_user_preferences(ui, self)
                                        }
                                        WorkspacePage::Company => {
                                            super::company_settings::show(ui, self)
                                        }
                                        WorkspacePage::Products => {
                                            super::product_master::show(ui, self)
                                        }
                                        page => show_module_intro(ui, page),
                                    }
                                    ui.add_space(24.0);
                                },
                            );
                        });
                    });
            });

        show_toast(ctx, self);
    }
}

// ---------------------------------------------------------------------- tabs

/// Browser / file-explorer style tab strip: tabs share the available width,
/// shrink down to a minimum as more are opened, then scroll. The active tab
/// is joined to the page below it.
fn show_open_tabs(ui: &mut egui::Ui, app: &mut AsterApp) {
    let ctx = ui.ctx().clone();
    let anim_time = anim(&ctx);
    let open_pages = app.open_pages.clone();
    let count = open_pages.len().max(1) as f32;
    let strip = ui.available_width();
    let ideal = ((strip - (count - 1.0) * TAB_GAP) / count)
        .clamp(TAB_MIN_WIDTH, TAB_MAX_WIDTH)
        .floor();

    let last_selected_id = Id::new("aster_last_selected_tab");
    let last_selected: Option<WorkspacePage> = ui.data(|d| d.get_temp(last_selected_id));
    let selection_changed = last_selected != Some(app.workspace_page);
    ui.data_mut(|d| d.insert_temp(last_selected_id, app.workspace_page));

    let mut close_page = None;
    let mut select_page = None;

    egui::ScrollArea::horizontal()
        .id_salt("aster_open_workspace_tabs")
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(TAB_GAP, 0.0);
                for (index, page) in open_pages.iter().copied().enumerate() {
                    let selected = app.workspace_page == page;
                    let closable = page != WorkspacePage::Dashboard;
                    let next_selected = open_pages
                        .get(index + 1)
                        .map(|next| *next == app.workspace_page)
                        .unwrap_or(false);

                    // New tabs grow in from zero width; resizing is smooth.
                    let width_id = Id::new(("aster_tab_width", page));
                    let born_id = width_id.with("born");
                    let first_frame = !ui.data(|d| d.get_temp::<bool>(born_id).unwrap_or(false));
                    if first_frame {
                        ui.data_mut(|d| d.insert_temp(born_id, true));
                        if anim_time > 0.0 {
                            ctx.animate_value_with_time(width_id, 0.0, 0.0);
                        }
                    }
                    let width = ctx
                        .animate_value_with_time(width_id, ideal, anim_time * 1.6)
                        .max(1.0);

                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width, HEADER_HEIGHT), Sense::click());
                    let hover_t = ctx.animate_bool_with_time(
                        width_id.with("hover"),
                        response.hovered(),
                        anim_time,
                    );
                    let sel_t =
                        ctx.animate_bool_with_time(width_id.with("sel"), selected, anim_time * 1.3);

                    let cy = rect.top() + 34.0;
                    let painter = ui.painter();

                    // Hover pill for inactive tabs
                    if hover_t > 0.0 && sel_t < 1.0 {
                        let pill = Rect::from_min_max(
                            Pos2::new(rect.left() + 1.0, rect.top() + 16.0),
                            Pos2::new(rect.right() - 1.0, rect.bottom() - 6.0),
                        );
                        painter.rect_filled(
                            pill,
                            8.0,
                            Color32::from_rgb(244, 242, 249)
                                .gamma_multiply(hover_t * (1.0 - sel_t)),
                        );
                    }

                    // Active tab: merges with the page below
                    if sel_t > 0.0 {
                        let top = CornerRadius {
                            nw: 10,
                            ne: 10,
                            sw: 0,
                            se: 0,
                        };
                        let inner_top = CornerRadius {
                            nw: 9,
                            ne: 9,
                            sw: 0,
                            se: 0,
                        };
                        let outer =
                            Rect::from_min_max(Pos2::new(rect.left(), rect.top() + 14.0), rect.max);
                        painter.rect_filled(outer, top, BORDER.gamma_multiply(sel_t));
                        let inner = Rect::from_min_max(
                            outer.min + Vec2::new(1.0, 1.0),
                            outer.max - Vec2::new(1.0, 0.0),
                        );
                        painter.rect_filled(inner, inner_top, CANVAS.gamma_multiply(sel_t));
                        let accent = Rect::from_min_max(
                            Pos2::new(inner.left() + 12.0, inner.top()),
                            Pos2::new(inner.right() - 12.0, inner.top() + 2.0),
                        );
                        painter.rect_filled(accent, 1.0, PURPLE.gamma_multiply(sel_t));
                    }

                    // Divider between two idle neighbours
                    if !selected && !next_selected && index + 1 < open_pages.len() {
                        painter.vline(
                            rect.right() + TAB_GAP / 2.0,
                            (cy - 8.0)..=(cy + 8.0),
                            Stroke::new(1.0_f32, BORDER.gamma_multiply(1.0 - hover_t)),
                        );
                    }

                    // Icon + title
                    let text_color = mix(MUTED, PURPLE, sel_t);
                    let show_icon = width >= 120.0;
                    let text_x = rect.left() + if show_icon { 38.0 } else { 14.0 };
                    if show_icon {
                        draw_icon(
                            painter,
                            Pos2::new(rect.left() + 21.0, cy),
                            page.icon(),
                            text_color,
                        );
                    }
                    let reserve = if closable { 32.0 } else { 14.0 };
                    let avail = (rect.right() - reserve - text_x).max(0.0);
                    if avail > 18.0 {
                        let mut job = egui::text::LayoutJob::simple_singleline(
                            page.title().to_owned(),
                            FontId::proportional(13.0),
                            text_color,
                        );
                        job.wrap.max_width = avail;
                        job.wrap.max_rows = 1;
                        job.wrap.break_anywhere = true;
                        job.wrap.overflow_character = Some('…');
                        let galley = painter.layout_job(job);
                        painter.galley(
                            Pos2::new(text_x, cy - galley.size().y / 2.0),
                            galley,
                            text_color,
                        );
                    }

                    // Close button fades in on hover / selection
                    let mut close_clicked = false;
                    if closable {
                        let visible = hover_t.max(sel_t);
                        let close_rect = Rect::from_center_size(
                            Pos2::new(rect.right() - 18.0, cy),
                            Vec2::splat(20.0),
                        );
                        let close = ui.interact(close_rect, width_id.with("close"), Sense::click());
                        if visible > 0.05 {
                            let painter = ui.painter();
                            if close.hovered() {
                                painter.circle_filled(
                                    close_rect.center(),
                                    10.0,
                                    Color32::from_rgb(228, 222, 238),
                                );
                                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            let c = close_rect.center();
                            let x = Stroke::new(
                                1.4_f32,
                                mix(SOFT, PURPLE, if close.hovered() { 1.0 } else { 0.0 })
                                    .gamma_multiply(visible),
                            );
                            painter.line_segment(
                                [c + Vec2::new(-3.0, -3.0), c + Vec2::new(3.0, 3.0)],
                                x,
                            );
                            painter.line_segment(
                                [c + Vec2::new(-3.0, 3.0), c + Vec2::new(3.0, -3.0)],
                                x,
                            );
                            if close.clicked() {
                                close_page = Some(page);
                                close_clicked = true;
                            }
                        }
                    }

                    if closable && response.middle_clicked() {
                        close_page = Some(page);
                        close_clicked = true;
                    }
                    if response.clicked() && !close_clicked {
                        select_page = Some(page);
                    }
                    if response.hovered() {
                        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if selected && selection_changed {
                        ui.scroll_to_rect(rect, None);
                    }
                    if width < 150.0 {
                        response.on_hover_text(page.title());
                    }
                }
            });
        });

    if let Some(page) = select_page {
        app.workspace_page = page;
    }
    if let Some(page) = close_page {
        ui.data_mut(|d| d.remove::<bool>(Id::new(("aster_tab_width", page)).with("born")));
        app.close_page(page);
    }
}

fn profile_button(ui: &mut egui::Ui, app: &AsterApp) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(200.0, 44.0), Sense::click());
    let hover_t = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), anim(ui.ctx()));
    if hover_t > 0.0 {
        ui.painter().rect_filled(
            rect,
            10.0,
            Color32::from_rgb(246, 243, 251).gamma_multiply(hover_t),
        );
    }
    let avatar = Pos2::new(rect.left() + 24.0, rect.center().y);
    ui.painter().circle_filled(
        avatar,
        16.0,
        mix(PURPLE_TINT, Color32::from_rgb(230, 222, 244), hover_t),
    );
    ui.painter().text(
        avatar,
        egui::Align2::CENTER_CENTER,
        initials_of(app),
        FontId::proportional(12.5),
        PURPLE,
    );
    let name = if app.display_name.is_empty() {
        &app.current_user
    } else {
        &app.display_name
    };
    ui.painter().text(
        Pos2::new(rect.left() + 48.0, rect.center().y - 2.0),
        egui::Align2::LEFT_BOTTOM,
        name,
        FontId::proportional(13.5),
        INK,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 48.0, rect.center().y + 1.0),
        egui::Align2::LEFT_TOP,
        format!("@{}  ·  Administrator", app.current_user),
        FontId::proportional(10.5),
        MUTED,
    );
    response.on_hover_text("Open user preferences")
}

// ------------------------------------------------------------------ sidebar

fn collapse_button(ui: &mut egui::Ui, collapsed: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), anim(ui.ctx()));
    if t > 0.0 {
        ui.painter().rect_filled(
            rect,
            7.0,
            Color32::from_rgb(242, 239, 247).gamma_multiply(t),
        );
    }
    let center = rect.center();
    let stroke = Stroke::new(1.6_f32, Color32::from_rgb(100, 91, 116));
    let dir = if collapsed { 1.0 } else { -1.0 };
    ui.painter().line_segment(
        [
            center + Vec2::new(-2.5 * dir, -5.0),
            center + Vec2::new(2.5 * dir, 0.0),
        ],
        stroke,
    );
    ui.painter().line_segment(
        [
            center + Vec2::new(2.5 * dir, 0.0),
            center + Vec2::new(-2.5 * dir, 5.0),
        ],
        stroke,
    );
    response.on_hover_text(if collapsed {
        "Expand navigation"
    } else {
        "Collapse navigation"
    })
}

fn nav_group(ui: &mut egui::Ui, label: &str, collapsed: bool) {
    if collapsed {
        ui.add_space(8.0);
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
        ui.painter().hline(
            (rect.left() + 18.0)..=(rect.right() - 18.0),
            rect.center().y,
            Stroke::new(1.0_f32, BORDER),
        );
        ui.add_space(6.0);
    } else {
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(22.0);
            ui.label(RichText::new(label).size(10.5).strong().color(SOFT));
        });
        ui.add_space(4.0);
    }
}

fn nav_item(
    ui: &mut egui::Ui,
    current: &mut WorkspacePage,
    page: WorkspacePage,
    label: &str,
    collapsed: bool,
    tooltips: bool,
) {
    let ctx = ui.ctx().clone();
    let (full, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 38.0), Sense::click());
    let rect = full.shrink2(Vec2::new(10.0, 0.0));
    let selected = *current == page;
    let hover_t = ctx.animate_bool_with_time(response.id, response.hovered(), anim(&ctx));
    let sel_t = ctx.animate_bool_with_time(response.id.with("sel"), selected, anim(&ctx) * 1.4);

    if hover_t > 0.0 {
        ui.painter().rect_filled(
            rect,
            8.0,
            Color32::from_rgb(248, 247, 250).gamma_multiply(hover_t * (1.0 - sel_t)),
        );
    }
    if sel_t > 0.0 {
        ui.painter()
            .rect_filled(rect, 8.0, PURPLE_TINT.gamma_multiply(sel_t));
        let bar_h = 20.0 * sel_t;
        ui.painter().rect_filled(
            Rect::from_min_size(
                Pos2::new(full.left(), rect.center().y - bar_h / 2.0),
                Vec2::new(3.0, bar_h),
            ),
            2.0,
            PURPLE,
        );
    }

    let color = mix(MUTED, PURPLE, sel_t);
    let icon_center = if collapsed {
        rect.center()
    } else {
        Pos2::new(rect.left() + 22.0, rect.center().y)
    };
    draw_icon(ui.painter(), icon_center, page.icon(), color);

    if !collapsed {
        ui.painter().text(
            Pos2::new(rect.left() + 44.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(14.0),
            color,
        );
    }
    if response.clicked() {
        *current = page;
    }
    if collapsed && tooltips {
        response.on_hover_text(label);
    }
}

fn draw_icon(painter: &egui::Painter, center: Pos2, icon: NavIcon, color: Color32) {
    let stroke = Stroke::new(1.65_f32, color);
    let line = |a: Vec2, b: Vec2| painter.line_segment([center + a, center + b], stroke);
    match icon {
        NavIcon::Dashboard => {
            for offset in [(-3.5, -3.5), (3.5, -3.5), (-3.5, 3.5), (3.5, 3.5)] {
                painter.circle_filled(center + Vec2::new(offset.0, offset.1), 1.9, color);
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
        NavIcon::Person | NavIcon::Profile => {
            painter.circle_stroke(center + Vec2::new(0.0, -3.5), 3.0, stroke);
            line(Vec2::new(-6.0, 6.0), Vec2::new(-4.0, 2.0));
            line(Vec2::new(-4.0, 2.0), Vec2::new(4.0, 2.0));
            line(Vec2::new(4.0, 2.0), Vec2::new(6.0, 6.0));
        }
        NavIcon::Truck => {
            painter.rect_stroke(
                Rect::from_min_max(center + Vec2::new(-7.5, -5.5), center + Vec2::new(2.0, 3.0)),
                1.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            line(Vec2::new(2.0, -2.0), Vec2::new(5.0, -2.0));
            line(Vec2::new(5.0, -2.0), Vec2::new(7.5, 1.0));
            line(Vec2::new(7.5, 1.0), Vec2::new(7.5, 3.0));
            line(Vec2::new(7.5, 3.0), Vec2::new(2.0, 3.0));
            painter.circle_stroke(center + Vec2::new(-4.0, 5.5), 1.6, stroke);
            painter.circle_stroke(center + Vec2::new(4.0, 5.5), 1.6, stroke);
        }
        NavIcon::Box => {
            painter.rect_stroke(
                Rect::from_center_size(center, Vec2::new(13.0, 12.0)),
                2.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            line(Vec2::new(-6.0, -2.0), Vec2::new(6.0, -2.0));
            line(Vec2::new(0.0, -6.0), Vec2::new(0.0, -2.0));
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
            line(Vec2::new(-6.0, 6.0), Vec2::new(6.0, 6.0));
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
    }
}

// ------------------------------------------------------------ shared pieces

fn page_header(ui: &mut egui::Ui, eyebrow: &str, title: &str, subtitle: &str) {
    ui.label(RichText::new(eyebrow).size(11.0).strong().color(SOFT));
    ui.add_space(4.0);
    ui.label(RichText::new(title).size(28.0).strong().color(INK));
    if !subtitle.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(subtitle).size(14.5).color(MUTED));
    }
    ui.add_space(22.0);
}

/// A white bordered card with an exact outer width.
fn card(ui: &mut egui::Ui, width: f32, margin: f32, content: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, 0.0),
        egui::Layout::top_down(Align::Min),
        |ui| {
            egui::Frame::new()
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(12.0)
                .inner_margin(margin)
                .show(ui, |ui| {
                    ui.set_width((width - margin * 2.0 - 2.0).max(0.0));
                    content(ui);
                });
        },
    );
}

fn icon_chip(ui: &mut egui::Ui, icon: NavIcon) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
    ui.painter().rect_filled(rect, 9.0, PURPLE_TINT);
    draw_icon(ui.painter(), rect.center(), icon, PURPLE);
}

fn pill(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(20.0)
        .inner_margin(Margin::symmetric(9, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).strong().color(fg));
        });
}

/// Shared button painter with an animated hover colour.
fn button_base(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
    enabled: bool,
    primary: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(width, 34.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let ctx = ui.ctx().clone();
    let t = ctx.animate_bool_with_time(response.id, enabled && response.hovered(), anim(&ctx));
    let pressed = enabled && response.is_pointer_button_down_on();
    let r = if pressed { rect.shrink(0.75) } else { rect };

    if primary {
        let fill = if enabled {
            mix(PURPLE, PURPLE_DEEP, t)
        } else {
            Color32::from_rgb(208, 201, 222)
        };
        ui.painter().rect_filled(r, 8.0, fill);
        ui.painter().text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(13.0),
            Color32::WHITE,
        );
    } else {
        let fill = mix(
            Color32::from_rgb(250, 249, 252),
            Color32::from_rgb(243, 240, 249),
            t,
        );
        let stroke = mix(BORDER, Color32::from_rgb(207, 195, 226), t);
        ui.painter().rect_filled(r, 8.0, fill);
        ui.painter().rect_stroke(
            r,
            8.0,
            Stroke::new(1.0_f32, stroke),
            egui::StrokeKind::Inside,
        );
        let text = if enabled { mix(INK, PURPLE, t) } else { SOFT };
        ui.painter().text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(13.0),
            text,
        );
    }
    if enabled && response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn primary_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    button_base(ui, label, 140.0, enabled, true)
}

fn secondary_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    button_base(ui, label, 100.0, enabled, false)
}

/// Animated on/off switch.
fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(40.0, 22.0), Sense::click());
    if response.clicked() {
        *on = !*on;
    }
    let ctx = ui.ctx().clone();
    let t = ctx.animate_bool_with_time(response.id, *on, anim(&ctx) * 1.3);
    ui.painter()
        .rect_filled(rect, 11.0, mix(Color32::from_rgb(214, 210, 224), PURPLE, t));
    let knob_x = egui::lerp((rect.left() + 11.0)..=(rect.right() - 11.0), t);
    ui.painter()
        .circle_filled(Pos2::new(knob_x, rect.center().y), 8.0, Color32::WHITE);
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

// ---------------------------------------------------------------- dashboard

fn show_dashboard(ui: &mut egui::Ui, app: &mut AsterApp) {
    page_header(
        ui,
        "BUSINESS OVERVIEW",
        "Dashboard",
        "A clear view of your billing and stock activity.",
    );

    let metrics = [
        (
            "Today's sales",
            "₹—",
            "Invoice totals appear here",
            NavIcon::Sales,
        ),
        ("Stock alerts", "—", "Low stock items", NavIcon::Box),
        (
            "Receivables",
            "₹—",
            "Outstanding balances",
            NavIcon::Accounts,
        ),
        (
            "Expiring batches",
            "—",
            "Upcoming expiry dates",
            NavIcon::Stock,
        ),
    ];

    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        ui.columns(4, |columns| {
            for (index, (column, (title, value, note, icon))) in
                columns.iter_mut().zip(metrics).enumerate()
            {
                reveal(column, "aster_dash_enter", index, |ui| {
                    egui::Frame::new()
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .corner_radius(12.0)
                        .inner_margin(18.0)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.set_min_height(104.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(title).size(13.0).strong().color(MUTED));
                                ui.with_layout(egui::Layout::right_to_left(Align::Min), |ui| {
                                    icon_chip(ui, icon);
                                });
                            });
                            ui.add_space(4.0);
                            ui.label(RichText::new(value).size(28.0).strong().color(INK));
                            ui.add_space(2.0);
                            ui.label(RichText::new(note).size(12.5).color(SOFT));
                        });
                });
            }
        });
    });

    ui.add_space(16.0);

    let gap = 16.0;
    let total = ui.available_width();
    let left_w = ((total - gap) * 0.64).floor();
    let right_w = total - gap - left_w;
    let mut target: Option<WorkspacePage> = None;

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;

        reveal(ui, "aster_dash_enter", 4, |ui| {
            card(ui, left_w, 22.0, |ui| {
                ui.label(
                    RichText::new("Recent activity")
                        .size(16.5)
                        .strong()
                        .color(INK),
                );
                ui.add_space(3.0);
                ui.label(
                    RichText::new("Invoices and stock updates will appear here.")
                        .size(13.5)
                        .color(MUTED),
                );
                ui.add_space(14.0);
                ui.separator();
                ui.add_space(8.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width(), 190.0),
                    egui::Layout::top_down(Align::Center),
                    |ui| {
                        ui.add_space(52.0);
                        ui.label(
                            RichText::new("No activity yet")
                                .size(15.0)
                                .strong()
                                .color(MUTED),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Create an invoice or receive stock to get started.")
                                .size(13.0)
                                .color(SOFT),
                        );
                    },
                );
            });
        });

        reveal(ui, "aster_dash_enter", 5, |ui| {
            card(ui, right_w, 22.0, |ui| {
                ui.label(
                    RichText::new("Quick actions")
                        .size(16.5)
                        .strong()
                        .color(INK),
                );
                ui.add_space(3.0);
                ui.label(
                    RichText::new("Jump straight into common work.")
                        .size(13.5)
                        .color(MUTED),
                );
                ui.add_space(14.0);
                ui.spacing_mut().item_spacing.y = 8.0;
                for (label, page) in [
                    ("New sales invoice", WorkspacePage::Sales),
                    ("Record purchase", WorkspacePage::Purchase),
                    ("Add product", WorkspacePage::Products),
                    ("Check stock", WorkspacePage::Stock),
                    ("Open reports", WorkspacePage::Reports),
                ] {
                    let width = ui.available_width();
                    if button_base(ui, label, width, true, false).clicked() {
                        target = Some(page);
                    }
                }
            });
        });
    });

    if let Some(page) = target {
        app.open_page(page);
    }
}

// --------------------------------------------------------- user preferences

fn validate_name(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        Some("Display name is required.".into())
    } else if v.chars().count() < 2 {
        Some("Use at least 2 characters.".into())
    } else {
        None
    }
}

fn is_valid_email(value: &str) -> bool {
    let s = value.trim();
    if s.is_empty() || s.contains(char::is_whitespace) {
        return false;
    }
    let mut parts = s.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    !local.is_empty()
        && domain.len() >= 3
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

fn validate_email(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        Some("Email address is required.".into())
    } else if !is_valid_email(value) {
        Some("Enter a valid email address.".into())
    } else {
        None
    }
}

fn password_checks(pw: &str) -> [(&'static str, bool); 4] {
    [
        ("At least 10 characters", pw.chars().count() >= 10),
        (
            "Upper and lower case letters",
            pw.chars().any(|c| c.is_uppercase()) && pw.chars().any(|c| c.is_lowercase()),
        ),
        (
            "At least one number",
            pw.chars().any(|c| c.is_ascii_digit()),
        ),
        (
            "A symbol (recommended)",
            pw.chars()
                .any(|c| !c.is_alphanumeric() && !c.is_whitespace()),
        ),
    ]
}

fn password_requirements_met(pw: &str) -> bool {
    password_checks(pw).iter().take(3).all(|(_, ok)| *ok)
}

fn password_score(pw: &str) -> usize {
    let checks = password_checks(pw);
    let mut score = checks.iter().filter(|(_, ok)| *ok).count();
    if pw.chars().count() >= 14 {
        score += 1;
    }
    score.min(5)
}

fn validate_current_password(value: &str) -> Option<String> {
    if value.is_empty() {
        Some("Enter your current password.".into())
    } else {
        None
    }
}

fn validate_new_password(value: &str, current: &str) -> Option<String> {
    if value.is_empty() {
        Some("Enter a new password.".into())
    } else if !password_requirements_met(value) {
        Some("Password does not meet the requirements below.".into())
    } else if !current.is_empty() && value == current {
        Some("New password must differ from the current one.".into())
    } else {
        None
    }
}

fn validate_confirm_password(value: &str, new_password: &str) -> Option<String> {
    if value.is_empty() {
        Some("Confirm your new password.".into())
    } else if value != new_password {
        Some("Passwords do not match.".into())
    } else {
        None
    }
}

fn flag(ui: &egui::Ui, id: Id) -> bool {
    ui.data(|d| d.get_temp::<bool>(id).unwrap_or(false))
}

fn set_flag(ui: &egui::Ui, id: Id, value: bool) {
    ui.data_mut(|d| d.insert_temp(id, value));
}

struct Field<'a> {
    label: &'a str,
    hint: &'a str,
    required: bool,
    password: bool,
}

fn eye_button(ui: &mut egui::Ui, visible: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), anim(ui.ctx()));
    ui.painter().rect_filled(
        rect,
        8.0,
        Color32::from_rgb(243, 240, 249).gamma_multiply(t),
    );
    let c = rect.center();
    let color = mix(SOFT, PURPLE, if visible { 1.0 } else { t });
    let stroke = Stroke::new(1.5_f32, color);
    let pts = vec![
        c + Vec2::new(-7.0, 0.0),
        c + Vec2::new(-3.5, -4.0),
        c + Vec2::new(3.5, -4.0),
        c + Vec2::new(7.0, 0.0),
        c + Vec2::new(3.5, 4.0),
        c + Vec2::new(-3.5, 4.0),
    ];
    ui.painter().add(egui::Shape::closed_line(pts, stroke));
    ui.painter().circle_stroke(c, 2.2, stroke);
    if !visible {
        ui.painter()
            .line_segment([c + Vec2::new(-7.0, 7.0), c + Vec2::new(7.0, -7.0)], stroke);
    }
    ui.ctx().set_cursor_icon(if response.hovered() {
        egui::CursorIcon::PointingHand
    } else {
        egui::CursorIcon::Default
    });
    response.on_hover_text(if visible {
        "Hide passwords"
    } else {
        "Show passwords"
    })
}

/// Labelled text input with required marker, inline validation and animated error.
/// Returns true when the current value is invalid.
fn text_field(
    ui: &mut egui::Ui,
    field: Field,
    value: &mut String,
    submitted: bool,
    validate: &dyn Fn(&str) -> Option<String>,
) -> bool {
    let ctx = ui.ctx().clone();

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.label(RichText::new(field.label).size(12.0).strong().color(MUTED));
        if field.required {
            ui.label(RichText::new("*").size(12.0).strong().color(DANGER));
        } else {
            ui.label(RichText::new("(optional)").size(11.0).color(SOFT));
        }
    });
    ui.add_space(3.0);

    let error = if field.required || !value.is_empty() {
        validate(value)
    } else {
        None
    };
    let touched_id = Id::new(("aster_touched", field.label));
    let touched = flag(ui, touched_id);
    let show_error = error.is_some() && (submitted || touched);

    let response = if field.password {
        let show_id = Id::new("aster_show_passwords");
        let visible = flag(ui, show_id);
        let mut inner = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let width = (ui.available_width() - 40.0).max(60.0);
            inner = Some(
                ui.add(
                    egui::TextEdit::singleline(value)
                        .password(!visible)
                        .desired_width(width)
                        .margin(Margin::symmetric(10, 8))
                        .hint_text(field.hint),
                ),
            );
            if eye_button(ui, visible).clicked() {
                set_flag(ui, show_id, !visible);
            }
        });
        inner.expect("text edit response")
    } else {
        ui.add(
            egui::TextEdit::singleline(value)
                .desired_width(ui.available_width())
                .margin(Margin::symmetric(10, 8))
                .hint_text(field.hint),
        )
    };

    if response.lost_focus() {
        set_flag(ui, touched_id, true);
    }
    if show_error {
        ui.painter().rect_stroke(
            response.rect,
            ui.visuals().widgets.inactive.corner_radius,
            Stroke::new(1.5_f32, DANGER),
            egui::StrokeKind::Outside,
        );
    }

    let message_id = Id::new(("aster_err_msg", field.label));
    if let Some(message) = &error {
        ui.data_mut(|d| d.insert_temp(message_id, message.clone()));
    }
    let t = ctx.animate_bool_with_time(Id::new(("aster_err", field.label)), show_error, anim(&ctx));
    if t > 0.0 {
        let message: String = error
            .clone()
            .or_else(|| ui.data(|d| d.get_temp::<String>(message_id)))
            .unwrap_or_default();
        ui.vertical(|ui| {
            ui.set_opacity(t);
            ui.add_space(3.0 * t);
            ui.label(RichText::new(message).size(12.0).color(DANGER));
        });
    }
    ui.add_space(10.0);
    error.is_some()
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
        .corner_radius(12.0)
        .inner_margin(22.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(16.5).strong().color(INK));
            ui.add_space(2.0);
            ui.label(RichText::new(subtitle).size(12.5).color(MUTED));
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(12.0);
            content(ui);
        });
}

fn readonly_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.5).color(MUTED));
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).size(13.0).strong().color(INK));
        });
    });
    ui.add_space(8.0);
}

fn strength_meter(ui: &mut egui::Ui, password: &str) {
    let ctx = ui.ctx().clone();
    let score = password_score(password);
    let (label, color) = if password.is_empty() {
        ("Enter a password", SOFT)
    } else {
        match score {
            0..=2 => ("Weak", DANGER),
            3 => ("Fair", AMBER),
            4 => ("Good", Color32::from_rgb(98, 150, 60)),
            _ => ("Strong", SUCCESS),
        }
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new("Password strength").size(12.0).color(MUTED));
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(label).size(12.0).strong().color(color));
        });
    });
    ui.add_space(4.0);

    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, 3.0, Color32::from_rgb(234, 231, 240));
    let fraction = if password.is_empty() {
        0.0
    } else {
        score as f32 / 5.0
    };
    let filled = ctx.animate_value_with_time(
        Id::new("aster_strength_fill"),
        fraction * rect.width(),
        anim(&ctx) * 1.8,
    );
    let color_t = ctx.animate_value_with_time(
        Id::new("aster_strength_color_t"),
        score as f32,
        anim(&ctx) * 1.8,
    );
    let bar_color = if password.is_empty() {
        SOFT
    } else if color_t < 2.5 {
        mix(DANGER, AMBER, (color_t / 2.5).clamp(0.0, 1.0))
    } else {
        mix(AMBER, SUCCESS, ((color_t - 2.5) / 2.5).clamp(0.0, 1.0))
    };
    if filled > 0.5 {
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(filled, rect.height())),
            3.0,
            bar_color,
        );
    }
    ui.add_space(10.0);

    for (index, (text, ok)) in password_checks(password).iter().enumerate() {
        let t = ctx.animate_bool_with_time(Id::new(("aster_pw_check", index)), *ok, anim(&ctx));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
            let c = r.center();
            ui.painter().circle_stroke(
                c,
                6.5,
                Stroke::new(1.3_f32, mix(Color32::from_rgb(200, 195, 212), SUCCESS, t)),
            );
            if t > 0.0 {
                ui.painter().circle_filled(c, 6.5 * t, SUCCESS);
                let check = Stroke::new(1.6_f32, Color32::WHITE.gamma_multiply(t));
                ui.painter()
                    .line_segment([c + Vec2::new(-2.6, 0.2), c + Vec2::new(-0.6, 2.2)], check);
                ui.painter()
                    .line_segment([c + Vec2::new(-0.6, 2.2), c + Vec2::new(3.0, -2.0)], check);
            }
            ui.label(
                RichText::new(*text)
                    .size(12.5)
                    .color(mix(SOFT, SUCCESS_TEXT, t)),
            );
        });
        ui.add_space(1.0);
    }
}

fn show_user_preferences(ui: &mut egui::Ui, app: &mut AsterApp) {
    page_header(
        ui,
        "ACCOUNT",
        "User preferences",
        "Manage your profile, workspace preferences and sign-in security. Fields marked * are required.",
    );

    let snapshot_id = Id::new("aster_profile_snapshot");
    let snapshot: (String, String) = ui.data_mut(|d| {
        d.get_temp_mut_or_insert_with(snapshot_id, || {
            (app.display_name.clone(), app.profile_email.clone())
        })
        .clone()
    });

    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        ui.columns(2, |columns| {
            // ------------------------------------------------------ left column
            {
                let left = &mut columns[0];

                reveal(left, "aster_prefs_enter", 0, |ui| {
                    section_card(ui, "Account", "Your signed-in access", |ui| {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(56.0), Sense::hover());
                            ui.painter().circle_filled(r.center(), 28.0, PURPLE_TINT);
                            ui.painter().text(
                                r.center(),
                                egui::Align2::CENTER_CENTER,
                                initials_of(app),
                                FontId::proportional(20.0),
                                PURPLE,
                            );
                            ui.add_space(6.0);
                            ui.vertical(|ui| {
                                ui.add_space(4.0);
                                let name = if app.display_name.trim().is_empty() {
                                    app.current_user.clone()
                                } else {
                                    app.display_name.clone()
                                };
                                ui.label(RichText::new(name).size(16.0).strong().color(INK));
                                ui.label(
                                    RichText::new(format!("@{}", app.current_user))
                                        .size(12.5)
                                        .color(MUTED),
                                );
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    pill(ui, "Administrator", PURPLE, PURPLE_TINT);
                                    pill(ui, "Active", SUCCESS_TEXT, SUCCESS_TINT);
                                });
                            });
                        });
                        ui.add_space(14.0);
                        ui.separator();
                        ui.add_space(10.0);
                        readonly_row(
                            ui,
                            "Email",
                            if app.profile_email.trim().is_empty() {
                                "Not set"
                            } else {
                                app.profile_email.trim()
                            },
                        );
                        readonly_row(
                            ui,
                            "Financial year",
                            &format!("{}–{}", app.selected_year, app.selected_year + 1),
                        );
                        readonly_row(ui, "Session date", &app.date_text);
                    });
                });
                left.add_space(16.0);

                reveal(left, "aster_prefs_enter", 1, |ui| {
                    section_card(
                        ui,
                        "Profile",
                        "Required details shown across the workspace",
                        |ui| {
                            let submitted_id = Id::new("aster_profile_submitted");
                            let submitted = flag(ui, submitted_id);

                            text_field(
                                ui,
                                Field {
                                    label: "Display name",
                                    hint: "Your full name",
                                    required: true,
                                    password: false,
                                },
                                &mut app.display_name,
                                submitted,
                                &validate_name,
                            );
                            text_field(
                                ui,
                                Field {
                                    label: "Email address",
                                    hint: "name@company.com",
                                    required: true,
                                    password: false,
                                },
                                &mut app.profile_email,
                                submitted,
                                &validate_email,
                            );

                            let dirty =
                                app.display_name != snapshot.0 || app.profile_email != snapshot.1;
                            let dirty_t = ui.ctx().animate_bool_with_time(
                                Id::new("aster_dirty_badge"),
                                dirty,
                                anim(ui.ctx()),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 10.0;
                                let save = primary_button(ui, "Save changes", dirty);
                                let discard = secondary_button(ui, "Discard", dirty);
                                if dirty_t > 0.0 {
                                    let (r, _) = ui
                                        .allocate_exact_size(Vec2::new(8.0, 34.0), Sense::hover());
                                    ui.painter().circle_filled(
                                        r.center(),
                                        3.5,
                                        AMBER.gamma_multiply(dirty_t),
                                    );
                                    ui.label(
                                        RichText::new("Unsaved changes")
                                            .size(12.0)
                                            .color(AMBER.gamma_multiply(dirty_t)),
                                    );
                                }

                                if save.clicked() {
                                    set_flag(ui, submitted_id, true);
                                    if validate_name(&app.display_name).is_some()
                                        || validate_email(&app.profile_email).is_some()
                                    {
                                        app.profile_message =
                                            "Please correct the highlighted fields.".into();
                                    } else {
                                        app.display_name = app.display_name.trim().to_string();
                                        app.profile_email = app.profile_email.trim().to_string();
                                        ui.data_mut(|d| {
                                            d.insert_temp(
                                                snapshot_id,
                                                (
                                                    app.display_name.clone(),
                                                    app.profile_email.clone(),
                                                ),
                                            )
                                        });
                                        set_flag(ui, submitted_id, false);
                                        app.profile_message =
                                            "Profile saved for this session.".into();
                                    }
                                }
                                if discard.clicked() {
                                    app.display_name = snapshot.0.clone();
                                    app.profile_email = snapshot.1.clone();
                                    set_flag(ui, submitted_id, false);
                                }
                            });
                        },
                    );
                });
            }

            // ----------------------------------------------------- right column
            {
                let right = &mut columns[1];
                reveal(right, "aster_prefs_enter", 2, |ui| {
                    section_card(
                        ui,
                        "Security",
                        "Change the password for this account. All fields are required.",
                        |ui| {
                            let submitted_id = Id::new("aster_security_submitted");
                            let submitted = flag(ui, submitted_id);
                            let current_copy = app.current_password.clone();

                            text_field(
                                ui,
                                Field {
                                    label: "Current password",
                                    hint: "Enter your current password",
                                    required: true,
                                    password: true,
                                },
                                &mut app.current_password,
                                submitted,
                                &validate_current_password,
                            );
                            text_field(
                                ui,
                                Field {
                                    label: "New password",
                                    hint: "At least 10 characters",
                                    required: true,
                                    password: true,
                                },
                                &mut app.new_password,
                                submitted,
                                &|v: &str| validate_new_password(v, &current_copy),
                            );
                            let new_copy = app.new_password.clone();
                            strength_meter(ui, &app.new_password);
                            ui.add_space(8.0);
                            text_field(
                                ui,
                                Field {
                                    label: "Confirm password",
                                    hint: "Re-enter the new password",
                                    required: true,
                                    password: true,
                                },
                                &mut app.confirm_new_password,
                                submitted,
                                &|v: &str| validate_confirm_password(v, &new_copy),
                            );

                            ui.add_space(2.0);
                            if primary_button(ui, "Update password", true).clicked() {
                                set_flag(ui, submitted_id, true);
                                let invalid = validate_current_password(&app.current_password)
                                    .is_some()
                                    || validate_new_password(
                                        &app.new_password,
                                        &app.current_password,
                                    )
                                    .is_some()
                                    || validate_confirm_password(
                                        &app.confirm_new_password,
                                        &app.new_password,
                                    )
                                    .is_some();
                                if invalid {
                                    app.profile_message =
                                        "Please correct the highlighted fields.".into();
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
                                            set_flag(ui, submitted_id, false);
                                            for label in [
                                                "Current password",
                                                "New password",
                                                "Confirm password",
                                            ] {
                                                set_flag(
                                                    ui,
                                                    Id::new(("aster_touched", label)),
                                                    false,
                                                );
                                            }
                                            app.profile_message =
                                                "Password updated successfully.".into();
                                        }
                                        Err(error) => app.profile_message = error,
                                    }
                                } else {
                                    app.profile_message = "Database is unavailable.".into();
                                }
                            }
                        },
                    );
                });
            }
        });
    });

    ui.add_space(16.0);
    reveal(ui, "aster_prefs_enter", 3, |ui| {
        section_card(
            ui,
            "Workspace",
            "Personal display choices. They apply instantly while this session is open.",
            |ui| {
                ui.columns(3, |columns| {
                    toggle_row(
                        &mut columns[0],
                        "Compact spacing",
                        "Tighter margins around pages",
                        &mut app.compact_density,
                    );
                    toggle_row(
                        &mut columns[1],
                        "Navigation tooltips",
                        "Names on hover when collapsed",
                        &mut app.show_tooltips,
                    );
                    toggle_row(
                        &mut columns[2],
                        "Reduce motion",
                        "Turn off interface animation",
                        &mut app.reduced_motion,
                    );
                });
            },
        );
    });
}

fn toggle_row(ui: &mut egui::Ui, title: &str, description: &str, value: &mut bool) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(title).size(13.5).strong().color(INK));
            ui.label(RichText::new(description).size(12.0).color(MUTED));
        });
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            toggle_switch(ui, value);
        });
    });
}

/// Bottom-right notification that slides in, then fades out by itself.
fn show_toast(ctx: &egui::Context, app: &mut AsterApp) {
    let state_id = Id::new("aster_toast_state");
    if app.profile_message.is_empty() {
        ctx.data_mut(|d| d.remove::<(String, f64)>(state_id));
        return;
    }
    let now = ctx.input(|i| i.time);
    let mut state: (String, f64) = ctx
        .data_mut(|d| d.get_temp::<(String, f64)>(state_id))
        .unwrap_or_default();
    if state.0 != app.profile_message {
        state = (app.profile_message.clone(), now);
        ctx.data_mut(|d| d.insert_temp(state_id, state.clone()));
    }
    let elapsed = (now - state.1) as f32;
    let duration = 4.5_f32;
    if elapsed > duration {
        app.profile_message.clear();
        return;
    }
    let fade_in = (elapsed / 0.25).clamp(0.0, 1.0);
    let fade_out = ((duration - elapsed) / 0.4).clamp(0.0, 1.0);
    let t = fade_in.min(fade_out);
    let eased = 1.0 - (1.0 - fade_in).powi(3);
    if anim(ctx) == 0.0 {
        ctx.request_repaint_after(Duration::from_millis(250));
    } else {
        ctx.request_repaint_after(Duration::from_millis(16));
    }

    let ok = message_is_ok(&app.profile_message);
    let (fill, border, text) = if ok {
        (
            Color32::from_rgb(240, 249, 244),
            Color32::from_rgb(190, 225, 203),
            SUCCESS_TEXT,
        )
    } else {
        (
            Color32::from_rgb(253, 242, 242),
            Color32::from_rgb(235, 196, 196),
            DANGER,
        )
    };
    let slide = (1.0 - eased) * 14.0;

    egui::Area::new(Id::new("aster_toast"))
        .order(egui::Order::Foreground)
        .anchor(
            egui::Align2::RIGHT_BOTTOM,
            Vec2::new(-28.0, -(STATUS_HEIGHT + 22.0) + slide),
        )
        .interactable(false)
        .show(ctx, |ui| {
            ui.set_opacity(t);
            egui::Frame::new()
                .fill(fill)
                .stroke(Stroke::new(1.0_f32, border))
                .corner_radius(10.0)
                .inner_margin(Margin::symmetric(16, 12))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        let (dot, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                        ui.painter().circle_filled(dot.center(), 4.0, text);
                        ui.label(RichText::new(&app.profile_message).size(13.0).color(text));
                    });
                });
        });
}

// -------------------------------------------------------------- placeholder

fn show_module_intro(ui: &mut egui::Ui, page: WorkspacePage) {
    page_header(ui, "WORKSPACE", page.title(), "");
    let width = ui.available_width();
    card(ui, width, 28.0, |ui| {
        ui.set_min_height(240.0);
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(60.0), Sense::hover());
            ui.painter().circle_filled(rect.center(), 30.0, PURPLE_TINT);
            draw_icon(ui.painter(), rect.center(), page.icon(), PURPLE);
            ui.add_space(14.0);
            ui.label(
                RichText::new(format!("{} workspace", page.title()))
                    .size(19.0)
                    .strong()
                    .color(INK),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new("This screen is being prepared for a later implementation phase.")
                    .size(13.5)
                    .color(MUTED),
            );
        });
    });
}
