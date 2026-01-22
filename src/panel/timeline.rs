use crate::{
    app::ClicksEditorApp,
    clip::{Clip, ClipManager},
};
use common::{beat::Beat, cue::Cue, event::{EventCursor, EventDescription, JumpModeChange, JumpRequirement}};
use egui::{pos2, vec2, Align, Align2, Color32, FontId, Pos2, Rect, Response, Stroke};

#[derive(Clone)]
struct RunningClip {
    channel_idx: usize,
    clip_idx: usize,
    sample: i32,
    sample_offset_from_start: i64,
}

struct TimelineRenderer {
    resp: Response,
    rect: Rect,
    cue: Cue,
    head: Pos2,
    base_beat_width: f32,
    proportional_scaling: bool,
    beat_idx: i32,
    time_head: i64,
    running_clips: Vec<RunningClip>,
}

impl TimelineRenderer {
    const LANE_HEIGHT: f32 = 15.0;
    const LANE_BUFFER: f32 = 0.0;
    const TEXT_SIZE: f32 = 12.0;
    const TEXT_BUMP: f32 = 12.0 * 0.2;
    const FONT: FontId = FontId::monospace(Self::TEXT_SIZE);

    fn new(app: &mut ClicksEditorApp, ui: &mut egui::Ui, cue: Cue) -> Self {
        let (rect, resp) = ui.allocate_exact_size(
            vec2(app.zoom * cue.beats.len() as f32, ui.available_height()),
            egui::Sense::click(),
        );
        let mut a = Self {
            rect,
            resp,
            cue,
            head: rect.min,
            time_head: 0,
            base_beat_width: app.zoom,
            proportional_scaling: app.proportional_beat_length,
            beat_idx: -1,
            running_clips: vec![],
        };
        a.head.x -= a.base_beat_width;
        a
    }

    fn head_text(&self) -> Pos2 {
        self.head + vec2(Self::TEXT_BUMP, 0.0)
    }

    fn try_zoom(&self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        if ui.rect_contains_pointer(self.rect) {
            app.zoom *= ui.input(|i| i.zoom_delta());
        }
    }

    fn beat_width(&self) -> f32 {
        if self.beat_idx >= 0 && (self.beat_idx as usize) < self.cue.beats.len() {
            self.beat_width_from_length(self.cue.beats[self.beat_idx as usize].length)
        } else {
            self.base_beat_width
        }
    }

    fn beat_width_from_length(&self, length: u32) -> f32 {
        if self.proportional_scaling {
            self.base_beat_width * length as f32 / 500000.0
        } else {
            self.base_beat_width
        }
    }

    fn head_at_idx(&self, index: usize) -> Pos2 {
        let mut head = self.rect.min.x;
        for beat in &self.cue.beats[0..index] {
            head += self.beat_width_from_length(beat.length);
        }
        pos2(head, self.head.y)
    }

    fn next_lane(&mut self) {
        self.head.x = self.rect.min.x - self.base_beat_width;
        self.time_head = 0 - self.cue.get_beat(0).unwrap_or_default().length as i64;
        self.head.y += Self::LANE_HEIGHT + Self::LANE_BUFFER;
        self.beat_idx = -1;
    }

    fn next_beat(&mut self) -> Option<Beat> {
        self.head.x += self.beat_width();
        self.beat_idx += 1;
        let beat = self.cue.get_beat(self.beat_idx as u16)?;
        self.time_head += beat.length as i64;
        Some(beat)
    }

    fn bar_numbers(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        self.blockout_lane(app, ui);
        let p = ui.painter();
        while let Some(beat) = self.next_beat() {
            if beat.count == 1 {
                p.text(
                    self.head_text(),
                    Align2::LEFT_TOP,
                    beat.bar_number.to_string(),
                    Self::FONT,
                    Color32::GRAY,
                );
            }
        }
    }

