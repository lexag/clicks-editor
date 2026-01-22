use crate::{actions, app::ClicksEditorApp};

pub fn display(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    ui.horizontal_top(|ui| {
        ui.menu_button("File", |ui| {
            for act_id in actions::actions("project") {
                actions::action(&act_id).button(app, ui);
            }
            ui.label(app.project_file.show.metadata.name.str())
        });
        ui.menu_button("Edit", |ui| {
            for (name, category) in [
                ("Show", "show"),
                ("Cue", "cue"),
                ("Beat", "beat"),
                ("Select", "select"),
            ] {
                ui.menu_button(name, |ui| {
                    for act_id in actions::actions(category) {
                        actions::action(&act_id).button(app, ui);
                    }
                });
            }
        });
        ui.menu_button("View", |ui| {
            for id in actions::actions("view") {
                actions::action(&id).button(app, ui);
            }
        });
        ui.menu_button("Help", |ui| {
            ui.label(format!("Editor version {}", ClicksEditorApp::VERSION));
            ui.label(format!("Common version {}", common::VERSION));
        });
        if let Some(action) = &app.last_action {
            ui.label(action.name_global.clone());
        }
    });
}
