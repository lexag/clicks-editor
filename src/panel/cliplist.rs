use crate::app::ClicksEditorApp;
use egui::vec2;

pub fn display(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    egui::TopBottomPanel::bottom("clip_buttons")
        .resizable(false)
        .exact_height(50.0)
        .show_inside(ui, |ui| buttons(app, ui));

    let height = 7.0;
    egui::Grid::new("cliplist")
        .striped(true)
        .num_columns(4)
        .spacing(vec2(20.0, height))
        .show(ui, |ui| {
            ui.label("Ch");
            ui.label("Idx");
            ui.label("Length");
            ui.end_row();

            for (i, val) in app.clip_manager.clips.iter().enumerate() {
                let ch_idx = val.0 .0;
                let cl_idx = val.0 .1;
                let clip = val.1;

                ui.label(ch_idx.to_string());
                ui.label(cl_idx.to_string());
                ui.label(format!(
                    "{} min {} sec ({} samples)",
                    clip.length / 48000 / 60,
                    clip.length / 48000 % 60,
                    clip.length,
                ));
                if ui.small_button("Open").clicked() {
                    let _ = open::that(clip.path.clone());
                }
                ui.end_row();
            }
        });
}

pub fn buttons(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui.button("Refresh").clicked() {
            let _ = app.clip_manager.import(app.project_file.path.clone());
        }
    });
}
