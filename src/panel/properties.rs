use crate::{actions, app::ClicksEditorApp, panel::edit_event::edit_event};
use common::{
    event::EventCursor,
    mem::str::StaticString,
};

pub fn display(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    if app.selected_cue_idx >= app.project_file.show.cues.len() {
        return;
    }
    let cue = &mut app.project_file.show.cues[app.selected_cue_idx];

    let mut recalculate_flag = false;
    egui::ScrollArea::horizontal().show(ui, |ui| {
        ui.vertical(|ui| {
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
            if app.selected_beat_idx >= cue.beats.len() {
                return;
            }

            let _beat = &mut cue.beats[app.selected_beat_idx];
            let mut delete_idx: i32 = -1;
            let events_clone = cue.events.clone();
            let _cursor = EventCursor::new(&events_clone);
            if let Some(event) = cue.events.get_mut(app.selected_event_idx as u8) {
                ui.separator();
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        if ui.small_button("X").clicked() {
                            delete_idx = app.selected_event_idx as i32;
                        }
                        ui.label(
                            egui::RichText::new(
                                event
                                    .event
                                    .map(|e| e.get_name().to_string())
                                    .unwrap_or_default(),
                            )
                            .heading(),
                        );
                    });
                    egui::Grid::new("event-properties")
                        .num_columns(2)
                        .show(ui, |ui| {
                            recalculate_flag = recalculate_flag || edit_event(ui, event)
                        });
                });
                if delete_idx > -1 {
                    cue.events.pop(delete_idx as u8);
                }
            }
        });
    });
    if recalculate_flag {
        (actions::action("cue:recalculate_tempo_changes").function)(app)
    }
}
