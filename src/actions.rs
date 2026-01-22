use crate::{
    app::ClicksEditorApp,
};
use common::{
    beat::Beat,
    cue::{Cue, CueMetadata, Show},
    event::{Event, EventDescription, JumpModeChange, JumpRequirement},
    mem::{smpte::TimecodeInstant, str::StaticString},
};
use egui::{Color32, Image, Key, KeyboardShortcut, ModifierNames, Modifiers};

#[derive(Clone)]
pub struct Action {
    pub symbol: char,
    pub name_global: String,
    pub name_concise: String,
    pub icon: String,
    pub function: fn(&mut ClicksEditorApp) -> (),
    pub interactible: fn(&ClicksEditorApp) -> bool,
    pub active: fn(&ClicksEditorApp) -> bool,
    pub hotkey: Option<KeyboardShortcut>,
}

impl Action {
    pub fn run(&self, app: &mut ClicksEditorApp) {
        app.last_action = Some(self.clone());
        (self.function)(app)
    }

    fn hotkey_str(&self) -> String {
        if let Some(hotkey) = self.hotkey {
            hotkey.format(&ModifierNames::NAMES, false)
        } else {
            String::new()
        }
    }

    pub fn button(&self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        let inter = (self.interactible)(app);
        if ui
            .add_enabled(
                inter,
                egui::Button::new(self.name_global.clone()).shortcut_text(self.hotkey_str()),
            )
            .on_hover_cursor(if inter {
                egui::CursorIcon::PointingHand
            } else {
                egui::CursorIcon::NotAllowed
            })
            .on_hover_text_at_pointer(format!(
                "{} ({})",
                self.name_global.clone(),
                self.hotkey_str()
            ))
            .clicked()
        {
            self.run(app);
        };
    }

    pub fn button_concise(&self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        let inter = (self.interactible)(app);
        if ui
            .add_enabled(inter, egui::Button::new(self.name_concise.clone()))
            .on_hover_cursor(if inter {
                egui::CursorIcon::PointingHand
            } else {
                egui::CursorIcon::NotAllowed
            })
            .on_hover_text_at_pointer(format!(
                "{} ({})",
                self.name_global.clone(),
                self.hotkey_str()
            ))
            .clicked()
        {
            self.run(app);
        };
    }

    pub fn button_icon(&self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) -> bool {
        let inter = (self.interactible)(app);
        let style = ui.style_mut();
        style.spacing.button_padding = [0.0; 2].into();
        let col = if (self.active)(app) {
            Color32::DARK_GREEN
        } else {
            Color32::TRANSPARENT
        };
        style.visuals.widgets.inactive.weak_bg_fill = col;
        style.visuals.widgets.hovered.weak_bg_fill = col;
        style.visuals.widgets.active.weak_bg_fill = col;
        if ui
            .add_enabled(
                inter,
                egui::Button::new(egui::RichText::new(self.icon.clone()).font(egui::FontId {
                    size: 24.0,
                    family: egui::FontFamily::Monospace,
                })),
            )
            .on_hover_cursor(if inter {
                egui::CursorIcon::PointingHand
            } else {
                egui::CursorIcon::NotAllowed
            })
            .on_hover_text_at_pointer(format!(
                "{} ({})",
                self.name_global.clone(),
                self.hotkey_str()
            ))
            .clicked()
        {
            self.run(app);
            return true;
        };
        false
    }
}

macro_rules! cue {
    ($app:ident) => {
        $app.project_file.show.cues[$app.selected_cue_idx].clone()
    };
}
macro_rules! beat {
    ($app:ident) => {
        cue!($app).beats[$app.selected_beat_idx].clone()
    };
}
macro_rules! cue_mut {
    ($app:ident) => {
        $app.project_file.show.cues[$app.selected_cue_idx]
    };
}
macro_rules! beat_mut {
    ($app:ident) => {
        cue_mut!($app).beats[$app.selected_beat_idx]
    };
}

