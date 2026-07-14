use crate::{actions, app::ClicksEditorApp};

pub fn display(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    ui.horizontal_top(|ui| {
        ui.menu_button("File", |ui| submenu_file(app, ui));
        ui.menu_button("Edit", |ui| submenu_edit(app, ui));
        ui.menu_button("View", |ui| submenu_view(app, ui));
        ui.menu_button("Help", |ui| submenu_help(app, ui));
        if let Some(action) = &app.last_action {
            ui.label(action.name_global.clone());
        }
    });
}

fn submenu_file(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    for act_id in actions::actions("project") {
        actions::action(&act_id).button(app, ui);
    }
    ui.label(app.project_file.show.metadata.name.str());
}

fn submenu_help(_app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    ui.label(format!("Editor version {}", ClicksEditorApp::VERSION));
    ui.label(format!("Common version {}", ks_common_generic::VERSION));
}

fn submenu_view(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    for id in actions::actions("view") {
        actions::action(&id).button(app, ui);
    }
}

fn submenu_edit(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
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
}
