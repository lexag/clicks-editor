use crate::app::ClicksEditorApp;
use egui::vec2;

pub fn display(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    egui::TopBottomPanel::bottom("clip_buttons")
        .resizable(false)
        .exact_height(50.0)
        .show_inside(ui, |ui| buttons(app, ui));

    let height = 7.0;
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("beatlist")
            .striped(true)
            .num_columns(5)
            .spacing(vec2(0.0, height))
            .show(ui, |ui| {
                ui.label("Cue");
                ui.label("#");
                ui.label("Bar");
                ui.label("Beat");
                ui.label("");
                ui.end_row();

                for (cue_idx, cue) in app.project_file.show.cues.iter().enumerate() {
                    for (beat_idx, beat) in cue.get_beats().iter().enumerate() {
                        ui.label(cue_idx.to_string());
                        ui.label(beat_idx.to_string());
                        ui.label(beat.bar_number.to_string());
                        ui.label(beat.count.to_string());
                        //ui.label(
                        //    beat.events
                        //        .iter()
                        //        .map(|e| e.get_name().to_string())
                        //        .collect::<Vec<String>>()
                        //        .join(", "),
                        //);
                        ui.end_row();
                    }
                }
            });
    });
}

pub fn buttons(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {});
}
