use crate::app::ClicksEditorApp;
use rfd::MessageLevel;
use std::path::PathBuf;

pub fn pick_dir() -> Option<PathBuf> {
    if let Some(path) = rfd::FileDialog::new().pick_folder() {
        let mut pbuf: PathBuf = path;
        if pbuf.file_name().expect("no stupid names pls") != "clicks.show" {
            pbuf = pbuf.join("clicks.show");
        }
        return Some(pbuf);
    }
    None
}

pub fn save_file() -> Option<PathBuf> {
    if let Some(path) = rfd::FileDialog::new().save_file() {
        let pbuf: PathBuf = path;
        return Some(pbuf);
    }
    None
}

pub fn pick_file() -> Option<PathBuf> {
    if let Some(path) = rfd::FileDialog::new().pick_file() {
        let pbuf: PathBuf = path;
        return Some(pbuf);
    }
    None
}

pub fn show_dialog(level: MessageLevel, title: String, content: String) {
    rfd::MessageDialog::new()
        .set_level(level)
        .set_title(title)
        .set_buttons(rfd::MessageButtons::Ok)
        .set_description(content)
        .show();
}

pub fn save(app: &mut ClicksEditorApp) {
    if app
        .project_file
        .path
        .to_str()
        .expect("No stupid paths pls")
        .is_empty()
    {
        save_as(app);
    } else {
        let _ = app.project_file.save();
    }
}

pub fn save_as(app: &mut ClicksEditorApp) {
    if let Some(dir) = pick_dir() {
        let _ = app.project_file.save_as(dir);
    }
}

pub fn load(app: &mut ClicksEditorApp) {
    if let Some(dir) = pick_dir() {
        let _ = app.project_file.load(dir);
    }
}

pub fn export_json(app: &mut ClicksEditorApp) {
    if let Some(dir) = save_file() && let Err(err) = app.project_file.export_json(dir.clone()) {
        show_dialog(
            MessageLevel::Error,
            "Export failed".to_string(),
            err.to_string(),
        );
    }
}

pub fn import_json(app: &mut ClicksEditorApp) {
    if let Some(dir) = pick_file() && let Err(err) = app.project_file.import_json(dir.clone()) {
        show_dialog(
            MessageLevel::Error,
            "Import failed".to_string(),
            err.to_string(),
        );
    }
}
