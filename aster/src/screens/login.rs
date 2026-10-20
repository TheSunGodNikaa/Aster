use std::{fs, path::PathBuf};

use eframe::egui::{
    self, Align, Color32, FontId, Id, Key, Margin, Modifiers, Pos2, Rect, RichText, Sense, Stroke,
    Vec2,
};

use crate::{
    app::{AsterApp, Stage},
    screens,
};

const INK: Color32 = Color32::from_rgb(43, 38, 53);
const MUTED: Color32 = Color32::from_rgb(116, 109, 128);
const SOFT: Color32 = Color32::from_rgb(132, 123, 147);
const PURPLE: Color32 = Color32::from_rgb(91, 65, 139);
const PURPLE_DEEP: Color32 = Color32::from_rgb(74, 50, 120);
const PURPLE_TINT: Color32 = Color32::from_rgb(241, 237, 248);
const BORDER: Color32 = Color32::from_rgb(234, 231, 240);
const DANGER: Color32 = Color32::from_rgb(171, 54, 62);
const DANGER_TINT: Color32 = Color32::from_rgb(253, 240, 240);
const SUCCESS: Color32 = Color32::from_rgb(45, 146, 94);
const AMBER: Color32 = Color32::from_rgb(200, 133, 30);

const LOGIN_CARD: Vec2 = Vec2::new(440.0, 456.0);
const SETUP_CARD: Vec2 = Vec2::new(440.0, 548.0);
const CARD_PADDING: Vec2 = Vec2::new(36.0, 32.0);

const CREDENTIAL_SERVICE: &str = "ASTER Billing";

// ------------------------------------------------------ remembered user names

fn data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    base.map(|dir| dir.join("ASTER"))
}

fn users_file() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("users.txt"))
}

fn load_known_users() -> Vec<String> {
    let Some(path) = users_file() else {
        return Vec::new();
    };
    let mut names: Vec<String> = fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
        .collect();
    names.dedup();
    names
}

fn remember_user(name: &str) {
    let name = name.trim();
    if name.is_empty() {
        return;
    }
    let mut names = load_known_users();
    names.retain(|existing| !existing.eq_ignore_ascii_case(name));
    names.insert(0, name.to_owned()); // most recent first
    names.truncate(12);
    if let Some(path) = users_file() {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(path, names.join("\n"));
    }
}

// ------------------------------------ Windows Credential Manager (saved login)

#[cfg(windows)]
mod credentials {
    use super::CREDENTIAL_SERVICE;

    pub(super) const AVAILABLE: bool = true;

    // Windows target names are not reliably case-sensitive, so normalise.
    fn entry(user: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(CREDENTIAL_SERVICE, &user.trim().to_lowercase())
            .map_err(|e| e.to_string())
    }

    pub(super) fn get(user: &str) -> Result<Option<String>, String> {
        match entry(user)?.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    pub(super) fn set(user: &str, password: &str) -> Result<(), String> {
        let entry = entry(user)?;
        entry.set_password(password).map_err(|e| e.to_string())?;
        // Read it back so a silent failure is reported instead of ignored.
        match entry.get_password() {
            Ok(saved) if saved == password => Ok(()),
            Ok(_) => Err("Windows stored a different value than expected.".into()),
            Err(error) => Err(error.to_string()),
        }
    }

    pub(super) fn delete(user: &str) {
        if let Ok(entry) = entry(user) {
            let _ = entry.delete_credential();
        }
    }
}

#[cfg(not(windows))]
mod credentials {
    pub(super) const AVAILABLE: bool = false;
    pub(super) fn get(_user: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
    pub(super) fn set(_user: &str, _password: &str) -> Result<(), String> {
        Ok(())
    }
    pub(super) fn delete(_user: &str) {}
}

// ----------------------------------------------------------------- UI state

#[derive(Clone, Default)]
struct LoginUi {
    users: Vec<String>,
    loaded: bool,
    focused_once: bool,
    highlight: i32,
    open_all: bool,
    dismissed: bool,
    popup_hover: bool,
    show_password: bool,
    remember: bool,
    autofilled: bool,
    filled_for: String,
    error_at: f64,
    last_error: String,
    cred_note: String,
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

fn initial_of(name: &str) -> String {
    name.chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".into())
}

// ------------------------------------------------------------------ screens

impl AsterApp {
    // ---------------------------------------------------------------- sign in

