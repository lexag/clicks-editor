use crate::{actions, app::ClicksEditorApp};
use common::cue::Cue;
use egui::vec2;

pub fn display(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    egui::TopBottomPanel::bottom("cue_buttons")
        .resizable(false)
        .exact_height(50.0)
        .show_inside(ui, |ui| buttons(app, ui));

    let height = 7.0;
    egui::Grid::new("cuelist")
        .striped(true)
        .num_columns(3)
        .spacing(vec2(0.0, height))
        .show(ui, |ui| {
            ui.label("");
            ui.label("Id");
            ui.label("Name");
            ui.end_row();

            for (i, cue) in app.project_file.show.cues.iter().enumerate() {
                let tl = ui.cursor().min;
                ui.label(if i == app.selected_cue_idx { ">" } else { "" });

                ui.label(cue.metadata.human_ident.str());
                ui.label(cue.metadata.name.str());
                let br = ui.cursor().min + vec2(0.0, height);
                ui.end_row();

                let response = ui.interact(
                    egui::Rect::from_min_max(tl, br),
                    ui.make_persistent_id(i),
                    egui::Sense::click(),
                );

                if response.clicked() {
                    app.selected_cue_idx = i;
                }
            }
        });
}

pub fn buttons(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let actions = actions::actions("show");
        let width = ui.available_width() / actions.len() as f32 * 0.9;
        let height = ui.available_height() * 0.9;
        for act_id in actions {
            actions::action(&act_id).button_concise(app, ui);
        }
    });
}
