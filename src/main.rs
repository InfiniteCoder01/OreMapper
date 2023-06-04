#![allow(non_snake_case)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod lib {
    pub mod egui_image;
    pub mod image_draw;
    pub mod math;
    pub mod more_ui;
}

mod project;
mod view {
    pub mod atlas_view;
    pub mod content_view;
    pub mod editor_view;
    pub mod inspector_view;
}

use project::*;

struct Application {
    assets: Assets,
    status: RichText,
}

impl Application {
    pub fn new() -> Self {
        Self {
            assets: Assets::load(&std::path::PathBuf::from(
                "/mnt/Dev/Arduino/Projects/GameBoyStory/res/Mario/Project/",
            ))
            .unwrap(), // TODO: open project and last project
            status: RichText::new("Ready"),
        }
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Main views
        // Menu
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        if let Err(err) = self.assets.save() {
                            self.status = RichText::new(err.to_string()).color(Color32::RED);
                        }
                    }
                });
            })
        });
        // Status bar
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| ui.label(self.status.clone()));
        CentralPanel::default().show(ctx, |ui| {
            TopBottomPanel::bottom("bottom_panel")
                .resizable(true)
                .show_inside(ui, |ui| {
                    SidePanel::left("content_view")
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            if let Err(err) = view::content_view::show(ui, &mut self.assets) {
                                self.status = RichText::new(err.to_string()).color(Color32::RED);
                            }
                        });
                    view::atlas_view::show(ui, &mut self.assets);
                });

            SidePanel::left("inspector_panel")
                .resizable(true)
                .show_inside(ui, |ui| view::inspector_view::show(ui, &mut self.assets));

            if let Err(err) = view::editor_view::show(ui, &mut self.assets, &mut self.status) {
                self.status = RichText::new(err.to_string()).color(Color32::RED);
            }
        });

        // Keys
        if ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::S))
        }) {
            if let Err(err) = self.assets.save() {
                self.status = RichText::new(err.to_string()).color(Color32::RED);
            }
        }
        if ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::E))
        }) {
            if let Err(err) = self.assets.export() {
                self.status = RichText::new(err.to_string()).color(Color32::RED);
            }
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    eframe::run_native(
        "OreMapper",
        Default::default(),
        Box::new(|_cc| Box::new(Application::new())),
    )
}
