use crate::{actions, app::ClicksEditorApp, panel::edit_event::edit_event};
use common::mem::str::StaticString;

pub fn display(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    if app.selected_cue_idx >= app.project_file.show.cues.len() {
        return;
    }

    egui::ScrollArea::horizontal().show(ui, |ui| {
        ui.vertical(|ui| {
            let cue = &mut app.project_file.show.cues[app.selected_cue_idx];
            edit_cue_properties(ui, cue);
            if app.selected_beat_idx >= cue.beats.len() {
                return;
            }

            let Ok(event_idx) = u8::try_from(app.selected_event_idx) else {
                return;
            };

            let mut action = None;
            if let Some(event) = cue.events.get_mut(event_idx) {
                ui.separator();
                action = edit_event(ui, event);
            }
            if let Some(action) = action {
                action.run(app);
            }
        });
    });
}

fn edit_cue_properties(ui: &mut egui::Ui, cue: &mut common::cue::Cue) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "Cue details: {} ({})",
                cue.metadata.name.str(),
                cue.metadata.human_ident.str()
            ))
            .heading(),
        );
        egui::Grid::new("cue-properties")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Name:");
                let mut name = cue.metadata.name.str().to_string();
                ui.text_edit_singleline(&mut name);
                cue.metadata.name = StaticString::new(&name);
                ui.end_row();
                ui.label("Identifier:");
                let mut ident = cue.metadata.human_ident.str().to_string();
                ui.text_edit_singleline(&mut ident);
                cue.metadata.human_ident = StaticString::new(&ident);
                ui.end_row();
            });
    });
}
