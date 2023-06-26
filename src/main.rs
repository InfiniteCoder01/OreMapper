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
    assets: Option<Assets>,
    status: RichText,

    #[cfg(feature = "egui_file")]
    open_dialog: Option<egui_file::FileDialog>,
}

impl Application {
    pub fn new() -> Self {
        Self {
            assets: None,
            status: RichText::new("Ready"),
            #[cfg(feature = "egui_file")]
            open_dialog: None,
        }
    }

    #[cfg(feature = "egui_file")]
    pub fn open_project(&mut self) -> Result<()> {
        let mut dialog = egui_file::FileDialog::select_folder(None);
        dialog.open();
        self.open_dialog = Some(dialog);
        Ok(())
    }

    #[cfg(not(feature = "egui_file"))]
    pub fn open_project(&mut self) -> Result<()> {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            let assets = Assets::load(&path)?;
            if let Some(assets) = &self.assets {
                assets.save()?;
            }
            self.assets = Some(assets);
            Ok(())
        } else {
            Ok(())
        }
    }

    pub fn reload_project(&mut self) -> Result<()> {
        if let Some(assets) = &mut self.assets {
            assets.save()?;
            let mut new_assets = Assets::load(&assets.path)?;
            new_assets.content_viewer_path = assets.content_viewer_path.clone();
            new_assets.atlas_selected = assets.atlas_selected.clone();
            new_assets.map_selected = assets.map_selected.clone();
            new_assets.object_selected = assets.object_selected;
            new_assets.component_selected = assets.component_selected.clone();
            self.assets = Some(new_assets);
            Ok(())
        } else {
            bail!("[PROBABLY A BUG] Reloading, when no project is selected!");
        }
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        macro_rules! try_or_status {
            ($expr: expr) => {
                if let Err(err) = $expr {
                    self.status = RichText::new(format!("{}", err)).color(Color32::RED);
                    println!("{}", err);
                }
            };
        }

        // Menu
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open (Ctrl+O)").clicked() {
                        try_or_status!(self.open_project());
                    }
                    if let Some(assets) = &mut self.assets {
                        if ui.button("Save (Ctrl+S)").clicked() {
                            try_or_status!(assets.save());
                        }
                        if ui.button("Export (Ctrl+E)").clicked() {
                            try_or_status!(assets.export());
                        }
                        if ui.button("Reload (F5)").clicked() {
                            try_or_status!(self.reload_project());
                        }
                    }
                });
            })
        });
        // Status bar
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| ui.label(self.status.clone()));

        if let Some(assets) = &mut self.assets {
            // Main views
            CentralPanel::default().show(ctx, |ui| {
                TopBottomPanel::bottom("bottom_panel")
                    .resizable(true)
                    .show_inside(ui, |ui| {
                        SidePanel::left("content_view")
                            .resizable(true)
                            .show_inside(ui, |ui| {
                                try_or_status!(view::content_view::show(ui, assets));
                            });
                        view::atlas_view::show(ui, assets);
                    });

                SidePanel::left("inspector_panel")
                    .resizable(true)
                    .show_inside(ui, |ui| view::inspector_view::show(ui, assets));

                try_or_status!(view::editor_view::show(ui, assets, &mut self.status));
            });

            // Keys
            if ctx.input_mut(|input| {
                input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::S))
            }) {
                try_or_status!(assets.save());
            }

            if ctx.input_mut(|input| {
                input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::E))
            }) {
                try_or_status!(assets.export());
            }

            if ctx.input_mut(|input| {
                input.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::F5))
            }) {
                try_or_status!(self.reload_project());
            }
        } else {
            CentralPanel::default().show(ctx, |ui| {
                ui.heading("No project is opened. Ctrl+O or File->Open to open a project.");
            });
        }

        if ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::O))
        }) {
            try_or_status!(self.open_project());
        }

        #[cfg(feature = "egui_file")]
        if let Some(dialog) = &mut self.open_dialog {
            if dialog.show(ctx).selected() {
                if let Some(path) = dialog.path() {
                    let assets = Assets::load(&path);
                    if let Ok(assets) = assets {
                        if let Some(assets) = &self.assets {
                            try_or_status!(assets.save());
                        }
                        self.assets = Some(assets);
                    } else {
                        try_or_status!(assets);
                    }
                    self.open_dialog = None;
                }
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
