use crate::{actions, app::ClicksEditorApp};

pub fn display(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    if app.project_file.show.cues.is_empty() {
        return;
    }

    if app.project_file.show.cues[app.selected_cue_idx].beats.len() <= app.selected_beat_idx {
        app.selected_beat_idx = 0
    }

    ui.horizontal(|ui| {
        for category in ["cue", "view", "beat", "reload"] {
            for action_id in actions::actions(category) {
                let action = actions::action(&action_id);
                if action.button_icon(app, ui) {
                    app.project_file.show.cues[app.selected_cue_idx].recalculate_tempo_changes();
                }
            }
            ui.separator();
        }
    });
}