    pub(crate) fn show_login(&mut self, ctx: &egui::Context) {
        ctx.data_mut(|d| {
            d.insert_temp(
                Id::new("aster_anim_time"),
                if self.reduced_motion { 0.0_f32 } else { 0.14_f32 },
            )
        });
        let state_id = Id::new("aster_login_ui");
        let mut ls: LoginUi = ctx.data(|d| d.get_temp(state_id)).unwrap_or_default();
        if !ls.loaded {
            ls.users = load_known_users();
            ls.loaded = true;
            ls.highlight = -1;
            // Pre-select the last signed-in user for speed.
            if self.username.trim().is_empty() {
                if let Some(first) = ls.users.first() {
                    self.username = first.clone();
                }
            }
        }
        let now = ctx.input(|i| i.time);

        // Look up a saved Windows credential when the user name is settled
        // (picked, tabbed away from, or an exact known name) - not per keystroke.
        let typed = self.username.trim().to_owned();
        let editing = ctx.memory(|m| m.has_focus(Id::new("aster_login_user")));
        let known = ls.users.iter().any(|u| u.eq_ignore_ascii_case(&typed));
        if typed != ls.filled_for && (!editing || known) {
            ls.filled_for = typed.clone();
            let saved = if credentials::AVAILABLE && !typed.is_empty() {
                credentials::get(&typed)
            } else {
                Ok(None)
            };
            match saved {
                Ok(Some(password)) if self.password.is_empty() || ls.autofilled => {
                    self.password = password;
                    ls.autofilled = true;
                    ls.remember = true;
                    ls.cred_note = "Password filled in from Windows Credential Manager".into();
                }
                Ok(Some(_)) => {}
                Ok(None) => {
                    if ls.autofilled {
                        self.password.clear();
                        ls.autofilled = false;
                        ls.remember = false;
                    }
                    ls.cred_note.clear();
                }
                Err(error) => {
                    ls.cred_note = format!("Could not read Windows Credential Manager: {error}");
                }
            }
        }

        let mut submit = false;
        let mut back = false;

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(248, 247, 251))
                    .inner_margin(0.0),
            )
            .show(ctx, |ui| {
                screens::paint_wallpaper(ui, self);
                let card = Rect::from_center_size(ui.max_rect().center(), LOGIN_CARD);
                paint_card(ui, card);

                let mut form = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(card.shrink2(CARD_PADDING))
                        .layout(egui::Layout::top_down(Align::Min)),
                );
                let ui = &mut form;

                brand(ui);
                ui.add_space(22.0);
                ui.label(RichText::new("Welcome back").size(26.0).strong().color(INK));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.label(RichText::new("Signing in to").size(13.5).color(MUTED));
                    year_pill(ui, self.selected_year);
                    ui.label(RichText::new(&self.date_text).size(13.0).color(SOFT));
                });
                ui.add_space(22.0);

                // ---- user name
                field_label(ui, "User name");
                let user_id = Id::new("aster_login_user");
                let pass_id = Id::new("aster_login_pass");
                if !ls.focused_once {
                    ls.focused_once = true;
                    let target = if self.username.trim().is_empty() { user_id } else { pass_id };
                    ui.memory_mut(|m| m.request_focus(target));
                }

                // keyboard navigation of the suggestion list (consumed before the text box)
                let query = self.username.trim().to_lowercase();
                let show_list = !query.is_empty() || ls.open_all;
                let mut suggestions: Vec<String> = ls
                    .users
                    .iter()
                    .filter(|u| {
                        let lower = u.to_lowercase();
                        lower != query && (query.is_empty() || lower.contains(&query))
                    })
                    .cloned()
                    .collect();
                suggestions.sort_by_key(|u| !u.to_lowercase().starts_with(&query));
                suggestions.truncate(5);

