use crate::{actions, app::ClicksEditorApp};
use common::{
    event::{
        Event, EventCursor, EventDescription, JumpModeChange, JumpRequirement, PauseEventBehaviour,
    },
    mem::str::StaticString,
};
use std::fmt::Display;

pub fn display(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    if app.selected_cue_idx >= app.project_file.show.cues.len() {
        return;
    }
    let cue = &mut app.project_file.show.cues[app.selected_cue_idx];

    let mut recalculate_flag = false;
    egui::ScrollArea::horizontal().show(ui, |ui| {
        ui.horizontal_top(|ui| {
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

            let beat = &mut cue.beats[app.selected_beat_idx];
            let mut delete_idx: i32 = -1;
            let events_clone = cue.events.clone();
            let mut cursor = EventCursor::new(&events_clone);
            for event_idx in 0..cue.events.len() {
                let event: &mut Event = match cue.events.get_mut(event_idx as u8) {
                    None => break,
                    Some(ev) => ev,
                };
                if event.location != app.selected_beat_idx as u16 {
                    continue;
                }
                ui.separator();
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        if ui.small_button("X").clicked() {
                            delete_idx = event_idx as i32;
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
                    egui::Grid::new(format!("event-properties-{}", event_idx))
                        .num_columns(2)
                        .show(ui, |ui| match &mut event.event {
                            Some(EventDescription::TempoChangeEvent { tempo }) => {
                                ui.label("Tempo:");
                                if ui
                                    .add(
                                        egui::DragValue::new(tempo)
                                            .speed(0.4)
                                            .max_decimals(0)
                                            .suffix(" BPM")
                                            .range(1..=500),
                                    )
                                    .is_pointer_button_down_on()
                                {
                                    recalculate_flag = true
                                }
                                ui.end_row();
                            }
                            Some(EventDescription::GradualTempoChangeEvent {
                                start_tempo,
                                end_tempo,
                                length,
                            }) => {
                                ui.label("Start Tempo:");
                                if ui
                                    .add(
                                        egui::DragValue::new(start_tempo)
                                            .speed(0.4)
                                            .max_decimals(0)
                                            .suffix(" BPM")
                                            .range(1..=500),
                                    )
                                    .is_pointer_button_down_on()
                                {
                                    recalculate_flag = true
                                }
                                ui.end_row();
                                ui.label("End Tempo:");
                                if ui
                                    .add(
                                        egui::DragValue::new(end_tempo)
                                            .speed(0.4)
                                            .max_decimals(0)
                                            .suffix(" BPM")
                                            .range(1..=500),
                                    )
                                    .is_pointer_button_down_on()
                                {
                                    recalculate_flag = true
                                }
                                ui.end_row();
                                ui.label("Length:");
                                if ui
                                    .add(
                                        egui::DragValue::new(length)
                                            .speed(0.4)
                                            .max_decimals(0)
                                            .suffix(" beats")
                                            .range(1..=999),
                                    )
                                    .is_pointer_button_down_on()
                                {
                                    recalculate_flag = true;
                                }
                                ui.end_row();
                            }
                            Some(EventDescription::RehearsalMarkEvent { label }) => {
                                ui.label("Label:");
                                let mut name = label.str().to_string();
                                ui.text_edit_singleline(&mut name);
                                *label = StaticString::new(&name);
                                ui.end_row();
                            }
                            Some(EventDescription::TimecodeEvent { time }) => {
                                ui.label("Time:");
                                ui.horizontal(|ui| {
                                    for (val, max, unit) in [
                                        (&mut time.h, 99, 'h'),
                                        (&mut time.m, 59, 'm'),
                                        (&mut time.s, 59, 's'),
                                        (&mut time.f, 99, 'f'),
                                    ] {
                                        ui.add(
                                            egui::DragValue::new(val)
                                                .speed(0.1)
                                                .custom_formatter(|n, _| format!("{n:02}"))
                                                .max_decimals(0)
                                                .range(0..=max),
                                        );
                                    }
                                });
                                ui.end_row();
                            }
                            Some(EventDescription::JumpEvent {
                                destination,
                                requirement,
                                when_jumped,
                                when_passed,
                            }) => {
                                ui.label("Destination:");
                                ui.add(
                                    egui::DragValue::new(destination)
                                        .speed(0.4)
                                        .max_decimals(0)
                                        .suffix("")
                                        .range(0..=999),
                                );
                                ui.end_row();

                                ui.label("Requirement:");
                                egui::ComboBox::from_id_salt("jumpreq")
                                    .selected_text(format!("{}", requirement))
                                    .show_ui(ui, |ui| {
                                        for val in [
                                            JumpRequirement::None,
                                            JumpRequirement::JumpModeOn,
                                            JumpRequirement::JumpModeOff,
                                        ] {
                                            ui.selectable_value(
                                                requirement,
                                                val.clone(),
                                                format!("{}", val),
                                            );
                                        }
                                    });
                                ui.end_row();

                                ui.label("When jumped:");
                                egui::ComboBox::from_id_salt("when-jumped")
                                    .selected_text(format!("{}", when_jumped))
                                    .show_ui(ui, |ui| {
                                        for val in [
                                            JumpModeChange::None,
                                            JumpModeChange::SetOn,
                                            JumpModeChange::SetOff,
                                            JumpModeChange::Toggle,
                                        ] {
                                            ui.selectable_value(
                                                when_jumped,
                                                val.clone(),
                                                format!("{}", val),
                                            );
                                        }
                                    });
                                ui.end_row();

                                ui.label("When passed:");
                                egui::ComboBox::from_id_salt("when-passed")
                                    .selected_text(format!("{}", when_passed))
                                    .show_ui(ui, |ui| {
                                        for val in [
                                            JumpModeChange::None,
                                            JumpModeChange::SetOn,
                                            JumpModeChange::SetOff,
                                            JumpModeChange::Toggle,
                                        ] {
                                            ui.selectable_value(
                                                when_jumped,
                                                val.clone(),
                                                format!("{}", val),
                                            );
                                        }
                                    });
                                ui.end_row();
                            }
                            Some(EventDescription::PlaybackEvent {
                                channel_idx,
                                clip_idx,
                                sample,
                            }) => {
                                ui.label("Channel:");
                                ui.add(
                                    egui::DragValue::new(channel_idx)
                                        .speed(0.1)
                                        .max_decimals(0)
                                        .range(0..=29),
                                );
                                ui.end_row();
                                ui.label("Clip:");
                                ui.add(
                                    egui::DragValue::new(clip_idx)
                                        .speed(0.1)
                                        .max_decimals(0)
                                        .range(0..=64),
                                );
                                ui.end_row();
                                ui.label("Start offset:");
                                ui.add(
                                    egui::DragValue::new(sample)
                                        .speed(100)
                                        .max_decimals(0)
                                        .suffix(" samples")
                                        .range(0..=usize::MAX),
                                );
                                ui.end_row();
                            }
                            Some(EventDescription::PlaybackStopEvent { channel_idx }) => {
                                ui.label("Channel:");
                                ui.add(
                                    egui::DragValue::new(channel_idx)
                                        .speed(0.1)
                                        .max_decimals(0)
                                        .range(0..=29),
                                );
                                ui.end_row();
                            }
                            Some(EventDescription::PauseEvent { behaviour }) => {
                                ui.label("Behaviour:");
                                egui::ComboBox::from_id_salt("behaviour box")
                                    .selected_text(format!("{}", behaviour))
                                    .show_ui(ui, |ui| {
                                        for val in [
                                            PauseEventBehaviour::Hold,
                                            PauseEventBehaviour::RestartBeat,
                                            PauseEventBehaviour::RestartCue,
                                            PauseEventBehaviour::NextCue,
                                            PauseEventBehaviour::Jump { destination: 0 },
                                        ] {
                                            ui.selectable_value(
                                                behaviour,
                                                val.clone(),
                                                format!("{}", val),
                                            );
                                        }
                                    });
                                ui.end_row();
                            }
                            _ => {}
                        });
                });
            }
            if delete_idx > -1 {
                cue.events.pop(delete_idx as u8);
            }
        });
    });
    if recalculate_flag {
        (actions::action("cue:recalculate_tempo_changes").function)(app)
    }
}
