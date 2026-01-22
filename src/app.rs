use crate::{actions::{self, Action}, clip::ClipManager};
use common::cue::Show;
use egui::{Context, FontFamily};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, default, path::PathBuf};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct ClicksEditorApp {
    pub project_file: ProjectFile,
    #[serde(skip)]
    pub ctx: Context,
    pub selected_cue_idx: usize,
    pub selected_beat_idx: usize,
    pub zoom: f32,
    pub proportional_beat_length: bool,
    pub left_display_select: DisplaySelect,
    #[serde(skip)]
    pub clip_manager: ClipManager,
    #[serde(skip)]
    pub last_action: Option<Action>,
}

#[derive(Default, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ProjectFile {
    pub path: PathBuf,
    #[serde(skip)]
    pub show: Show,
}

#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DisplaySelect {
    #[default]
    Cues,
    Clips,
    Beats,
}

impl ProjectFile {
    pub fn save(&mut self) -> Result<(), std::io::Error> {
        if !self.path.try_exists()? {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidFilename,
                format!("'{:?}' is not a valid save path", self.path),
            ));
        }
        self.save_as(self.path.clone())
    }

    pub fn save_as(&mut self, path: PathBuf) -> Result<(), std::io::Error> {
        if !path
            .parent()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidFilename,
                format!("'{:?}' is not a valid save path", path),
            ))?
            .try_exists()?
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidFilename,
                format!("'{:?}' is not a valid save path", path),
            ));
        }
        self.path = path.clone();

        // Create clicks.show if not already there
        if !path.try_exists()? {
            std::fs::create_dir(&path)?;
        }

        // Serialize show into show.bin
        let res = postcard::to_stdvec::<Show>(&self.show).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path.join("show.bin"), &res)?;

        Ok(())
    }

    pub fn load(&mut self, path: PathBuf) -> Result<(), std::io::Error> {
        if !path.try_exists()? {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidFilename,
                format!("'{:?}' is not a valid path", path),
            ));
        }

        self.path = path.clone();

        let data = &std::fs::read(path.join("show.bin"))?;
        self.show = {
            postcard::from_bytes(data)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
        };

        Ok(())
    }

    pub fn export_json(&mut self, path: PathBuf) -> Result<(), std::io::Error> {
        if !path
            .parent()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidFilename,
                format!("'{:?}' is not a valid save path", path),
            ))?
            .try_exists()?
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidFilename,
                format!("'{:?}' is not a valid save path", path),
            ));
        }
        self.path = path.clone();

        // Serialize show into show.json
        let res = serde_json::to_string::<Show>(&self.show)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, res)?;

        Ok(())
    }

    pub fn import_json(&mut self, path: PathBuf) -> Result<(), std::io::Error> {
        if !path.try_exists()? {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidFilename,
                format!("'{:?}' is not a valid path", path),
            ));
        }

        self.path = path.clone();

        let data = &std::fs::read_to_string(path)?;
        self.show = {
            serde_json::from_str(data)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
        };

        Ok(())
    }
}

impl Default for ClicksEditorApp {
    fn default() -> Self {
        Self {
            ctx: Context::default(),
            project_file: ProjectFile::default(),
            selected_cue_idx: 0,
            selected_beat_idx: 0,
            zoom: 10.0,
            proportional_beat_length: false,
            left_display_select: DisplaySelect::Cues,
            clip_manager: ClipManager::default(),
            last_action: None
        }
    }
}

impl ClicksEditorApp {
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");

    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Box<Self> {
        let mut a: Box<ClicksEditorApp> = if false && let Some(storage) = cc.storage {
            serde_json::from_str(
                &eframe::Storage::get_string(storage, eframe::APP_KEY).unwrap_or_default(),
            )
            .unwrap_or_default()
        } else {
            Box::new(Self::default())
        };
        a.ctx = cc.egui_ctx.clone();
        egui_extras::install_image_loaders(&a.ctx);
        a.setup_custom_fonts(&a.ctx);
        let _ = a.project_file.load(a.project_file.path.clone());

        (actions::action("show:refresh_audio_clips").function)(&mut a);

        a
    }
    fn setup_custom_fonts(&self, ctx: &egui::Context) {
        // Load the font from file
        let font_data =
            include_bytes!("../assets/font/MaterialSymbolsOutlined_48pt-Regular.ttf").to_vec();

        let mut fonts = egui::FontDefinitions::default();

        // Register the custom monospace font under a unique name
        fonts.font_data.insert(
            "mono_custom".to_string(),
            egui::FontData::from_owned(font_data).into(),
        );

        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .push("mono_custom".to_string());

        // Apply the font definitions
        ctx.set_fonts(fonts);
    }

    fn check_hotkeys(&mut self, ui: &mut egui::Ui) {
        for action in actions::all_actions() {
            if let Some(hotkey) = action.hotkey &&
                !ui.ctx().wants_keyboard_input() &&
                    ui
                    .input(|i| i.modifiers == hotkey.modifiers && i.key_pressed(hotkey.logical_key))
                    && (action.interactible)(self)
                {
                    action.run(self)
                }
        }
    }
}

impl eframe::App for ClicksEditorApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        return;
        match rfd::MessageDialog::new()
            .set_title("Unsaved changes")
            .set_description("Would you like to save?")
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show()
        {
            rfd::MessageDialogResult::Yes => {
                crate::io::save(self);
            }
            //FIXME: cancel doesn't really work. oops.
            rfd::MessageDialogResult::Cancel => self
                .ctx
                .send_viewport_cmd(egui::ViewportCommand::CancelClose),
            rfd::MessageDialogResult::No => {}
            _ => {}
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(eframe::APP_KEY, serde_json::to_string(&self).unwrap());
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.check_hotkeys(ui);
            crate::panel::menubar::display(self, ui);
        });

        egui::SidePanel::left("list_panel")
            .resizable(true)
            .default_width(400.0)
            .min_width(400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.left_display_select, DisplaySelect::Cues, "Cues");
                    ui.selectable_value(
                        &mut self.left_display_select,
                        DisplaySelect::Clips,
                        "Clips",
                    );
                    ui.selectable_value(
                        &mut self.left_display_select,
                        DisplaySelect::Beats,
                        "Beats",
                    );
                });
                match self.left_display_select {
                    DisplaySelect::Cues => {
                        crate::panel::cuelist::display(self, ui);
                    }
                    DisplaySelect::Clips => {
                        crate::panel::cliplist::display(self, ui);
                    }
                    DisplaySelect::Beats => {
                        crate::panel::beatlist::display(self, ui);
                    }
                    _ => {}
                }
            });

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            crate::panel::toolbar::display(self, ui);
        });

        egui::TopBottomPanel::bottom("properties_panel")
            .exact_height(200.0)
            .show(ctx, |ui| {
                crate::panel::properties::display(self, ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            crate::panel::timeline::display(self, ui);
        });
    }
}