                let user_has_focus = ui.memory(|m| m.has_focus(user_id));
                let list_visible = (user_has_focus || ls.popup_hover)
                    && show_list
                    && !ls.dismissed
                    && !suggestions.is_empty();

                let mut picked: Option<String> = None;
                if user_has_focus {
                    if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) {
                        if list_visible {
                            ls.highlight = (ls.highlight + 1).rem_euclid(suggestions.len() as i32);
                        } else {
                            ls.open_all = true;
                            ls.dismissed = false;
                        }
                    }
                    if list_visible {
                        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) {
                            ls.highlight = if ls.highlight <= 0 {
                                suggestions.len() as i32 - 1
                            } else {
                                ls.highlight - 1
                            };
                        }
                        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                            ls.dismissed = true;
                        }
                        if ls.highlight >= 0
                            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter))
                        {
                            picked = suggestions.get(ls.highlight as usize).cloned();
                        }
                        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab)) {
                            let index = ls.highlight.max(0) as usize;
                            picked = suggestions.get(index).cloned();
                        }
                    }
                }

                let user_width = ui.available_width();
                let user_response = ui.add(
                    egui::TextEdit::singleline(&mut self.username)
                        .id(user_id)
                        .desired_width(user_width)
                        .margin(Margin::symmetric(12, 10))
                        .hint_text("Enter your user name"),
                );
                if user_response.changed() {
                    ls.highlight = -1;
                    ls.open_all = false;
                    ls.dismissed = false;
                    self.message.clear();
                }
                if user_response.clicked() {
                    ls.open_all = true;
                    ls.dismissed = false;
                }
                if user_response.lost_focus()
                    && ui.input(|i| i.key_pressed(Key::Enter))
                    && picked.is_none()
                {
                    ui.memory_mut(|m| m.request_focus(pass_id));
                }

                // suggestion popup
                ls.popup_hover = false;
                if list_visible {
                    let popup_pos = user_response.rect.left_bottom() + Vec2::new(0.0, 6.0);
                    let width = user_response.rect.width();
                    let area = egui::Area::new(Id::new("aster_user_suggestions"))
                        .order(egui::Order::Foreground)
                        .fixed_pos(popup_pos)
                        .show(ui.ctx(), |ui| {
                            egui::Frame::new()
                                .fill(Color32::WHITE)
                                .stroke(Stroke::new(1.0_f32, BORDER))
                                .corner_radius(12.0)
                                .shadow(egui::Shadow {
                                    offset: [0, 6],
                                    blur: 18,
                                    spread: 0,
                                    color: Color32::from_black_alpha(28),
                                })
                                .inner_margin(6.0)
                                .show(ui, |ui| {
                                    ui.set_width(width - 12.0);
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    ui.label(
                                        RichText::new("SUGGESTED USERS")
                                            .size(10.0)
                                            .strong()
                                            .color(SOFT),
                                    );
                                    ui.add_space(2.0);
                                    for (index, name) in suggestions.iter().enumerate() {
                                        let highlighted = ls.highlight == index as i32;
                                        if suggestion_row(ui, name, &query, highlighted).clicked() {
                                            picked = Some(name.clone());
                                        }
                                    }
                                });
                        });
                    let pointer = ui.ctx().input(|i| i.pointer.hover_pos());
                    ls.popup_hover = pointer.map_or(false, |p| area.response.rect.contains(p));
                }

                if let Some(name) = picked {
                    self.username = name;
                    ls.highlight = -1;
                    ls.open_all = false;
                    ls.dismissed = true;
                    ls.popup_hover = false;
                    self.message.clear();
                    ui.memory_mut(|m| m.request_focus(pass_id));
                }
                ui.add_space(14.0);

                // ---- password
                field_label(ui, "Password");
                let pass_response = password_input(
                    ui,
                    &mut self.password,
                    pass_id,
                    "Enter your password",
                    &mut ls.show_password,
                );
                if pass_response.changed() {
                    ls.autofilled = false;
                    self.message.clear();
                }
                if pass_response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    submit = true;
                }
                ui.add_space(12.0);

                // ---- remember (Windows Credential Manager)
                if credentials::AVAILABLE {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        check_box(ui, &mut ls.remember);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.label(
                                RichText::new("Remember my password on this PC")
                                    .size(13.0)
                                    .color(INK),
                            );
                            let (note, note_color) = if ls.cred_note.is_empty() {
                                ("Saved securely in Windows Credential Manager".to_owned(), SOFT)
                            } else if ls.cred_note.starts_with("Could not") {
                                (ls.cred_note.clone(), DANGER)
                            } else {
                                (ls.cred_note.clone(), SUCCESS)
                            };
                            ui.label(RichText::new(note).size(11.5).color(note_color));
                        });
                    });
                } else {
                    ui.add_space(2.0);
                }

                // ---- error banner (fades in)
                if !self.message.is_empty() && self.message != ls.last_error {
                    ls.last_error = self.message.clone();
                    ls.error_at = now;
                }
                if self.message.is_empty() {
                    ls.last_error.clear();
                }
                ui.add_space(12.0);
                if !self.message.is_empty() {
                    let t = if anim(ui.ctx()) == 0.0 {
                        1.0
                    } else {
                        (((now - ls.error_at) / 0.2) as f32).clamp(0.0, 1.0)
                    };
                    if t < 1.0 {
                        ui.ctx().request_repaint();
                    }
                    error_banner(ui, &self.message, t);
                } else {
                    ui.add_space(0.0);
                }

                // ---- actions (pinned to the bottom of the card)
                let actions = Rect::from_min_size(
                    Pos2::new(card.left() + CARD_PADDING.x, card.bottom() - CARD_PADDING.y - 44.0),
                    Vec2::new(card.width() - CARD_PADDING.x * 2.0, 44.0),
                );
                if back_button_at(ui, Rect::from_min_size(actions.min, Vec2::new(92.0, 44.0))).clicked()
                {
                    back = true;
                }
                if primary_button_at(
                    ui,
                    Rect::from_min_max(
                        Pos2::new(actions.left() + 104.0, actions.top()),
                        actions.max,
                    ),
                    "Sign in",
                )
                .clicked()
                {
                    submit = true;
                }
                screens::show_background_error(ui, self);
            });

        if back {
            self.password.clear();
            self.message.clear();
            ls.autofilled = false;
            ls.loaded = false;
            ls.focused_once = false;
            ls.filled_for.clear();
            ctx.data_mut(|d| d.insert_temp(state_id, ls));
            self.set_stage(Stage::SelectYearAndDate, ctx);
            return;
        }
        if submit {
            self.attempt_login(ctx, &mut ls);
        }
        ctx.data_mut(|d| d.insert_temp(state_id, ls));
    }

    fn attempt_login(&mut self, ctx: &egui::Context, ls: &mut LoginUi) {
        let username = self.username.trim().to_owned();
        if username.is_empty() {
            self.message = "Enter your user name.".into();
            return;
        }
        if self.password.is_empty() {
            self.message = "Enter your password.".into();
            return;
        }
        let password = std::mem::take(&mut self.password);
        let result = self
            .db
            .as_ref()
            .map(|db| db.verify_user(&username, &password));
        match result {
            Some(Ok(true)) => {
                let mut save_error = None;
                if credentials::AVAILABLE {
                    if ls.remember {
                        if let Err(error) = credentials::set(&username, &password) {
                            save_error = Some(format!(
                                "Signed in, but the password could not be saved: {error}"
                            ));
                        }
                    } else {
                        credentials::delete(&username);
                    }
                }
                remember_user(&username);
                self.current_user = username;
                self.display_name = self.current_user.clone();
                self.profile_message = save_error.unwrap_or_default();
                self.message.clear();
                self.workspace_page = super::WorkspacePage::Dashboard;
                ls.loaded = false;
                ls.focused_once = false;
                ls.autofilled = false;
                ls.filled_for.clear();
                self.set_stage(Stage::Main, ctx);
            }
            Some(Ok(false)) => {
                if ls.autofilled && credentials::AVAILABLE {
                    credentials::delete(&username);
                    ls.autofilled = false;
                    ls.remember = false;
                    self.message =
                        "The saved password is out of date. Enter your current password.".into();
                } else {
                    self.message = "User name or password is incorrect.".into();
                }
                ctx.memory_mut(|m| m.request_focus(Id::new("aster_login_pass")));
            }
            Some(Err(error)) => self.message = error,
            None => self.message = "Database is unavailable.".into(),
        }
    }

    // ------------------------------------------------------ administrator setup

    pub(crate) fn show_first_run_setup(&mut self, ctx: &egui::Context) {
        ctx.data_mut(|d| {
            d.insert_temp(
                Id::new("aster_anim_time"),
                if self.reduced_motion { 0.0_f32 } else { 0.14_f32 },
            )
        });
        let now = ctx.input(|i| i.time);
        let state_id = Id::new("aster_setup_ui");
        let mut ls: LoginUi = ctx.data(|d| d.get_temp(state_id)).unwrap_or_default();
        let mut submit = false;
        let mut back = false;

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(248, 247, 251))
                    .inner_margin(0.0),
            )
            .show(ctx, |ui| {
                screens::paint_wallpaper(ui, self);
                let card = Rect::from_center_size(ui.max_rect().center(), SETUP_CARD);
                paint_card(ui, card);
                let mut form = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(card.shrink2(CARD_PADDING))
                        .layout(egui::Layout::top_down(Align::Min)),
                );
                let ui = &mut form;

                brand(ui);
                ui.add_space(22.0);
                ui.label(
                    RichText::new("Create administrator")
                        .size(26.0)
                        .strong()
                        .color(INK),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Set up the first account for this installation.")
                        .size(13.5)
                        .color(MUTED),
                );
                ui.add_space(22.0);

                field_label(ui, "User name");
                let width = ui.available_width();
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.username)
                        .desired_width(width)
                        .margin(Margin::symmetric(12, 10))
                        .hint_text("Administrator name"),
                );
                if response.changed() {
                    self.message.clear();
                }
                ui.add_space(14.0);

                field_label(ui, "Password");
                let pw_response = password_input(
                    ui,
                    &mut self.password,
                    Id::new("aster_setup_pass"),
                    "At least 10 characters",
                    &mut ls.show_password,
                );
                if pw_response.changed() {
                    self.message.clear();
                }
                strength_bar(ui, &self.password);
                ui.add_space(14.0);

                field_label(ui, "Confirm password");
                let show = ls.show_password;
                let confirm_width = ui.available_width();
                let confirm = ui.add(
                    egui::TextEdit::singleline(&mut self.confirm_password)
                        .password(!show)
                        .desired_width(confirm_width)
                        .margin(Margin::symmetric(12, 10))
                        .hint_text("Repeat password"),
                );
                if confirm.changed() {
                    self.message.clear();
                }
                if confirm.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    submit = true;
                }
                if !self.confirm_password.is_empty() {
                    ui.add_space(4.0);
                    let matches = self.confirm_password == self.password;
                    ui.label(
                        RichText::new(if matches { "Passwords match" } else { "Passwords do not match yet" })
                            .size(12.0)
                            .color(if matches { SUCCESS } else { AMBER }),
                    );
                }

                if !self.message.is_empty() && self.message != ls.last_error {
                    ls.last_error = self.message.clone();
                    ls.error_at = now;
                }
                if self.message.is_empty() {
                    ls.last_error.clear();
                }
                ui.add_space(10.0);
                if !self.message.is_empty() {
                    let t = if anim(ui.ctx()) == 0.0 {
                        1.0
                    } else {
                        (((now - ls.error_at) / 0.2) as f32).clamp(0.0, 1.0)
                    };
                    if t < 1.0 {
                        ui.ctx().request_repaint();
                    }
                    error_banner(ui, &self.message, t);
                }

                let actions = Rect::from_min_size(
                    Pos2::new(card.left() + CARD_PADDING.x, card.bottom() - CARD_PADDING.y - 44.0),
                    Vec2::new(card.width() - CARD_PADDING.x * 2.0, 44.0),
                );
                if back_button_at(ui, Rect::from_min_size(actions.min, Vec2::new(92.0, 44.0))).clicked()
                {
                    back = true;
                }
                if primary_button_at(
                    ui,
                    Rect::from_min_max(
                        Pos2::new(actions.left() + 104.0, actions.top()),
                        actions.max,
                    ),
                    "Create account",
                )
                .clicked()
                {
                    submit = true;
                }
                screens::show_background_error(ui, self);
            });

        if back {
            self.message.clear();
            ctx.data_mut(|d| d.insert_temp(state_id, ls));
            self.set_stage(Stage::SelectYearAndDate, ctx);
            return;
        }
        if submit {
            let username = self.username.trim().to_owned();
            if username.is_empty() {
                self.message = "Enter a user name.".into();
            } else if self.password.chars().count() < 10 {
                self.message = "Use a password with at least 10 characters.".into();
            } else if self.password != self.confirm_password {
                self.message = "The passwords do not match.".into();
            } else if let Some(db) = &self.db {
                match db.create_admin(&username, &self.password) {
                    Ok(()) => {
                        remember_user(&username);
                        self.username = username;
                        self.password.clear();
                        self.confirm_password.clear();
                        self.message.clear();
                        ls.show_password = false;
                        ctx.data_mut(|d| d.remove::<LoginUi>(Id::new("aster_login_ui")));
                        self.set_stage(Stage::Login, ctx);
                    }
                    Err(error) => self.message = error,
                }
            } else {
                self.message = "Database is unavailable.".into();
            }
        }
        ctx.data_mut(|d| d.insert_temp(state_id, ls));
    }
}