    fn timecode(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        self.blockout_lane(app, ui);
        let p = ui.painter();
        let events = self.cue.events.clone();
        let mut cursor = EventCursor::new(&events);
        while let Some(_beat) = self.next_beat() {
            while cursor.at_or_before(self.beat_idx as u16) && let Some(event) = cursor.get_next() {
                if let Some(EventDescription::TimecodeEvent { time }) = event.event {
                    p.rect_filled(
                        Rect::from_min_size(
                            self.head,
                            vec2(Self::TEXT_SIZE * 7.0, Self::TEXT_SIZE),
                        ),
                        0.0,
                        Color32::BLACK,
                    );
                    p.text(
                        self.head_text(),
                        Align2::LEFT_TOP,
                        time.to_string(),
                        Self::FONT,
                        Color32::WHITE,
                    );
                }
            }
        }
    }

    fn tempo(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        self.blockout_lane(app, ui);
        let p = ui.painter();
        let mut i: usize = 0;
        let events = self.cue.events.clone();
        let mut cursor = EventCursor::new(&events);
        while let Some(_beat) = self.next_beat() {
            while cursor.at_or_before(self.beat_idx as u16) && let Some(event) = cursor.get_next() {
                if let Some(EventDescription::TempoChangeEvent { tempo }) = event.event {
                    p.text(
                        self.head_text(),
                        Align2::LEFT_TOP,
                        tempo,
                        Self::FONT,
                        Color32::YELLOW,
                    );
                } else if let Some(EventDescription::GradualTempoChangeEvent {
                    start_tempo,
                    end_tempo,
                    length,
                }) = event.event
                {
                    p.text(
                        self.head_text(),
                        Align2::LEFT_TOP,
                        start_tempo,
                        Self::FONT,
                        Color32::YELLOW,
                    );
                    let mut line_length = 0.0;
                    //line_length -= Self::TEXT_SIZE * 2.5;
                    for beat_forward in
                        &app.project_file.show.cues[app.selected_cue_idx].beats[i..i + length as usize]
                    {
                        line_length += self.beat_width_from_length(beat_forward.length);
                    }
                    p.line_segment(
                        [
                            self.head + vec2(Self::TEXT_SIZE * 2.5, Self::LANE_HEIGHT / 2.0),
                            self.head + vec2(line_length, Self::LANE_HEIGHT / 2.0),
                        ],
                        Stroke::new(2.0, Color32::YELLOW),
                    );
                    p.text(
                        self.head_text() + vec2(line_length, 0.0),
                        Align2::LEFT_TOP,
                        end_tempo,
                        Self::FONT,
                        Color32::YELLOW,
                    );
                }
            }
            i += 1;
        }
    }

    fn rehearsal_marks(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        self.blockout_lane(app, ui);
        let p = ui.painter();
        let events = self.cue.events.clone();
        let mut cursor = EventCursor::new(&events);
        while let Some(_beat) = self.next_beat() {
            while cursor.at_or_before(self.beat_idx as u16) && let Some(event) = cursor.get_next() {
                if let Some(EventDescription::RehearsalMarkEvent { label }) = event.event {
                    p.text(
                        self.head_text(),
                        Align2::LEFT_TOP,
                        label.str(),
                        Self::FONT,
                        Color32::RED,
                    );
                    p.line_segment(
                        [self.head, self.head + vec2(0.0, Self::LANE_HEIGHT)],
                        Stroke::new(2.0, Color32::RED),
                    );
                }
            }
        }
    }

