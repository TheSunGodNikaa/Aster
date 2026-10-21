use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Id, Key, Margin, Pos2, Rect, RichText, Sense,
    Stroke, Vec2,
};

use crate::{
    app::{AsterApp, Stage},
    date::local_date,
};

const INK: Color32 = Color32::from_rgb(43, 38, 53);
const MUTED: Color32 = Color32::from_rgb(116, 109, 128);
const SOFT: Color32 = Color32::from_rgb(132, 123, 147);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const PURPLE_DEEP: Color32 = Color32::from_rgb(74, 50, 120);
const PURPLE_TINT: Color32 = Color32::from_rgb(241, 237, 248);
const BORDER: Color32 = Color32::from_rgb(234, 231, 240);
const PANE_FILL: Color32 = Color32::from_rgb(247, 245, 251);
const DANGER: Color32 = Color32::from_rgb(171, 54, 62);
const DISABLED: Color32 = Color32::from_rgb(205, 200, 216);

const CARD_SIZE: Vec2 = Vec2::new(800.0, 500.0);
const LEFT_PANE_WIDTH: f32 = 300.0;

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const WEEKDAYS_LONG: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

// ---------------------------------------------------------------- date maths

type Ymd = (i32, i32, i32);

fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
    }
}

fn days_from_civil(date: Ymd) -> i64 {
    let (year, month, day) = date;
    let y = year as i64 - if month <= 2 { 1 } else { 0 };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let mp = (month as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> Ymd {
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    ((y + if m <= 2 { 1 } else { 0 }) as i32, m as i32, d as i32)
}

fn add_days(date: Ymd, delta: i64) -> Ymd {
    civil_from_days(days_from_civil(date) + delta)
}

/// 0 = Monday … 6 = Sunday.
fn weekday_mon0(date: Ymd) -> usize {
    (days_from_civil(date) + 3).rem_euclid(7) as usize
}

fn fy_of(date: Ymd) -> i32 {
    if date.1 >= 4 { date.0 } else { date.0 - 1 }
}

fn in_fy(date: Ymd, fy: i32) -> bool {
    (fy, 4, 1) <= date && date <= (fy + 1, 3, 31)
}

fn parse_ymd(text: &str) -> Option<Ymd> {
    let cleaned = text.trim().replace(['-', '.'], "/");
    let mut parts = cleaned.split('/');
    let day = parts.next()?.trim().parse::<i32>().ok()?;
    let month = parts.next()?.trim().parse::<i32>().ok()?;
    let year = parts.next()?.trim().parse::<i32>().ok()?;
    if parts.next().is_some() || !(1900..=2200).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    if day < 1 || day > days_in_month(year, month) {
        return None;
    }
    Some((year, month, day))
}

fn format_ymd(date: Ymd) -> String {
    format!("{:02}/{:02}/{}", date.2, date.1, date.0)
}

fn long_date(date: Ymd) -> String {
    format!(
        "{}, {} {} {}",
        WEEKDAYS_LONG[weekday_mon0(date)],
        date.2,
        MONTHS[(date.1 - 1) as usize],
        date.0
    )
}

fn today() -> Ymd {
    let (year, month, day) = local_date();
    (year as i32, month as i32, day as i32)
}

fn default_date_for_fy(fy: i32, today: Ymd) -> Ymd {
    if in_fy(today, fy) {
        today
    } else if fy < fy_of(today) {
        (fy + 1, 3, 31)
    } else {
        (fy, 4, 1)
    }
}

// ----------------------------------------------------------------- UI state

#[derive(Clone)]
struct CalState {
    fy: i32,
    view_y: i32,
    view_m: i32,
    months_mode: bool,
    changed_at: f64,
}

impl CalState {
    fn set_view_index(&mut self, fy: i32, index: i32, now: f64) {
        let total = fy * 12 + 3 + index.clamp(0, 11);
        let (y, m) = (total / 12, total % 12 + 1);
        if (y, m) != (self.view_y, self.view_m) {
            self.view_y = y;
            self.view_m = m;
            self.changed_at = now;
        }
    }

    fn view_index(&self, fy: i32) -> i32 {
        (self.view_y * 12 + self.view_m) - (fy * 12 + 4)
    }

    fn show_date(&mut self, fy: i32, date: Ymd, now: f64) {
        let index = (date.0 * 12 + date.1) - (fy * 12 + 4);
        self.set_view_index(fy, index, now);
    }
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

fn anim(ctx: &egui::Context) -> f32 {
    ctx.data(|d| d.get_temp::<f32>(Id::new("aster_anim_time")))
        .unwrap_or(0.14)
}

// ------------------------------------------------------------------- screen

impl AsterApp {
    pub(crate) fn show_year_and_date(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let today = today();
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

        let state_id = Id::new("aster_calendar_state");
        let mut cal: CalState = ctx.data(|d| d.get_temp(state_id)).unwrap_or(CalState {
            fy: i32::MIN,
            view_y: 0,
            view_m: 4,
            months_mode: false,
            changed_at: -10.0,
        });

        // Keep the working date inside the selected financial year.
        if cal.fy != self.selected_year {
            let fy = self.selected_year;
            if !parse_ymd(&self.date_text).map_or(false, |d| in_fy(d, fy)) {
                self.date_text = format_ymd(default_date_for_fy(fy, today));
            }
            cal.fy = fy;
            cal.months_mode = false;
            if let Some(date) = parse_ymd(&self.date_text) {
                cal.view_y = date.0;
                cal.view_m = date.1;
            }
            cal.changed_at = -10.0;
        }

        let mut go = false;

        // Keyboard: arrows move the date, Enter continues.
        if !ctx.wants_keyboard_input() {
            let fy = self.selected_year;
            let delta: i64 = ctx.input(|i| {
                if i.key_pressed(Key::ArrowLeft) {
                    -1
                } else if i.key_pressed(Key::ArrowRight) {
                    1
                } else if i.key_pressed(Key::ArrowUp) {
                    -7
                } else if i.key_pressed(Key::ArrowDown) {
                    7
                } else {
                    0
                }
            });
            if delta != 0 {
                if let Some(selected) = parse_ymd(&self.date_text).filter(|d| in_fy(*d, fy)) {
                    let next = add_days(selected, delta);
                    if in_fy(next, fy) {
                        self.date_text = format_ymd(next);
                        self.message.clear();
                        cal.show_date(fy, next, now);
                    }
                }
            }
            if ctx.input(|i| i.key_pressed(Key::Enter)) {
                go = true;
            }
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(248, 247, 251))
                    .inner_margin(0.0),
            )
            .show(ctx, |ui| {
                super::paint_wallpaper(ui, self);

                let card = Rect::from_center_size(ui.max_rect().center(), CARD_SIZE);
                for step in 1..=4 {
                    let grow = step as f32 * 3.0;
                    ui.painter().rect_filled(
                        card.expand(grow).translate(Vec2::new(0.0, 4.0)),
                        18.0 + grow,
                        Color32::from_black_alpha(5),
                    );
                }
                ui.painter().rect_filled(
                    card,
                    18.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 248),
                );

                let left = Rect::from_min_max(
                    card.min,
                    Pos2::new(card.left() + LEFT_PANE_WIDTH, card.bottom()),
                );
                let right = Rect::from_min_max(
                    Pos2::new(card.left() + LEFT_PANE_WIDTH, card.top()),
                    card.max,
                );
                ui.painter().rect_filled(
                    left,
                    CornerRadius {
                        nw: 18,
                        sw: 18,
                        ne: 0,
                        se: 0,
                    },
                    PANE_FILL,
                );
                ui.painter()
                    .vline(left.right(), left.y_range(), Stroke::new(1.0_f32, BORDER));
                ui.painter().rect_stroke(
                    card,
                    18.0,
                    Stroke::new(1.0_f32, BORDER),
                    egui::StrokeKind::Inside,
                );

                // ---------------------------------------------------- left pane
                let mut left_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(left.shrink2(Vec2::new(28.0, 28.0)))
                        .layout(egui::Layout::top_down(Align::Min)),
                );
                self.year_left_pane(&mut left_ui, &mut cal, today, now, &mut go);

                // --------------------------------------------------- right pane
                let mut right_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(Rect::from_min_max(
                            Pos2::new(right.left() + 36.0, right.top() + 30.0),
                            Pos2::new(right.right() - 36.0, right.bottom() - 30.0),
                        ))
                        .layout(egui::Layout::top_down(Align::Min)),
                );
                self.year_calendar(&mut right_ui, &mut cal, today, now, &mut go);

                // ------------------------------------------------------ footer
                let fy = self.selected_year;
                let selected = parse_ymd(&self.date_text).filter(|d| in_fy(*d, fy));
                let footer = Rect::from_min_max(
                    Pos2::new(right.left() + 36.0, right.bottom() - 70.0),
                    Pos2::new(right.right() - 36.0, right.bottom() - 30.0),
                );
                let summary = Rect::from_min_max(
                    Pos2::new(footer.left(), footer.top() - 58.0),
                    Pos2::new(footer.right(), footer.top() - 10.0),
                );
                ui.painter().rect_filled(summary, 12.0, PANE_FILL);
                ui.painter().rect_stroke(
                    summary,
                    12.0,
                    Stroke::new(1.0_f32, BORDER),
                    egui::StrokeKind::Inside,
                );
                // calendar glyph
                let glyph = Rect::from_center_size(
                    Pos2::new(summary.left() + 28.0, summary.center().y),
                    Vec2::new(20.0, 20.0),
                );
                ui.painter().rect_stroke(
                    glyph,
                    4.0,
                    Stroke::new(1.5_f32, PURPLE),
                    egui::StrokeKind::Inside,
                );
                ui.painter().hline(
                    glyph.x_range(),
                    glyph.top() + 6.0,
                    Stroke::new(1.5_f32, PURPLE),
                );
                match selected {
                    Some(date) => {
                        ui.painter().text(
                            Pos2::new(summary.left() + 52.0, summary.center().y - 9.0),
                            egui::Align2::LEFT_CENTER,
                            long_date(date),
                            FontId::proportional(15.0),
                            INK,
                        );
                        ui.painter().text(
                            Pos2::new(summary.left() + 52.0, summary.center().y + 10.0),
                            egui::Align2::LEFT_CENTER,
                            format!("Financial year {} – {}", fy, fy + 1),
                            FontId::proportional(12.0),
                            MUTED,
                        );
                    }
                    None => {
                        ui.painter().text(
                            Pos2::new(summary.left() + 52.0, summary.center().y),
                            egui::Align2::LEFT_CENTER,
                            "Pick a date inside the financial year",
                            FontId::proportional(13.5),
                            MUTED,
                        );
                    }
                }

                // message (left) + continue button (right)
                if !self.message.is_empty() {
                    let galley = ui.painter().layout(
                        self.message.clone(),
                        FontId::proportional(12.5),
                        DANGER,
                        footer.width() - 200.0,
                    );
                    let pos = Pos2::new(footer.left(), footer.center().y - galley.size().y / 2.0);
                    ui.painter().galley(pos, galley, DANGER);
                }
                let button = Rect::from_min_size(
                    Pos2::new(footer.right() - 184.0, footer.top()),
                    Vec2::new(184.0, 40.0),
                );
                if primary_button_at(ui, button, "Continue to sign in", selected.is_some())
                    .clicked()
                {
                    go = true;
                }

                if go {
                    self.try_continue(ctx);
                }
                super::show_background_error(ui, self);
            });

        ctx.data_mut(|d| d.insert_temp(state_id, cal));
    }

    fn try_continue(&mut self, ctx: &egui::Context) {
        match parse_ymd(&self.date_text) {
            Some(date) if in_fy(date, self.selected_year) => {
                self.date_text = format_ymd(date);
                let has_users = self.db.as_ref().map(|db| db.has_users());
                match has_users {
                    Some(Ok(true)) => self.set_stage(Stage::Login, ctx),
                    Some(Ok(false)) => self.set_stage(Stage::FirstRunSetup, ctx),
                    Some(Err(error)) => self.set_stage(Stage::Error(error), ctx),
                    None => self.set_stage(Stage::Error("Database is unavailable.".into()), ctx),
                }
                self.message.clear();
            }
            Some(_) => {
                self.message = "Choose a date within the selected financial year.".into();
            }
            None => {
                self.message = "Enter a valid date as DD/MM/YYYY.".into();
            }
        }
    }

    // ---------------------------------------------------------------- left pane

    fn year_left_pane(
        &mut self,
        ui: &mut egui::Ui,
        cal: &mut CalState,
        today: Ymd,
        now: f64,
        go: &mut bool,
    ) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(RichText::new("ASTER").size(20.0).strong().color(PURPLE));
            ui.label(RichText::new("BILLING").size(10.5).strong().color(SOFT));
        });
        ui.add_space(18.0);
        ui.label(
            RichText::new("Set your working period")
                .size(22.0)
                .strong()
                .color(INK),
        );
        ui.add_space(3.0);
        ui.label(
            RichText::new("Choose the financial year and the date you are billing for.")
                .size(13.0)
                .color(MUTED),
        );
        ui.add_space(18.0);

        // ---- financial year list
        ui.label(
            RichText::new("FINANCIAL YEAR")
                .size(10.5)
                .strong()
                .color(SOFT),
        );
        ui.add_space(6.0);
        let mut years = self.financial_years.clone();
        years.sort_unstable_by(|a, b| b.cmp(a));
        let current_fy = fy_of(today);
        let mut picked = None;
        egui::ScrollArea::vertical()
            .id_salt("aster_fy_list")
            .max_height(150.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                for year in &years {
                    if fy_row(ui, *year, *year == self.selected_year, *year == current_fy).clicked()
                    {
                        picked = Some(*year);
                    }
                }
            });
        if let Some(year) = picked {
            if year != self.selected_year {
                self.selected_year = year;
                self.message.clear();
            }
        }

        ui.add_space(4.0);
        if link_button(ui, "+ Add next financial year", PURPLE).clicked() {
            if let Some(db) = &self.db {
                let next_year = self
                    .financial_years
                    .iter()
                    .max()
                    .copied()
                    .unwrap_or(self.selected_year)
                    + 1;
                match db.create_financial_year(next_year) {
                    Ok(()) => {
                        self.financial_years.push(next_year);
                        self.selected_year = next_year;
                        self.message.clear();
                    }
                    Err(error) => self.message = error,
                }
            }
        }

        // ---- working date
        ui.add_space(12.0);
        ui.label(
            RichText::new("WORKING DATE")
                .size(10.5)
                .strong()
                .color(SOFT),
        );
        ui.add_space(6.0);

        let fy = self.selected_year;
        let parsed = parse_ymd(&self.date_text);
        let problem = if self.date_text.trim().is_empty() {
            None
        } else {
            match parsed {
                None => Some("Use the format DD/MM/YYYY."),
                Some(date) if !in_fy(date, fy) => Some("This date is outside the financial year."),
                Some(_) => None,
            }
        };

        let mut today_clicked = false;
        let mut edit_response = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let width = (ui.available_width() - 84.0).max(80.0);
            edit_response = Some(
                ui.add(
                    egui::TextEdit::singleline(&mut self.date_text)
                        .desired_width(width)
                        .margin(Margin::symmetric(10, 8))
                        .char_limit(10)
                        .hint_text("DD/MM/YYYY"),
                ),
            );
            today_clicked = small_button(ui, "Today", 76.0, 34.0).clicked();
        });
        if let Some(response) = &edit_response {
            if problem.is_some() && !response.has_focus() {
                ui.painter().rect_stroke(
                    response.rect,
                    ui.visuals().widgets.inactive.corner_radius,
                    Stroke::new(1.4_f32, DANGER),
                    egui::StrokeKind::Outside,
                );
            }
            if response.changed() {
                self.message.clear();
                if let Some(date) = parse_ymd(&self.date_text).filter(|d| in_fy(*d, fy)) {
                    cal.show_date(fy, date, now);
                }
            }
            if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                *go = true;
            }
        }
        if today_clicked {
            if in_fy(today, fy) {
                self.date_text = format_ymd(today);
                self.message.clear();
                cal.show_date(fy, today, now);
            } else if self.financial_years.contains(&fy_of(today)) {
                self.selected_year = fy_of(today);
                self.date_text = format_ymd(today);
                self.message.clear();
            } else {
                self.message = "Today's financial year has not been created yet.".into();
            }
        }
        ui.add_space(5.0);
        match problem {
            Some(text) => {
                ui.label(RichText::new(text).size(12.0).color(DANGER));
            }
            None => {
                ui.label(
                    RichText::new("Financial year runs from 1 April to 31 March.")
                        .size(12.0)
                        .color(SOFT),
                );
            }
        }
    }

    // ----------------------------------------------------------------- calendar

    fn year_calendar(
        &mut self,
        ui: &mut egui::Ui,
        cal: &mut CalState,
        today: Ymd,
        now: f64,
        go: &mut bool,
    ) {
        let ctx = ui.ctx().clone();
        let fy = self.selected_year;
        let selected = parse_ymd(&self.date_text).filter(|d| in_fy(*d, fy));

        // ---- header: month title + arrows
        let (header, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), Sense::hover());
        let nav = 34.0;
        let prev_rect = Rect::from_center_size(
            Pos2::new(header.right() - nav * 1.5 - 4.0, header.center().y),
            Vec2::splat(nav),
        );
        let next_rect = Rect::from_center_size(
            Pos2::new(header.right() - nav * 0.5, header.center().y),
            Vec2::splat(nav),
        );
        let title_rect = Rect::from_min_max(
            header.min,
            Pos2::new(prev_rect.left() - 8.0, header.bottom()),
        );

        let index = cal.view_index(fy);
        let title = if cal.months_mode {
            format!("{} – {}", fy, fy + 1)
        } else {
            format!("{} {}", MONTHS[(cal.view_m - 1) as usize], cal.view_y)
        };
        let title_response = ui
            .interact(title_rect, Id::new("aster_cal_title"), Sense::click())
            .on_hover_text(if cal.months_mode {
                "Back to days"
            } else {
                "Pick a month"
            });
        let title_hover =
            ctx.animate_bool_with_time(title_response.id, title_response.hovered(), anim(&ctx));
        let title_galley = ui.painter().layout_no_wrap(
            title,
            FontId::proportional(21.0),
            mix(INK, PURPLE, title_hover),
        );
        let title_width = title_galley.size().x;
        ui.painter().galley(
            Pos2::new(
                header.left(),
                header.center().y - title_galley.size().y / 2.0,
            ),
            title_galley,
            INK,
        );
        // dropdown chevron
        let chevron_center = Pos2::new(header.left() + title_width + 14.0, header.center().y + 1.0);
        let chevron_dir = if cal.months_mode { -1.0 } else { 1.0 };
        let chevron_stroke = Stroke::new(1.6_f32, mix(SOFT, PURPLE, title_hover));
        ui.painter().line_segment(
            [
                chevron_center + Vec2::new(-4.0, -2.0 * chevron_dir),
                chevron_center + Vec2::new(0.0, 2.0 * chevron_dir),
            ],
            chevron_stroke,
        );
        ui.painter().line_segment(
            [
                chevron_center + Vec2::new(0.0, 2.0 * chevron_dir),
                chevron_center + Vec2::new(4.0, -2.0 * chevron_dir),
            ],
            chevron_stroke,
        );
        if title_response.clicked() {
            cal.months_mode = !cal.months_mode;
            cal.changed_at = now;
        }

        if chevron_button(ui, prev_rect, -1.0, !cal.months_mode && index > 0, "prev").clicked() {
            cal.set_view_index(fy, index - 1, now);
        }
        if chevron_button(ui, next_rect, 1.0, !cal.months_mode && index < 11, "next").clicked() {
            cal.set_view_index(fy, index + 1, now);
        }
        // Scroll wheel flips months while hovering the calendar.
        let wheel = ctx.input(|i| i.raw_scroll_delta.y);
        if !cal.months_mode && wheel.abs() > 4.0 && ui.rect_contains_pointer(ui.max_rect()) {
            let step = if wheel < 0.0 { 1 } else { -1 };
            if ctx.input(|i| i.time) - cal.changed_at > 0.18 {
                cal.set_view_index(fy, index + step, now);
            }
        }

        ui.add_space(8.0);

        // ---- body (fades in whenever the view changes)
        let fade = if anim(&ctx) == 0.0 {
            1.0
        } else {
            (((now - cal.changed_at) / 0.2) as f32).clamp(0.0, 1.0)
        };
        if fade < 1.0 {
            ctx.request_repaint();
        }
        let eased = 1.0 - (1.0 - fade).powi(3);

        ui.scope(|ui| {
            ui.set_opacity(0.3 + 0.7 * eased);
            let width = ui.available_width();

            if cal.months_mode {
                let (area, _) =
                    ui.allocate_exact_size(Vec2::new(width, 26.0 + 240.0), Sense::hover());
                let gap = 10.0;
                let cell_w = (width - gap * 2.0) / 3.0;
                let cell_h = (area.height() - gap * 3.0) / 4.0;
                for i in 0..12_i32 {
                    let month = (3 + i) % 12 + 1;
                    let year = if i >= 9 { fy + 1 } else { fy };
                    let col = (i % 3) as f32;
                    let row = (i / 3) as f32;
                    let rect = Rect::from_min_size(
                        area.min + Vec2::new(col * (cell_w + gap), row * (cell_h + gap)),
                        Vec2::new(cell_w, cell_h),
                    );
                    let id = Id::new(("aster_cal_month", year, month));
                    let response = ui.interact(rect, id, Sense::click());
                    let hover = ctx.animate_bool_with_time(id, response.hovered(), anim(&ctx));
                    let is_view = (year, month) == (cal.view_y, cal.view_m);
                    let has_date = selected.map_or(false, |d| (d.0, d.1) == (year, month));
                    let is_today_month = (today.0, today.1) == (year, month);
                    let fill = if is_view {
                        PURPLE
                    } else {
                        mix(PANE_FILL, PURPLE_TINT, hover)
                    };
                    ui.painter().rect_filled(rect, 12.0, fill);
                    if is_today_month && !is_view {
                        ui.painter().rect_stroke(
                            rect,
                            12.0,
                            Stroke::new(1.4_f32, PURPLE),
                            egui::StrokeKind::Inside,
                        );
                    }
                    ui.painter().text(
                        rect.center() + Vec2::new(0.0, -4.0),
                        egui::Align2::CENTER_CENTER,
                        MONTHS_SHORT[(month - 1) as usize],
                        FontId::proportional(16.0),
                        if is_view { Color32::WHITE } else { INK },
                    );
                    ui.painter().text(
                        rect.center() + Vec2::new(0.0, 14.0),
                        egui::Align2::CENTER_CENTER,
                        year.to_string(),
                        FontId::proportional(11.0),
                        if is_view {
                            Color32::from_rgb(222, 212, 240)
                        } else {
                            SOFT
                        },
                    );
                    if has_date && !is_view {
                        ui.painter().circle_filled(
                            rect.right_top() + Vec2::new(-12.0, 12.0),
                            3.0,
                            PURPLE,
                        );
                    }
                    if response.hovered() {
                        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if response.clicked() {
                        cal.view_y = year;
                        cal.view_m = month;
                        cal.months_mode = false;
                        cal.changed_at = now;
                    }
                }
                return;
            }

            // weekday header
            let cell_w = (width / 7.0).floor();
            let (weekday_row, _) =
                ui.allocate_exact_size(Vec2::new(cell_w * 7.0, 26.0), Sense::hover());
            for (i, name) in WEEKDAYS.iter().enumerate() {
                ui.painter().text(
                    Pos2::new(
                        weekday_row.left() + cell_w * (i as f32 + 0.5),
                        weekday_row.center().y,
                    ),
                    egui::Align2::CENTER_CENTER,
                    *name,
                    FontId::proportional(11.5),
                    if i == 6 {
                        Color32::from_rgb(200, 110, 118)
                    } else {
                        SOFT
                    },
                );
            }

            // day grid (always six rows so the card never changes height)
            let cell_h = 40.0;
            let (grid, _) =
                ui.allocate_exact_size(Vec2::new(cell_w * 7.0, cell_h * 6.0), Sense::hover());
            let first = (cal.view_y, cal.view_m, 1);
            let start = add_days(first, -(weekday_mon0(first) as i64));
            let mut clicked: Option<Ymd> = None;
            let mut double_clicked = false;

            for i in 0..42_usize {
                let date = add_days(start, i as i64);
                let col = (i % 7) as f32;
                let row = (i / 7) as f32;
                let rect = Rect::from_min_size(
                    grid.min + Vec2::new(col * cell_w, row * cell_h),
                    Vec2::new(cell_w, cell_h),
                );
                let cell = rect.shrink2(Vec2::new(3.0, 2.0));
                let enabled = in_fy(date, fy);
                let id = Id::new(("aster_day", date.0, date.1, date.2));
                let response = ui.interact(
                    rect,
                    id,
                    if enabled {
                        Sense::click()
                    } else {
                        Sense::hover()
                    },
                );
                let hover = ctx.animate_bool_with_time(
                    id.with("h"),
                    enabled && response.hovered(),
                    anim(&ctx),
                );
                let is_selected = selected == Some(date);
                let sel_t = ctx.animate_bool_with_time(id.with("s"), is_selected, anim(&ctx) * 1.2);
                let in_view_month = (date.0, date.1) == (cal.view_y, cal.view_m);
                let is_today = date == today;

                if hover > 0.0 && sel_t < 1.0 {
                    ui.painter().rect_filled(
                        cell,
                        10.0,
                        PURPLE_TINT.gamma_multiply(hover * (1.0 - sel_t)),
                    );
                }
                if sel_t > 0.0 {
                    ui.painter()
                        .rect_filled(cell, 10.0, PURPLE.gamma_multiply(sel_t));
                }
                if is_today && !is_selected {
                    ui.painter().rect_stroke(
                        cell,
                        10.0,
                        Stroke::new(1.4_f32, PURPLE),
                        egui::StrokeKind::Inside,
                    );
                }
                let base = if !enabled {
                    DISABLED
                } else if !in_view_month {
                    Color32::from_rgb(176, 170, 190)
                } else if weekday_mon0(date) == 6 {
                    Color32::from_rgb(176, 70, 80)
                } else {
                    INK
                };
                ui.painter().text(
                    cell.center(),
                    egui::Align2::CENTER_CENTER,
                    date.2.to_string(),
                    FontId::proportional(14.5),
                    mix(base, Color32::WHITE, sel_t),
                );
                if is_today && is_selected {
                    ui.painter().circle_filled(
                        Pos2::new(cell.center().x, cell.bottom() - 6.0),
                        1.8,
                        Color32::WHITE,
                    );
                }
                if response.hovered() && enabled {
                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if response.clicked() {
                    clicked = Some(date);
                }
                if response.double_clicked() {
                    double_clicked = true;
                }
            }

            if let Some(date) = clicked {
                self.date_text = format_ymd(date);
                self.message.clear();
                if (date.0, date.1) != (cal.view_y, cal.view_m) {
                    cal.show_date(fy, date, now);
                }
                if double_clicked {
                    *go = true;
                }
            }
        });
    }
}