// ------------------------------------------------------------------ widgets

fn paint_card(ui: &egui::Ui, card: Rect) {
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
    ui.painter().rect_stroke(
        card,
        18.0,
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );
}

fn brand(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(RichText::new("ASTER").size(20.0).strong().color(PURPLE));
        ui.label(RichText::new("BILLING").size(10.5).strong().color(SOFT));
    });
}

fn year_pill(ui: &mut egui::Ui, year: i32) {
    egui::Frame::new()
        .fill(PURPLE_TINT)
        .corner_radius(20.0)
        .inner_margin(Margin::symmetric(10, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!("FY {} – {}", year, year + 1))
                    .size(12.0)
                    .strong()
                    .color(PURPLE),
            );
        });
}

fn field_label(ui: &mut egui::Ui, label: &str) {
    ui.label(RichText::new(label).size(12.5).strong().color(MUTED));
    ui.add_space(4.0);
}

fn error_banner(ui: &mut egui::Ui, message: &str, t: f32) {
    ui.scope(|ui| {
        ui.set_opacity(t);
        egui::Frame::new()
            .fill(DANGER_TINT)
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(240, 208, 208)))
            .corner_radius(10.0)
            .inner_margin(Margin::symmetric(12, 9))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let (dot, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                    ui.painter().circle_filled(dot.center(), 5.0, DANGER);
                    ui.painter().text(
                        dot.center(),
                        egui::Align2::CENTER_CENTER,
                        "!",
                        FontId::proportional(9.0),
                        Color32::WHITE,
                    );
                    ui.label(RichText::new(message).size(12.5).color(DANGER));
                });
            });
    });
}