    fn jumps(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        self.blockout_lane(app, ui);
        let p = ui.painter();
        let events = self.cue.events.clone();
        let mut cursor = EventCursor::new(&events);
        while let Some(_beat) = self.next_beat() {
            while cursor.at_or_before(self.beat_idx as u16) && let Some(event) = cursor.get_next() {
                match event.event {
                    Some(EventDescription::JumpEvent {
                        destination,
                        requirement,
                        when_jumped,
                        when_passed,
                    }) => {
                        p.text(
                            self.head,
                            Align2::LEFT_TOP,
                            match (requirement, when_jumped, when_passed) {
                                (JumpRequirement::JumpModeOff, _, _) => {
                                    // Volta
                                    egui_material_icons::icons::ICON_STEP_OVER
                                }
                                (_, JumpModeChange::SetOff, _) => {
                                    // Repeat
                                    egui_material_icons::icons::ICON_REPEAT_ONE
                                }
                                (JumpRequirement::JumpModeOn, _, _) => {
                                    // Repeat
                                    egui_material_icons::icons::ICON_REPEAT
                                }
                                _ => egui_material_icons::icons::ICON_STEP_OUT,
                            },
                            FontId {
                                size: Self::FONT.size * 1.1,
                                family: egui::FontFamily::Monospace,
                            },
                            Color32::YELLOW,
                        );
                        p.text(
                            self.head_at_idx(destination as usize),
                            Align2::LEFT_TOP,
                            egui_material_icons::icons::ICON_STEP_INTO,
                            FontId {
                                size: Self::FONT.size * 1.1,
                                family: egui::FontFamily::Monospace,
                            },
                            Color32::YELLOW,
                        );
                    }
                    Some(EventDescription::PauseEvent { behaviour: _ }) => {
                        p.text(
                            self.head,
                            Align2::LEFT_TOP,
                            egui_material_icons::icons::ICON_PAUSE,
                            FontId {
                                size: Self::FONT.size * 1.1,
                                family: egui::FontFamily::Monospace,
                            },
                            Color32::YELLOW,
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    fn playbacks(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        let clip_height = Self::LANE_HEIGHT * 2.4;

        let p = ui.painter();
        let events = self.cue.events.clone();
        let mut cursor = EventCursor::new(&events);
        while let Some(beat) = self.next_beat() {
            while cursor.at_or_before(self.beat_idx as u16) && let Some(event) = cursor.get_next() {
            // Events, i.e. playback start or playback stop get triggered once at the beat they
            // occur
                if let Some(EventDescription::PlaybackEvent {
                    channel_idx,
                    clip_idx,
                    sample,
                }) = event.event
                {
                    self.running_clips.push(RunningClip {
                        channel_idx: channel_idx.into(),
                        clip_idx: clip_idx.into(),
                        sample,
                        sample_offset_from_start: self.time_head * 48 / 1000,
                    });
                    p.line(
                        vec![
                            self.head + vec2(0.0, channel_idx as f32 * clip_height),
                            self.head + vec2(0.0, (channel_idx + 1) as f32 * clip_height),
                        ],
                        Stroke::new(3.0, Color32::GREEN),
                    );
                } else if let Some(EventDescription::PlaybackStopEvent {
                    channel_idx: stop_channel_idx,
                }) = event.event
                {
                    self.running_clips
                        .retain(|e| e.channel_idx != stop_channel_idx as usize);
                    p.line(
                        vec![
                            self.head + vec2(0.0, stop_channel_idx as f32 * clip_height),
                            self.head + vec2(0.0, (stop_channel_idx + 1) as f32 * clip_height),
                        ],
                        Stroke::new(3.0, Color32::DARK_RED),
                    );
                }
            }

            // Clips running get triggered every beat until they end in a playback stop event, or
            // until the end of the cue
            for clip in self.running_clips.clone() {
                let beat_width = self.beat_width();
                p.rect_filled(
                    Rect::from_min_max(
                        self.head + vec2(0.0, clip.channel_idx as f32 * clip_height),
                        self.head + vec2(beat_width, (clip.channel_idx + 1) as f32 * clip_height),
                    ),
                    0.0,
                    Color32::BLUE.gamma_multiply(0.5),
                );

                // Waveform
                let sample_head =
                    self.time_head * 48 / 1000 - clip.sample_offset_from_start + clip.sample as i64;
                let sample_len = beat.length as i64 * 48 / 1000;
                let start_bucket = sample_head / ClipManager::PEAK_BUCKET_SIZE as i64;
                let end_bucket = (sample_head + sample_len) / ClipManager::PEAK_BUCKET_SIZE as i64;
                let bucket_width =
                    beat_width / (sample_len / ClipManager::PEAK_BUCKET_SIZE as i64) as f32;
                for bucket_idx in start_bucket..end_bucket {
                    let bucket_head = self.head
                        + vec2(
                            bucket_width * (bucket_idx - start_bucket) as f32,
                            clip.channel_idx as f32 * clip_height,
                        );

                    let bucket_val = match app
                        .clip_manager
                        .clips
                        .get(&(clip.channel_idx, clip.clip_idx))
                    {
                        Some(clip) => {
                            if (bucket_idx as usize) < clip.peak_buckets.len() {
                                clip.peak_buckets[bucket_idx as usize]
                            } else {
                                0.0
                            }
                        }
                        None => 0.0,
                    };
                    let center_y = vec2(0.0, clip_height / 2.0);
                    let height_push = clip_height / 2.0 * bucket_val.abs();
                    p.line_segment(
                        [
                            bucket_head + center_y - vec2(0.0, height_push),
                            bucket_head + center_y + vec2(0.0, height_push),
                        ],
                        Stroke::new(bucket_width.ceil(), Color32::WHITE),
                    );
                }
            }
        }
    }

    fn background(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        let sel_beat = self.cue.beats[app.selected_beat_idx].clone();
        let p = ui.painter();

        let mut beat_rect = Rect::from_min_max(self.rect.min, self.rect.min);
        for (i, beat) in self.cue.beats.iter().enumerate() {
            if beat.is_null() {
                break;
            }
            beat_rect.max =
                beat_rect.min + vec2(self.beat_width_from_length(beat.length), self.rect.height());

            // Selected beat marker
            if app.selected_beat_idx == i {
                p.rect_filled(beat_rect, 0.0, Color32::DARK_GREEN);
            }

            // Selected measure marker
            if beat.bar_number == sel_beat.bar_number {
                ui.scroll_to_rect(beat_rect, None);
                p.rect_filled(beat_rect, 0.0, Color32::DARK_GREEN.gamma_multiply(0.5));
            }

            // CI measure marker
            if beat.bar_number == 0 {
                p.rect_filled(beat_rect, 0.0, Color32::DARK_RED.gamma_multiply(0.5));
            }

            // Hovered beat marker
            if ui.rect_contains_pointer(beat_rect) {
                p.rect_filled(beat_rect, 0.0, Color32::GRAY.gamma_multiply(0.2));
                if self.resp.clicked() {
                    app.selected_beat_idx = i;
                }
            }

            // Downbeat line and text
            if beat.count == 1 {
                p.line_segment(
                    [beat_rect.left_top(), beat_rect.left_bottom()],
                    Stroke::new(1.0, Color32::GRAY),
                );
            }
            // Other beats
            else {
                p.line_segment(
                    [beat_rect.left_top(), beat_rect.left_bottom()],
                    Stroke::new(1.0, Color32::DARK_GRAY),
                );
            }
            beat_rect.min.x = beat_rect.max.x;
        }
    }

    fn blockout_lane(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        let p = ui.painter();

        p.rect_filled(
            Rect::from_min_max(
                self.head,
                pos2(self.rect.max.x, self.head.y + Self::LANE_HEIGHT),
            ),
            0,
            ui.style().visuals.window_fill().gamma_multiply(0.5),
        );
    }
}

pub fn display(app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    if app.project_file.show.cues.is_empty() {
        ui.label("Create a cue to start editing.");
        return;
    }

    let cue = app.project_file.show.cues[app.selected_cue_idx].clone();
    if cue.beats.is_empty() {
        ui.label("Insert a beat using the toolbar.");
        return;
    }

    egui::ScrollArea::horizontal().show(ui, |ui| {
        let mut tlr = TimelineRenderer::new(app, ui, cue);
        tlr.background(app, ui);

        tlr.jumps(app, ui);
        tlr.next_lane();
        tlr.bar_numbers(app, ui);
        tlr.next_lane();
        tlr.timecode(app, ui);
        tlr.next_lane();
        tlr.tempo(app, ui);
        tlr.next_lane();
        tlr.rehearsal_marks(app, ui);
        tlr.next_lane();
        tlr.playbacks(app, ui);

        tlr.try_zoom(app, ui);
    });
}
