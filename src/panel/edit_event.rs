use common::{
    event::{Event, EventDescription, JumpModeChange, JumpRequirement, PauseEventBehaviour},
    mem::{
        smpte::{TimecodeInstant, TimecodeProperties, TimecodeUserBitFormat},
        str::StaticString,
    },
};
use egui::{TextEdit, TextStyle};

pub fn edit_event(ui: &mut egui::Ui, event: &mut Event) -> bool {
    let mut recalculate_flag = false;
    match &mut event.event {
        Some(EventDescription::TempoChangeEvent { tempo }) => {
            edit_tempo_change(ui, &mut recalculate_flag, tempo);
        }
        Some(EventDescription::GradualTempoChangeEvent {
            start_tempo,
            end_tempo,
            length,
        }) => {
            edit_gradual_tempo_change(ui, &mut recalculate_flag, start_tempo, end_tempo, length);
        }
        Some(EventDescription::RehearsalMarkEvent { label }) => {
            edit_rehearsal_mark(ui, label);
        }
        Some(EventDescription::TimecodeEvent { time, properties }) => {
            edit_timecode_event(ui, time, properties);
        }
        Some(EventDescription::JumpEvent {
            destination,
            requirement,
            when_jumped,
            when_passed,
        }) => {
            edit_jump_event(ui, destination, requirement, when_jumped, when_passed);
        }
        Some(EventDescription::PlaybackEvent {
            channel_idx,
            clip_idx,
            sample,
        }) => {
            edit_playback_event(ui, channel_idx, clip_idx, sample);
        }
        Some(EventDescription::PlaybackStopEvent { channel_idx }) => {
            edit_playback_stop_event(ui, channel_idx);
        }
        Some(EventDescription::PauseEvent { behaviour }) => {
            edit_pause_event(ui, behaviour);
        }
        _ => {}
    }
    recalculate_flag
}

fn edit_pause_event(ui: &mut egui::Ui, behaviour: &mut PauseEventBehaviour) {
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
                ui.selectable_value(behaviour, val, format!("{}", val));
            }
        });
    ui.end_row();
}

fn edit_playback_stop_event(ui: &mut egui::Ui, channel_idx: &mut u16) {
    ui.label("Channel:");
    ui.add(
        egui::DragValue::new(channel_idx)
            .speed(0.1)
            .max_decimals(0)
            .range(0..=29),
    );
    ui.end_row();
}

fn edit_playback_event(
    ui: &mut egui::Ui,
    channel_idx: &mut u16,
    clip_idx: &mut u16,
    sample: &mut i32,
) {
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

fn edit_jump_event(
    ui: &mut egui::Ui,
    destination: &mut u16,
    requirement: &mut JumpRequirement,
    when_jumped: &mut JumpModeChange,
    when_passed: &mut JumpModeChange,
) {
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
                ui.selectable_value(requirement, val, format!("{}", val));
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
                ui.selectable_value(when_jumped, val, format!("{}", val));
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
                ui.selectable_value(when_passed, val, format!("{}", val));
            }
        });
    ui.end_row();
}

fn edit_timecode_event(
    ui: &mut egui::Ui,
    time: &mut TimecodeInstant,
    properties: &mut TimecodeProperties,
) {
    ui.label("Frame rate");
    ui.add(
        egui::DragValue::new(&mut time.frame_rate)
            .speed(0.1)
            .range(0..=30),
    );
    ui.end_row();
    ui.label("Time:");
    ui.horizontal(|ui| {
        for (val, max, _unit) in [
            (&mut time.h, 29, 'h'),
            (&mut time.m, 59, 'm'),
            (&mut time.s, 59, 's'),
            (&mut time.f, time.frame_rate, 'f'),
        ] {
            ui.add_enabled(
                !properties.use_wall_time,
                egui::DragValue::new(val)
                    .speed(0.1)
                    .custom_formatter(|n, _| format!("{n:02}"))
                    .max_decimals(0)
                    .range(0..=max),
            );
        }
    });
    ui.end_row();

    ui.label("Use wall time");
    ui.checkbox(&mut properties.use_wall_time, "");
    ui.end_row();

    ui.label("User bits");
    ui.end_row();
    ui.label("Format");
    egui::ComboBox::from_id_salt("user-bit-format")
        .selected_text(format!("{:?}", properties.user_bit_format)) // FIXME: user bit format needs
        // to implement Display, but
        // does not currently.
        .show_ui(ui, |ui| {
            for val in [
                TimecodeUserBitFormat::Unspecified,
                TimecodeUserBitFormat::DateTimezone,
                TimecodeUserBitFormat::EightBitLittleEndian,
            ] {
                ui.selectable_value(&mut properties.user_bit_format, val, format!("{:?}", val));
            }
        });
    ui.end_row();

    ui.label("Content");
    match properties.user_bit_format {
        TimecodeUserBitFormat::Unspecified => {
            ui.horizontal(|ui| {
                for i in 0..4 {
                    ui.add(
                        egui::DragValue::new(&mut properties.user_bits[i])
                            .speed(0.1)
                            .hexadecimal(2, false, true)
                            .range(0..=256),
                    );
                }
            });
        }
        TimecodeUserBitFormat::EightBitLittleEndian => {
            let mut text = String::from_utf8_lossy(&properties.user_bits).replace("\0", "");
            ui.add(
                TextEdit::singleline(&mut text)
                    .code_editor()
                    .lock_focus(false)
                    .char_limit(4)
                    .font(TextStyle::Monospace),
            );
            let mut bytes = vec![];
            for c in text.to_string().chars() {
                if c.is_ascii() {
                    let mut b = [0u8; 4];
                    c.encode_utf8(&mut b);
                    bytes.push(b[0])
                }
            }
            bytes.resize(4, 0);
            properties.user_bits.copy_from_slice(&bytes);
        }
        _ => {
            ui.label("(no options available)");
        }
    }
    ui.end_row();

    ui.label("Frame # offset");
    ui.add(
        egui::DragValue::new(&mut properties.frame_offset)
            .speed(0.1)
            .range(0..=30 - time.frame_rate),
    );
    ui.end_row();
    ui.label("Color framing");
    ui.checkbox(&mut properties.color_framing, "");
    ui.end_row();
    ui.label("Drop frame (29.97 fps)");
    if time.frame_rate != 30 {
        properties.drop_frame = false
    }
    ui.add_enabled_ui(time.frame_rate == 30, |ui| {
        ui.checkbox(&mut properties.drop_frame, "");
    });
    ui.end_row();
}

fn edit_rehearsal_mark(ui: &mut egui::Ui, label: &mut StaticString<8>) {
    ui.label("Label:");
    let mut name = label.str().to_string();
    ui.text_edit_singleline(&mut name);
    *label = StaticString::new(&name);
    ui.end_row();
}

fn edit_gradual_tempo_change(
    ui: &mut egui::Ui,
    recalculate_flag: &mut bool,
    start_tempo: &mut u16,
    end_tempo: &mut u16,
    length: &mut u16,
) {
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
        *recalculate_flag = true
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
        *recalculate_flag = true
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
        *recalculate_flag = true;
    }
    ui.end_row();
}

fn edit_tempo_change(ui: &mut egui::Ui, recalculate_flag: &mut bool, tempo: &mut u16) {
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
        *recalculate_flag = true
    }
    ui.end_row();
}