fn check_box(ui: &mut egui::Ui, value: &mut bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
    if response.clicked() {
        *value = !*value;
    }
    let ctx = ui.ctx().clone();
    let t = ctx.animate_bool_with_time(response.id, *value, anim(&ctx));
    let hover = ctx.animate_bool_with_time(response.id.with("h"), response.hovered(), anim(&ctx));
    let box_rect = rect.shrink(1.0);
    ui.painter().rect_filled(
        box_rect,
        6.0,
        mix(mix(Color32::WHITE, PURPLE_TINT, hover), PURPLE, t),
    );
    ui.painter().rect_stroke(
        box_rect,
        6.0,
        Stroke::new(1.4_f32, mix(Color32::from_rgb(206, 200, 220), PURPLE, t.max(hover))),
        egui::StrokeKind::Inside,
    );
    if t > 0.0 {
        let c = box_rect.center();
        let stroke = Stroke::new(1.8_f32, Color32::WHITE.gamma_multiply(t));
        ui.painter()
            .line_segment([c + Vec2::new(-3.6, 0.2), c + Vec2::new(-1.0, 3.0)], stroke);
        ui.painter()
            .line_segment([c + Vec2::new(-1.0, 3.0), c + Vec2::new(4.0, -3.0)], stroke);
    }
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn eye_button(ui: &mut egui::Ui, visible: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), anim(ui.ctx()));
    ui.painter().rect_filled(
        rect,
        9.0,
        Color32::from_rgb(243, 240, 249).gamma_multiply(t),
    );
    let c = rect.center();
    let color = mix(SOFT, PURPLE, if visible { 1.0 } else { t });
    let stroke = Stroke::new(1.5_f32, color);
    ui.painter().add(egui::Shape::closed_line(
        vec![
            c + Vec2::new(-8.0, 0.0),
            c + Vec2::new(-4.0, -4.5),
            c + Vec2::new(4.0, -4.5),
            c + Vec2::new(8.0, 0.0),
            c + Vec2::new(4.0, 4.5),
            c + Vec2::new(-4.0, 4.5),
        ],
        stroke,
    ));
    ui.painter().circle_stroke(c, 2.4, stroke);
    if !visible {
        ui.painter()
            .line_segment([c + Vec2::new(-7.0, 7.0), c + Vec2::new(7.0, -7.0)], stroke);
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.on_hover_text(if visible { "Hide password" } else { "Show password" })
}