macro_rules! bar_length {
    ($app:ident, $num:expr) => {
        cue!($app)
            .beats
            .iter()
            .filter(|b| b.bar_number == $num)
            .count()
    };
}

macro_rules! sel_bar_length {
    ($app:ident) => {
        bar_length!($app, beat!($app).bar_number)
    };
}

macro_rules! has_cue {
    ($app:ident) => {
        $app.selected_cue_idx < $app.project_file.show.cues.len()
    };
}

macro_rules! has_beat {
    ($app:ident) => {
        has_cue!($app) && $app.selected_beat_idx < cue!($app).beats.len()
    };
}

pub fn all_actions() -> Vec<Action> {
    let mut ret = vec![];
    for cat_id in categories() {
        for id in actions(&cat_id) {
            ret.push(action(&id))
        }
    }
    ret
}

pub fn categories() -> Vec<String> {
    ["cue", "view", "beat", "show", "reload", "select", "project"]
        .iter()
        .map(|&s| s.to_string())
        .collect()
}

pub fn actions(category_id: &str) -> Vec<String> {
    match category_id {
        "cue" => vec![
            "cue:add_beat",
            "cue:add_downbeat",
            "cue:add_measure",
            "cue:delete_beat",
            "cue:delete_measure",
            "cue:reorder",
            "cue:add_ci_measure",
        ],
        "view" => vec![
            "view:zoom_in",
            "view:zoom_default",
            "view:zoom_out",
            "view:toggle_proportional_beat_length",
        ],
        "beat" => vec![
            "beat:add_tempo_event",
            "beat:add_gradual_tempo_event",
            "beat:add_rehearsal_event",
            "beat:add_timecode_event",
            "beat:add_jump",
            "beat:add_vamp",
            "beat:add_repeat",
            "beat:add_volta",
            "beat:add_playback_event",
            "beat:add_playback_stop_event",
            "beat:add_pause_event",
        ],
        "show" => vec![
            "show:add_cue",
            "show:delete_cue",
            "show:duplicate_cue",
            "show:move_cue_up",
            "show:move_cue_down",
        ],
        "reload" => vec!["cue:recalculate_tempo_changes", "show:refresh_audio_clips"],
        "select" => vec![
            "select:next_cue",
            "select:previous_cue",
            "select:next_measure",
            "select:previous_measure",
            "select:next_beat",
            "select:previous_beat",
            "select:next_event",
            "select:previous_event",
        ],
        "project" => vec![
            "project:save_file",
            "project:save_file_as",
            "project:load_file",
            "project:import_json",
            "project:export_json",
        ],
        _ => vec![],
    }
    .iter()
    .map(|&s| s.to_string())
    .collect()
}