// ------------------------------------------------------------------ widgets

fn fy_row(ui: &mut egui::Ui, year: i32, selected: bool, current: bool) -> egui::Response {
    let ctx = ui.ctx().clone();
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 38.0), Sense::click());
    let hover = ctx.animate_bool_with_time(response.id, response.hovered(), anim(&ctx));
    let sel = ctx.animate_bool_with_time(response.id.with("s"), selected, anim(&ctx) * 1.2);

    ui.painter().rect_filled(
        rect,
        10.0,
        mix(
            mix(PANE_FILL, Color32::from_rgb(240, 236, 247), hover),
            Color32::WHITE,
            sel,
        ),
    );
    if sel > 0.0 {
        ui.painter().rect_stroke(
            rect,
            10.0,
            Stroke::new(1.4_f32, PURPLE.gamma_multiply(sel)),
            egui::StrokeKind::Inside,
        );
    }
    let dot = Pos2::new(rect.left() + 20.0, rect.center().y);
    ui.painter().circle_stroke(
        dot,
        7.0,
        Stroke::new(1.4_f32, mix(Color32::from_rgb(196, 190, 210), PURPLE, sel)),
    );
    if sel > 0.0 {
        ui.painter().circle_filled(dot, 3.6 * sel, PURPLE);
    }
    ui.painter().text(
        Pos2::new(rect.left() + 38.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        format!("{year} – {}", year + 1),
        FontId::proportional(14.0),
        mix(INK, PURPLE_DEEP, sel),
    );
    if current {
        let galley =
            ui.painter()
                .layout_no_wrap("Current".to_owned(), FontId::proportional(11.0), PURPLE);
        let pill = Rect::from_min_size(
            Pos2::new(rect.right() - galley.size().x - 24.0, rect.center().y - 9.0),
            Vec2::new(galley.size().x + 14.0, 18.0),
        );
        ui.painter().rect_filled(pill, 9.0, PURPLE_TINT);
        ui.painter().galley(
            Pos2::new(pill.left() + 7.0, pill.center().y - galley.size().y / 2.0),
            galley,
            PURPLE,
        );
    }
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn link_button(ui: &mut egui::Ui, label: &str, color: Color32) -> egui::Response {
    let ctx = ui.ctx().clone();
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(13.0), color);
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(galley.size().x + 8.0, 26.0), Sense::click());
    let hover = ctx.animate_bool_with_time(response.id, response.hovered(), anim(&ctx));
    let text_pos = Pos2::new(rect.left() + 4.0, rect.center().y - galley.size().y / 2.0);
    let underline_y = text_pos.y + galley.size().y + 1.0;
    let width = galley.size().x;
    ui.painter().galley(text_pos, galley, color);
    if hover > 0.0 {
        ui.painter().hline(
            text_pos.x..=(text_pos.x + width * hover),
            underline_y,
            Stroke::new(1.2_f32, color),
        );
    }
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn small_button(ui: &mut egui::Ui, label: &str, width: f32, height: f32) -> egui::Response {
    let ctx = ui.ctx().clone();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let hover = ctx.animate_bool_with_time(response.id, response.hovered(), anim(&ctx));
    let rect = if response.is_pointer_button_down_on() {
        rect.shrink(0.75)
    } else {
        rect
    };
    ui.painter().rect_filled(
        rect,
        8.0,
        mix(Color32::WHITE, Color32::from_rgb(243, 240, 249), hover),
    );
    ui.painter().rect_stroke(
        rect,
        8.0,
        Stroke::new(
            1.0_f32,
            mix(
                Color32::from_rgb(222, 218, 232),
                Color32::from_rgb(207, 195, 226),
                hover,
            ),
        ),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::proportional(13.0),
        mix(INK, PURPLE, hover),
    );
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn chevron_button(
    ui: &mut egui::Ui,
    rect: Rect,
    direction: f32,
    enabled: bool,
    salt: &str,
) -> egui::Response {
    let ctx = ui.ctx().clone();
    let response = ui.interact(
        rect,
        Id::new(("aster_cal_chevron", salt)),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let hover = ctx.animate_bool_with_time(response.id, enabled && response.hovered(), anim(&ctx));
    ui.painter().rect_filled(
        rect,
        9.0,
        Color32::from_rgb(241, 237, 248).gamma_multiply(hover),
    );
    let center = rect.center();
    let color = if enabled {
        mix(Color32::from_rgb(100, 91, 116), PURPLE, hover)
    } else {
        DISABLED
    };
    let stroke = Stroke::new(1.8_f32, color);
    ui.painter().line_segment(
        [
            center + Vec2::new(-2.5 * direction, -5.0),
            center + Vec2::new(2.5 * direction, 0.0),
        ],
        stroke,
    );
    ui.painter().line_segment(
        [
            center + Vec2::new(2.5 * direction, 0.0),
            center + Vec2::new(-2.5 * direction, 5.0),
        ],
        stroke,
    );
    if enabled && response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn primary_button_at(ui: &mut egui::Ui, rect: Rect, label: &str, enabled: bool) -> egui::Response {
    let ctx = ui.ctx().clone();
    let response = ui.interact(
        rect,
        Id::new(("aster_primary_button", label)),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let hover = ctx.animate_bool_with_time(response.id, enabled && response.hovered(), anim(&ctx));
    let rect = if enabled && response.is_pointer_button_down_on() {
        rect.shrink(0.75)
    } else {
        rect
    };
    let fill = if enabled {
        mix(PURPLE, PURPLE_DEEP, hover)
    } else {
        Color32::from_rgb(208, 201, 222)
    };
    ui.painter().rect_filled(rect, 10.0, fill);
    ui.painter().text(
        rect.center() - Vec2::new(8.0, 0.0),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::proportional(14.0),
        Color32::WHITE,
    );
    // arrow
    let arrow = Pos2::new(rect.right() - 22.0 + 3.0 * hover, rect.center().y);
    let stroke = Stroke::new(1.8_f32, Color32::WHITE);
    ui.painter().line_segment(
        [arrow + Vec2::new(-5.0, 0.0), arrow + Vec2::new(4.0, 0.0)],
        stroke,
    );
    ui.painter().line_segment(
        [arrow + Vec2::new(0.0, -4.0), arrow + Vec2::new(4.0, 0.0)],
        stroke,
    );
    ui.painter().line_segment(
        [arrow + Vec2::new(0.0, 4.0), arrow + Vec2::new(4.0, 0.0)],
        stroke,
    );
    if enabled && response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}