fn password_input(
    ui: &mut egui::Ui,
    value: &mut String,
    id: Id,
    hint: &str,
    visible: &mut bool,
) -> egui::Response {
    let mut response = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let width = (ui.available_width() - 46.0).max(80.0);
        response = Some(
            ui.add(
                egui::TextEdit::singleline(value)
                    .id(id)
                    .password(!*visible)
                    .desired_width(width)
                    .margin(Margin::symmetric(12, 10))
                    .hint_text(hint),
            ),
        );
        if eye_button(ui, *visible).clicked() {
            *visible = !*visible;
        }
    });
    response.expect("password response")
}

fn strength_bar(ui: &mut egui::Ui, password: &str) {
    let ctx = ui.ctx().clone();
    let len = password.chars().count();
    let mut score = 0;
    if len >= 10 {
        score += 1;
    }
    if len >= 14 {
        score += 1;
    }
    if password.chars().any(|c| c.is_uppercase()) && password.chars().any(|c| c.is_lowercase()) {
        score += 1;
    }
    if password.chars().any(|c| c.is_ascii_digit()) {
        score += 1;
    }
    if password.chars().any(|c| !c.is_alphanumeric() && !c.is_whitespace()) {
        score += 1;
    }
    let (label, color) = if password.is_empty() {
        ("At least 10 characters", SOFT)
    } else {
        match score {
            0..=1 => ("Weak", DANGER),
            2 => ("Fair", AMBER),
            3 => ("Good", Color32::from_rgb(98, 150, 60)),
            _ => ("Strong", SUCCESS),
        }
    };
    ui.add_space(6.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 5.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, 2.5, Color32::from_rgb(234, 231, 240));
    let fraction = if password.is_empty() { 0.0 } else { score as f32 / 5.0 };
    let filled =
        ctx.animate_value_with_time(Id::new("aster_setup_strength"), fraction * rect.width(), anim(&ctx) * 1.8);
    if filled > 0.5 {
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(filled, rect.height())),
            2.5,
            color,
        );
    }
    ui.add_space(4.0);
    ui.label(RichText::new(label).size(12.0).color(color));
}