pub fn action(action_id: &str) -> Action {
    match action_id {
        "cue:add_beat" => Action {
            symbol: '+',
            name_global: "Add beat".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_ADD.to_string(),
            function: |app| {
                if cue!(app).beats.is_empty() {
                    cue_mut!(app).beats.insert(
                        0,
                        Beat {
                            count: 1,
                            bar_number: 1,
                            ..Default::default()
                        },
                    );
                } else {
                    let beat = beat!(app);

                    cue_mut!(app).beats.insert(
                        app.selected_beat_idx + 1,
                        Beat {
                            count: beat.count + 1,
                            bar_number: beat.bar_number,
                            ..Default::default()
                        },
                    );
                    app.selected_beat_idx += 1;
                }
                cue_mut!(app).reorder_numbers();
                (action("cue:recalculate_tempo_changes").function)(app);
            },
            interactible: |app| has_cue!(app),
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::SHIFT,
                logical_key: Key::B,
            }),
        },
        "cue:add_downbeat" => Action {
            symbol: '+',
            name_global: "Add downbeat".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_ADD_CIRCLE.to_string(),
            function: |app| {
                let beat = beat!(app);
                cue_mut!(app).beats.insert(
                    app.selected_beat_idx + 1,
                    Beat {
                        count: 1,
                        bar_number: beat.bar_number + 1,
                        ..Default::default()
                    },
                );
                app.selected_beat_idx += 1;
                cue_mut!(app).reorder_numbers();
                (action("cue:recalculate_tempo_changes").function)(app);
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::SHIFT,
                logical_key: Key::N,
            }),
        },
        "cue:add_measure" => Action {
            symbol: '+',
            name_global: "Add measure".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_ADD_TO_QUEUE.to_string(),
            function: |app| {
                let beat = beat!(app);
                let num_beats = sel_bar_length!(app);
                for i in 0..num_beats {
                    cue_mut!(app).beats.push(
                        Beat {
                            count: (i + 1) as u8,
                            bar_number: beat.bar_number + 1,
                            ..Default::default()
                        },
                    );
                }
                    app.selected_beat_idx = cue!(app).beats.len() - 1;
                cue_mut!(app).reorder_numbers();
                (action("cue:recalculate_tempo_changes").function)(app);
            },
            interactible: |app| has_cue!(app),
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::SHIFT,
                logical_key: Key::M,
            }),
        },
        "cue:delete_beat" => Action {
            symbol: '+',
            name_global: "Delete beat".to_string(),
            name_concise: "Delete".to_string(),
            icon: egui_material_icons::icons::ICON_CLOSE.to_string(),
            function: |app| {
                cue_mut!(app).beats.remove(app.selected_beat_idx);
                app.selected_beat_idx = app.selected_beat_idx.saturating_sub(1);
                cue_mut!(app).reorder_numbers();
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::SHIFT,
                logical_key: Key::Delete,
            }),
        },
        "cue:delete_measure" => Action {
            symbol: '+',
            name_global: "Delete measure".to_string(),
            name_concise: "Delete".to_string(),
            icon: egui_material_icons::icons::ICON_REMOVE_FROM_QUEUE.to_string(),
            function: |app| {
                let beat = beat!(app);
                let len_pre = cue!(app).beats.len();
                let mut beat_vec = cue!(app)
                    .beats
                    .iter()
                    .filter(|b| b.bar_number != beat.bar_number)
                    .cloned()
                    .collect::<Vec<Beat>>();
                app.selected_beat_idx -= len_pre - cue!(app).beats.len();
                cue_mut!(app).beats = beat_vec;
                cue_mut!(app).reorder_numbers();
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::CTRL,
                logical_key: Key::Delete,
            }),
        },
        "cue:reorder" => Action {
            symbol: '+',
            name_global: "Reorder beat and measure numbering".to_string(),
            name_concise: "Reorder".to_string(),
            icon: egui_material_icons::icons::ICON_ROTATE_AUTO.to_string(),
            function: |app| {
                cue_mut!(app).reorder_numbers();
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::CTRL.plus(Modifiers::ALT),
                logical_key: Key::R,
            }),
        },
        "cue:add_ci_measure" => Action {
            symbol: '+',
            name_global: "Add count-in measure".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_TEXT_SELECT_END.to_string(),
            function: |app| {
                let sel_pre = app.selected_beat_idx;
                app.selected_beat_idx = 0;
                let beat = beat!(app);
                if beat.bar_number == 0 {
                    return;
                }
                let num_to_add = sel_bar_length!(app);
                for i in 0..num_to_add {
                    &mut cue_mut!(app).beats.insert(
                        i,
                        Beat {
                            count: i as u8 + 1,
                            bar_number: 0,
                            length: beat.length,
                        },
                    );
                }
                app.selected_beat_idx = sel_pre + num_to_add;
                cue_mut!(app).reorder_numbers();
            },
            interactible: |app| has_beat!(app) && cue!(app).beats[0].bar_number != 0,
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::SHIFT,
                logical_key: Key::I,
            }),
        },
        "cue:recalculate_tempo_changes" => Action {
            symbol: 'v',
            name_global: "Recalculate tempo changes".to_string(),
            name_concise: "Update".to_string(),
            icon: "\u{f37b}".to_string(),
            function: |app| {
                cue_mut!(app).recalculate_tempo_changes();
            },
            interactible: |app| true,
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::CTRL,
                logical_key: Key::R,
            }),
        },
        "view:zoom_in" => Action {
            symbol: '+',
            name_global: "Zoom in".to_string(),
            name_concise: "In".to_string(),
            icon: egui_material_icons::icons::ICON_ZOOM_IN.to_string(),
            function: |app| {
                app.zoom *= 1.1;
            },
            interactible: |app| true,
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::NONE,
                logical_key: Key::Plus,
            }),
        },
        "view:zoom_default" => Action {
            symbol: '+',
            name_global: "Zoom 100%".to_string(),
            name_concise: "100%".to_string(),
            icon: egui_material_icons::icons::ICON_CROP_FREE.to_string(),
            function: |app| {
                app.zoom = 12.0;
            },
            interactible: |app| (app.zoom - 12.0).abs() > 0.00001,
            active: |app| false,
            hotkey: None,
        },
        "view:zoom_out" => Action {
            symbol: '+',
            name_global: "Zoom out".to_string(),
            name_concise: "Out".to_string(),
            icon: egui_material_icons::icons::ICON_ZOOM_OUT.to_string(),
            function: |app| {
                app.zoom *= 0.9090909;
            },
            interactible: |app| true,
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::NONE,
                logical_key: Key::Minus,
            }),
        },
        "view:toggle_proportional_beat_length" => Action {
            symbol: '+',
            name_global: "Toggle proportional beat scaling".to_string(),
            name_concise: "Proportional scaling".to_string(),
            icon: egui_material_icons::icons::ICON_VIEW_REAL_SIZE.to_string(),
            function: |app| {
                app.proportional_beat_length = !app.proportional_beat_length;
            },
            interactible: |app| true,
            active: |app| app.proportional_beat_length,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::CTRL,
                logical_key: Key::Period,
            }),
        },
        "beat:add_tempo_event" => Action {
            symbol: '+',
            name_global: "Add tempo change".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_TIMER.to_string(),
            function: |app| {
                cue_mut!(app).events.push(Event::new(
                    app.selected_beat_idx as u16,
                    EventDescription::TempoChangeEvent { tempo: 120 },
                ));
                cue_mut!(app).recalculate_tempo_changes();
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: None,
        },
        "beat:add_gradual_tempo_event" => Action {
            symbol: '+',
            name_global: "Add gradual tempo change".to_string(),
            name_concise: "Add".to_string(),
            icon: "\u{f377}".to_string(),
            function: |app| {
                let tempo = beat!(app).tempo();
                cue_mut!(app).events.push(Event::new(
                    app.selected_beat_idx as u16,
                    EventDescription::GradualTempoChangeEvent {
                        start_tempo: tempo,
                        end_tempo: 120,
                        length: 4,
                    },
                ));
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: None,
        },
        "beat:add_rehearsal_event" => Action {
            symbol: '+',
            name_global: "Add rehearsal mark".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_TEXT_INCREASE.to_string(),
            function: |app| {
                cue_mut!(app).events.push(Event::new(
                    app.selected_beat_idx as u16,
                    EventDescription::RehearsalMarkEvent {
                        label: StaticString::new("A"),
                    },
                ));
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: None,
        },
        "beat:add_timecode_event" => Action {
            symbol: '+',
            name_global: "Add timecode".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_30FPS_SELECT.to_string(),
            function: |app| {
                cue_mut!(app).events.push(Event::new(
                    app.selected_beat_idx as u16,
                    EventDescription::TimecodeEvent {
                        time: TimecodeInstant::new(25),
                    },
                ));
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: None,
        },
        "beat:add_timecode_stop_event" => Action {
            symbol: '+',
            name_global: "Add timecode stop".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_AUTOFPS_SELECT.to_string(),
            function: |app| {
                cue_mut!(app).events.push(Event::new(
                    app.selected_beat_idx as u16,
                    EventDescription::TimecodeStopEvent,
                ));
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: None,
        },
        "beat:add_jump" => Action {
            symbol: '+',
            name_global: "Add jump".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_STEP.to_string(),
            function: |app| {
                cue_mut!(app).events.push(Event::new(
                    app.selected_beat_idx as u16,
                    EventDescription::JumpEvent {
                        destination: 0,
                        requirement: JumpRequirement::None,
                        when_jumped: JumpModeChange::None,
                        when_passed: JumpModeChange::None,
                    },
                ));
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: None,
        },
        "beat:add_vamp" => Action {
            symbol: '+',
            name_global: "Add vamp".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_REPEAT.to_string(),
            function: |app| {
                cue_mut!(app).events.push(Event::new(
                    app.selected_beat_idx as u16,
                    EventDescription::JumpEvent {
                        destination: 0,
                        requirement: JumpRequirement::JumpModeOn,
                        when_jumped: JumpModeChange::None,
                        when_passed: JumpModeChange::SetOff,
                    },
                ));
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: None,
        },
        "beat:add_repeat" => Action {
            symbol: '+',
            name_global: "Add repeat".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_REPEAT_ONE.to_string(),
            function: |app| {
                cue_mut!(app).events.push(Event::new(
                    app.selected_beat_idx as u16,
                    EventDescription::JumpEvent {
                        destination: 0,
                        requirement: JumpRequirement::JumpModeOn,
                        when_jumped: JumpModeChange::SetOff,
                        when_passed: JumpModeChange::SetOn,
                    },
                ));
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: None,
        },
        "beat:add_volta" => Action {
            symbol: '+',
            name_global: "Add volta".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_STEP_OVER.to_string(),
            function: |app| {
                cue_mut!(app).events.push(Event::new(
                    app.selected_beat_idx as u16,
                    EventDescription::JumpEvent {
                        destination: 0,
                        requirement: JumpRequirement::JumpModeOff,
                        when_jumped: JumpModeChange::SetOn,
                        when_passed: JumpModeChange::None,
                    },
                ));
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: None,
        },
        "beat:add_playback_event" => Action {
            symbol: '+',
            name_global: "Add audio playback".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_VOLUME_UP.to_string(),
            function: |app| {
                cue_mut!(app).events.push(Event::new(
                    app.selected_beat_idx as u16,
                    EventDescription::PlaybackEvent {
                        sample: 0,
                        channel_idx: 0,
                        clip_idx: 0,
                    },
                ));
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: None,
        },
        "beat:add_playback_stop_event" => Action {
            symbol: '+',
            name_global: "Add audio playback stop".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_VOLUME_OFF.to_string(),
            function: |app| {
                cue_mut!(app).events.push(Event::new(
                    app.selected_beat_idx as u16,
                    EventDescription::PlaybackStopEvent { channel_idx: 0 },
                ));
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: None,
        },
        "beat:add_pause_event" => Action {
            symbol: '+',
            name_global: "Add transport pause".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_PAUSE.to_string(),
            function: |app| {
                cue_mut!(app).events.push(Event::new(
                    app.selected_beat_idx as u16,
                    EventDescription::PauseEvent {
                        behaviour: common::event::PauseEventBehaviour::Hold,
                    },
                ));
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: None,
        },
        "show:add_cue" => Action {
            symbol: 'x',
            name_global: "Add cue".to_string(),
            name_concise: "Add".to_string(),
            icon: egui_material_icons::icons::ICON_ADD_BOX.to_string(),
            function: |app| {
                app.project_file
                    .show
                    .cues
                    .insert(app.selected_cue_idx, Cue::empty());
                app.project_file.show.cues[app.selected_cue_idx].metadata = CueMetadata {
                    name: StaticString::new("Unnamed cue"),
                    human_ident: StaticString::new("000"),
                };
            },
            interactible: |app| true,
            active: |app| false,
            hotkey: None,
        },
        "show:delete_cue" => Action {
            symbol: 'x',
            name_global: "Delete cue".to_string(),
            name_concise: "Delete".to_string(),
            icon: egui_material_icons::icons::ICON_DELETE.to_string(),
            function: |app| {
                app.project_file.show.cues.remove(app.selected_cue_idx);
                app.selected_cue_idx = app.selected_cue_idx.saturating_sub(1);
            },
            interactible: |app| !app.project_file.show.cues.is_empty(),
            active: |app| false,
            hotkey: None,
        },
        "show:duplicate_cue" => Action {
            symbol: 'D',
            name_global: "Duplicate cue".to_string(),
            name_concise: "Duplicate".to_string(),
            icon: egui_material_icons::icons::ICON_CONTENT_COPY.to_string(),
            function: |app| {
                if app
                    .project_file
                    .show
                    .cues
                    .last()
                    .expect("show is never empty")
                    .is_null()
                {
                    let cue = app.project_file.show.cues[app.selected_cue_idx].clone();
                    &mut app.project_file.show.cues.insert(app.selected_cue_idx, cue);
                    app.selected_cue_idx += 1;
                }
            },
            interactible: |app| !app.project_file.show.cues.is_empty(),
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::CTRL,
                logical_key: Key::D,
            }),
        },
        "show:move_cue_up" => Action {
            symbol: '^',
            name_global: "Move cue up".to_string(),
            name_concise: "Move up".to_string(),
            icon: egui_material_icons::icons::ICON_MOVE_UP.to_string(),
            function: |app| {
                app.project_file
                    .show
                    .cues
                    .swap(app.selected_cue_idx, app.selected_cue_idx - 1);
                app.selected_cue_idx -= 1;
            },
            interactible: |app| app.selected_cue_idx > 0,
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::SHIFT,
                logical_key: Key::PageUp,
            }),
        },
        "show:move_cue_down" => Action {
            symbol: 'v',
            name_global: "Move cue down".to_string(),
            name_concise: "Move down".to_string(),
            icon: egui_material_icons::icons::ICON_MOVE_DOWN.to_string(),
            function: |app| {
                app.project_file
                    .show
                    .cues
                    .swap(app.selected_cue_idx, app.selected_cue_idx + 1);
                app.selected_cue_idx += 1;
            },
            interactible: |app| app.selected_cue_idx + 1 < app.project_file.show.cues.len(),
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::SHIFT,
                logical_key: Key::PageDown,
            }),
        },
        "show:refresh_audio_clips" => Action {
            symbol: 'v',
            name_global: "Refresh audio clips".to_string(),
            name_concise: "Refresh".to_string(),
            icon: egui_material_icons::icons::ICON_REFRESH.to_string(),
            function: |app| {
                let _ = app.clip_manager.import(app.project_file.path.clone());
            },
            interactible: |app| true,
            active: |app| false,
            hotkey: None,
        },

        "select:next_cue" => Action {
            symbol: 'v',
            name_global: "Select next cue".to_string(),
            name_concise: "Next".to_string(),
            icon: egui_material_icons::icons::ICON_SKIP_NEXT.to_string(),
            function: |app| {
                app.selected_cue_idx += 1;
                app.selected_cue_idx = app
                    .selected_cue_idx
                    .max(0)
                    .min(app.project_file.show.cues.len() - 1);
            },
            interactible: |app| has_cue!(app),
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::NONE,
                logical_key: Key::PageDown,
            }),
        },
        "select:previous_cue" => Action {
            symbol: 'v',
            name_global: "Select previous cue".to_string(),
            name_concise: "Previous".to_string(),
            icon: egui_material_icons::icons::ICON_SKIP_PREVIOUS.to_string(),
            function: |app| {
                app.selected_cue_idx = app.selected_cue_idx.saturating_sub(1);
                app.selected_cue_idx = app
                    .selected_cue_idx
                    .max(0)
                    .min(app.project_file.show.cues.len() - 1);
            },
            interactible: |app| has_cue!(app),
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::NONE,
                logical_key: Key::PageUp,
            }),
        },
        "select:next_measure" => Action {
            symbol: 'v',
            name_global: "Select next measure".to_string(),
            name_concise: "Next".to_string(),
            icon: egui_material_icons::icons::ICON_CHEVRON_FORWARD.to_string(),
            function: |app| {
                let measure = beat!(app).bar_number;
                for _ in 0..36 {
                    if measure == beat!(app).bar_number || beat!(app).count != 1 {
                        (action("select:next_beat").function)(app)
                    } else {
                        break;
                    }
                }
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::SHIFT,
                logical_key: Key::ArrowRight,
            }),
        },
        "select:previous_measure" => Action {
            symbol: 'v',
            name_global: "Select previous measure".to_string(),
            name_concise: "Previous".to_string(),
            icon: egui_material_icons::icons::ICON_CHEVRON_BACKWARD.to_string(),
            function: |app| {
                let measure = beat!(app).bar_number;
                for _ in 0..36 {
                    if measure == beat!(app).bar_number || beat!(app).count != 1 {
                        (action("select:previous_beat").function)(app)
                    } else {
                        break;
                    }
                }
            },
            interactible: |app| has_beat!(app),
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::SHIFT,
                logical_key: Key::ArrowLeft,
            }),
        },
        "select:next_beat" => Action {
            symbol: 'v',
            name_global: "Select next beat".to_string(),
            name_concise: "Next".to_string(),
            icon: egui_material_icons::icons::ICON_ARROW_RIGHT_ALT.to_string(),
            function: |app| {
                app.selected_beat_idx += 1;
                app.selected_beat_idx = app.selected_beat_idx.max(0).min(cue!(app).beats.len() - 1);
            },
            interactible: |app| has_cue!(app),
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::NONE,
                logical_key: Key::ArrowRight,
            }),
        },
        "select:previous_beat" => Action {
            symbol: 'v',
            name_global: "Select previous beat".to_string(),
            name_concise: "Previous".to_string(),
            icon: egui_material_icons::icons::ICON_ARROW_RIGHT_ALT.to_string(),
            function: |app| {
                if app.selected_beat_idx > 0 {
                    app.selected_beat_idx -= 1;
                    app.selected_beat_idx =
                        app.selected_beat_idx.max(0).min(cue!(app).beats.len() - 1);
                }
            },
            interactible: |app| has_cue!(app),
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::NONE,
                logical_key: Key::ArrowLeft,
            }),
        },
        "project:load_file" => Action {
            symbol: 'v',
            name_global: "Load file".to_string(),
            name_concise: "Load".to_string(),
            icon: egui_material_icons::icons::ICON_FOLDER_OPEN.to_string(),
            function: |app| {
                if let Some(dir) = crate::io::pick_dir() {
                    let _ = app.project_file.load(dir);
                    action("show:refresh_audio_clips").run(app);  
                }
            },
            interactible: |app| true,
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::CTRL,
                logical_key: Key::O,
            }),
        },
        "project:save_file" => Action {
            symbol: 'v',
            name_global: "Save file".to_string(),
            name_concise: "Save".to_string(),
            icon: egui_material_icons::icons::ICON_SAVE.to_string(),
            function: |app| {
                if app
                    .project_file
                    .path
                    .to_str()
                    .expect("No stupid paths pls")
                    .is_empty()
                {
                    action("project:save_file_as").run(app);
                } else {
                    let _ = app.project_file.save();
                }
                app.last_action = None;
            },
            interactible: |app| true,
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::CTRL,
                logical_key: Key::S,
            }),
        },
        "project:save_file_as" => Action {
            symbol: 'v',
            name_global: "Save file as".to_string(),
            name_concise: "Save as".to_string(),
            icon: egui_material_icons::icons::ICON_SAVE_AS.to_string(),
            function: |app| {
                if let Some(dir) = crate::io::pick_dir() {
                    let _ = app.project_file.save_as(dir);
                }
                app.last_action = None;
            },
            interactible: |app| true,
            active: |app| false,
            hotkey: Some(KeyboardShortcut {
                modifiers: Modifiers::CTRL | Modifiers::SHIFT,
                logical_key: Key::S,
            }),
        },
        "project:import_json" => Action {
            symbol: 'v',
            name_global: "Import JSON".to_string(),
            name_concise: "Import".to_string(),
            icon: egui_material_icons::icons::ICON_FILE_OPEN.to_string(),
            function: |app| {
    if let Some(dir) = crate::io::pick_file() && let Err(err) = app.project_file.import_json(dir.clone()) {
        crate::io::show_dialog(
            rfd::MessageLevel::Error,
            "Import failed".to_string(),
            err.to_string(),
        );
    }
            },
            interactible: |app| true,
            active: |app| false,
            hotkey: None,
        },
        "project:export_json" => Action {
            symbol: 'v',
            name_global: "Export JSON".to_string(),
            name_concise: "Export".to_string(),
            icon: egui_material_icons::icons::ICON_FILE_SAVE.to_string(),
            function: |app| {
    if let Some(dir) = crate::io::save_file() && let Err(err) = app.project_file.export_json(dir.clone()) {
        crate::io::show_dialog(
            rfd::MessageLevel::Error,
            "Export failed".to_string(),
            err.to_string(),
        );
    }
            },
            interactible: |app| true,
            active: |app| false,
            hotkey: None,
        },
        //"select:next_event" => Action {
        //    symbol: 'v',
        //    name_global: "Select next cue".to_string(),
        //    name_concise: "Next".to_string(),
        //    icon: egui_material_icons::icons::ICON_SKIP_NEXT.to_string(),
        //    function: |app| {
        //        app.selected_cue_idx += 1;
        //        app.selected_cue_idx = app
        //            .selected_cue_idx
        //            .max(0)
        //            .min(app.project_file.show.cues.len());
        //    },
        //    interactible: |app| has_cue!(app),
        //    active: |app| false,
        //    hotkey: Some(KeyboardShortcut {
        //        modifiers: Modifiers::SHIFT,
        //        logical_key: Key::PageDown,
        //    }),
        //},
        //"select:previous_event" => Action {
        //    symbol: 'v',
        //    name_global: "Select next cue".to_string(),
        //    name_concise: "Next".to_string(),
        //    icon: egui_material_icons::icons::ICON_SKIP_NEXT.to_string(),
        //    function: |app| {
        //        app.selected_cue_idx += 1;
        //        app.selected_cue_idx = app
        //            .selected_cue_idx
        //            .max(0)
        //            .min(app.project_file.show.cues.len());
        //    },
        //    interactible: |app| has_cue!(app),
        //    active: |app| false,
        //    hotkey: Some(KeyboardShortcut {
        //        modifiers: Modifiers::SHIFT,
        //        logical_key: Key::PageDown,
        //    }),
        //},
        _ => Action {
            symbol: ' ',
            name_global: "".to_string(),
            name_concise: "".to_string(),
            icon: egui_material_icons::icons::ICON_ADB.to_string(),
            function: |app| {},
            interactible: |app| false,
            active: |app| false,
            hotkey: None,
        },
    }
}
