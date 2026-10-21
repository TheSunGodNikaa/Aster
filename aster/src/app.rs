use eframe::egui;

use crate::{
    database::Database,
    date::local_date,
    screens::{self, WorkspacePage},
    theme,
};

const BACKGROUND_PNG: &[u8] = include_bytes!("../assets/aster-background.png");
const LOGO_PNG: &[u8] = include_bytes!("../assets/Aster-logo.png");

/// Smallest size the main workspace can be shrunk to before the layout breaks.
const MAIN_MIN_SIZE: [f32; 2] = [1100.0, 680.0];

#[derive(Clone)]
pub(crate) enum Stage {
    SelectYearAndDate,
    FirstRunSetup,
    Login,
    Main,
    Error(String),
}

pub(crate) struct AsterApp {
    pub(crate) db: Option<Database>,
    pub(crate) stage: Stage,
    pub(crate) background: Option<egui::TextureHandle>,
    pub(crate) background_error: Option<String>,
    pub(crate) financial_years: Vec<i32>,
    pub(crate) selected_year: i32,
    pub(crate) date_text: String,
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) confirm_password: String,
    pub(crate) current_user: String,
    pub(crate) message: String,
    pub(crate) workspace_page: WorkspacePage,
    pub(crate) open_pages: Vec<WorkspacePage>,
    pub(crate) sidebar_collapsed: bool,
    pub(crate) display_name: String,
    pub(crate) profile_email: String,
    pub(crate) profile_message: String,
    pub(crate) current_password: String,
    pub(crate) new_password: String,
    pub(crate) confirm_new_password: String,
    pub(crate) reduced_motion: bool,
    pub(crate) compact_density: bool,
    pub(crate) show_tooltips: bool,
    pub(crate) pending_center: u8,
    pub(crate) layout_ready: bool,
    pub(crate) logo: Option<egui::TextureHandle>,
}

impl AsterApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::set_light_theme(&cc.egui_ctx);
        let (year, month, day) = local_date();
        let current_start = if month >= 4 { year } else { year - 1 };
        let (db, years, stage) = match Database::open() {
            Ok(db) => match db.financial_years() {
                Ok(years) => (Some(db), years, Stage::SelectYearAndDate),
                Err(error) => {
                    crate::logging::log_message("ERROR", &error);
                    (None, Vec::new(), Stage::Error(error))
                }
            },
            Err(error) => {
                crate::logging::log_message("ERROR", &error);
                (None, Vec::new(), Stage::Error(error))
            }
        };

        let (background, background_error) = match image::load_from_memory(BACKGROUND_PNG) {
            Ok(image) => {
                let image = image.to_rgba8();
                let size = [image.width() as usize, image.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
                (
                    Some(cc.egui_ctx.load_texture(
                        "aster-background",
                        color_image,
                        egui::TextureOptions::LINEAR,
                    )),
                    None,
                )
            }
            Err(error) => {
                crate::logging::log_message(
                    "ERROR",
                    &format!("Could not load ASTER background: {error}"),
                );
                (None, Some(error.to_string()))
            }
        };

        Self {
            db,
            stage,
            background,
            background_error,
            financial_years: years,
            selected_year: current_start,
            date_text: format!("{day:02}/{month:02}/{year}"),
            username: String::new(),
            password: String::new(),
            confirm_password: String::new(),
            current_user: String::new(),
            message: String::new(),
            workspace_page: WorkspacePage::Dashboard,
            open_pages: vec![WorkspacePage::Dashboard],
            sidebar_collapsed: false,
            display_name: String::new(),
            profile_email: String::new(),
            profile_message: String::new(),
            current_password: String::new(),
            new_password: String::new(),
            confirm_new_password: String::new(),
            reduced_motion: false,
            compact_density: false,
            show_tooltips: true,
            pending_center: 0,
            layout_ready: false,
            logo: image::load_from_memory(LOGO_PNG).ok().map(|img| {
                let img = img.to_rgba8();
                let size = [img.width() as usize, img.height() as usize];
                cc.egui_ctx.load_texture(
                    "aster-logo",
                    egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
                    egui::TextureOptions::LINEAR,
                )
            }),
        }
    }

    pub(crate) fn open_page(&mut self, page: WorkspacePage) {
        self.workspace_page = page;
        if !self.open_pages.contains(&page) {
            self.open_pages.push(page);
        }
    }

    pub(crate) fn close_page(&mut self, page: WorkspacePage) {
        if page == WorkspacePage::Dashboard || self.open_pages.len() == 1 {
            return;
        }
        self.open_pages.retain(|open| *open != page);
        if self.workspace_page == page {
            self.workspace_page = *self.open_pages.last().unwrap_or(&WorkspacePage::Dashboard);
        }
    }

    pub(crate) fn set_stage(&mut self, stage: Stage, ctx: &egui::Context) {
        self.stage = stage;
        match self.stage {
            Stage::SelectYearAndDate => {
                self.resize_small(ctx, "ASTER · Financial Year and Date", [800.0, 500.0])
            }
            Stage::FirstRunSetup => {
                self.resize_small(ctx, "ASTER · Administrator Setup", [520.0, 640.0])
            }
            Stage::Login => self.resize_small(ctx, "ASTER · Sign In", [520.0, 560.0]),
            Stage::Main => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                    "ASTER Billing · Workspace".into(),
                ));
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::Vec2::new(
                    MAIN_MIN_SIZE[0],
                    MAIN_MIN_SIZE[1],
                )));
                ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }
            Stage::Error(_) => self.resize_small(ctx, "ASTER · Startup Error", [540.0, 300.0]),
        }
    }

    fn resize_small(&mut self, ctx: &egui::Context, title: &str, size: [f32; 2]) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(false));
        // Reset the minimum first so a previous workspace minimum can't block shrinking.
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::Vec2::new(
            size[0], size[1],
        )));
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.into()));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::Vec2::new(
            size[0], size[1],
        )));
        // Centre once the new size has been applied (retried for a few frames).
        self.pending_center = 8;
        ctx.request_repaint();
    }
}

impl eframe::App for AsterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // The first stage never goes through set_stage, so size/centre it here.
        if !self.layout_ready {
            self.layout_ready = true;
            let stage = self.stage.clone();
            self.set_stage(stage, ctx);
        }
        if self.pending_center > 0 {
            self.pending_center -= 1;
            if let Some(command) = egui::ViewportCommand::center_on_screen(ctx) {
                ctx.send_viewport_cmd(command);
            }
            ctx.request_repaint();
        }
        match self.stage.clone() {
            Stage::SelectYearAndDate => self.show_year_and_date(ctx),
            Stage::FirstRunSetup => self.show_first_run_setup(ctx),
            Stage::Login => self.show_login(ctx),
            Stage::Main => self.show_main(ctx),
            Stage::Error(error) => screens::show_error(ctx, &error),
        }
    }
}