fn suggestion_row(ui: &mut egui::Ui, name: &str, query: &str, highlighted: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 38.0), Sense::click());
    let ctx = ui.ctx().clone();
    let hover = ctx.animate_bool_with_time(response.id, response.hovered() || highlighted, anim(&ctx));
    if hover > 0.0 {
        ui.painter()
            .rect_filled(rect, 8.0, PURPLE_TINT.gamma_multiply(hover));
    }
    let avatar = Pos2::new(rect.left() + 20.0, rect.center().y);
    ui.painter().circle_filled(avatar, 13.0, PURPLE_TINT);
    ui.painter().text(
        avatar,
        egui::Align2::CENTER_CENTER,
        initial_of(name),
        FontId::proportional(12.0),
        PURPLE,
    );

    // Bold the matching part of the name.
    let lower = name.to_lowercase();
    let text_pos = Pos2::new(rect.left() + 42.0, rect.center().y);
    let font = FontId::proportional(14.0);
    match (!query.is_empty()).then(|| lower.find(query)).flatten() {
        Some(start) if name.is_char_boundary(start) && name.is_char_boundary(start + query.len()) => {
            let end = start + query.len();
            let (head, mid, tail) = (&name[..start], &name[start..end], &name[end..]);
            let mut job = egui::text::LayoutJob::default();
            for (text, color, strong) in [(head, MUTED, false), (mid, PURPLE, true), (tail, MUTED, false)] {
                if text.is_empty() {
                    continue;
                }
                job.append(
                    text,
                    0.0,
                    egui::TextFormat {
                        font_id: font.clone(),
                        color,
                        underline: if strong { Stroke::new(1.2_f32, PURPLE) } else { Stroke::NONE },
                        ..Default::default()
                    },
                );
            }
            let galley = ui.painter().layout_job(job);
            ui.painter().galley(
                Pos2::new(text_pos.x, text_pos.y - galley.size().y / 2.0),
                galley,
                MUTED,
            );
        }
        _ => {
            ui.painter().text(
                text_pos,
                egui::Align2::LEFT_CENTER,
                name,
                font,
                INK,
            );
        }
    }
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn primary_button_at(ui: &mut egui::Ui, rect: Rect, label: &str) -> egui::Response {
    let ctx = ui.ctx().clone();
    let response = ui.interact(rect, Id::new(("aster_login_primary", label)), Sense::click());
    let hover = ctx.animate_bool_with_time(response.id, response.hovered(), anim(&ctx));
    let rect = if response.is_pointer_button_down_on() {
        rect.shrink(0.75)
    } else {
        rect
    };
    ui.painter()
        .rect_filled(rect, 11.0, mix(PURPLE, PURPLE_DEEP, hover));
    ui.painter().text(
        rect.center() - Vec2::new(8.0, 0.0),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::proportional(14.5),
        Color32::WHITE,
    );
    let arrow = Pos2::new(rect.center().x + 38.0 + 3.0 * hover, rect.center().y);
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
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn back_button_at(ui: &mut egui::Ui, rect: Rect) -> egui::Response {
    let ctx = ui.ctx().clone();
    let response = ui.interact(rect, Id::new("aster_login_back"), Sense::click());
    let hover = ctx.animate_bool_with_time(response.id, response.hovered(), anim(&ctx));
    ui.painter().rect_filled(
        rect,
        11.0,
        mix(Color32::WHITE, Color32::from_rgb(243, 240, 249), hover),
    );
    ui.painter().rect_stroke(
        rect,
        11.0,
        Stroke::new(
            1.0_f32,
            mix(Color32::from_rgb(222, 218, 232), Color32::from_rgb(207, 195, 226), hover),
        ),
        egui::StrokeKind::Inside,
    );
    let c = Pos2::new(rect.left() + 26.0 - 2.0 * hover, rect.center().y);
    let stroke = Stroke::new(1.7_f32, mix(MUTED, PURPLE, hover));
    ui.painter()
        .line_segment([c + Vec2::new(3.0, -5.0), c + Vec2::new(-2.0, 0.0)], stroke);
    ui.painter()
        .line_segment([c + Vec2::new(-2.0, 0.0), c + Vec2::new(3.0, 5.0)], stroke);
    ui.painter().text(
        Pos2::new(rect.left() + 40.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        "Back",
        FontId::proportional(14.0),
        mix(INK, PURPLE, hover),
    );
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}
