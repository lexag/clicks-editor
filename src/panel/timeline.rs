use crate::app::ClicksEditorApp;
use common::{
    cue::Cue,
    event::{EventDescription, JumpModeChange, JumpRequirement},
};
use egui::{
    Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Response, Shape, Stroke, Style,
    TextWrapMode, Vec2, Visuals, lerp, pos2, vec2,
};

#[derive(Clone)]
struct RunningClip {
    channel_idx: usize,
    clip_idx: usize,
    sample: i32,
    sample_offset_from_start: i64,
}

const NUM_LANES: usize = 35;

// rehearsal marks
// bar.beat ruler
// tempo changes
// jumps/vamps
// LTC ruler
// playback x30

struct TimelinePersistent {
    lane_heights: Vec<(f32, f32)>,
    lane_collapsed: Vec<bool>,
}

impl Default for TimelinePersistent {
    fn default() -> Self {
        Self {
            lane_heights: vec![(20.0, 100.0); NUM_LANES],
            lane_collapsed: vec![true; NUM_LANES],
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum TextFit {
    Truncate,
    Shrink,
    Hide,
    Ignore,
}

struct TimelineRenderer {
    style: Visuals,
    resp: Response,
    painter: Painter,
    cue: Cue,
    regions: Vec<(u16, u16, String)>,
    pan: Vec2,
    base_beat_width: f32,
    proportional_scaling: f32,
    beat_idx: i32,
    time_head: i64,
    running_clips: Vec<RunningClip>,
    persistent: TimelinePersistent,
}

impl TimelineRenderer {
    const LANE_HEIGHT: f32 = 15.0;
    const LANE_BUFFER: f32 = 0.0;
    const TEXT_SIZE: f32 = 12.0;
    const TEXT_BUMP: f32 = 12.0 * 0.2;
    const FONT: FontId = FontId::monospace(Self::TEXT_SIZE);

    fn new(
        app: &mut ClicksEditorApp,
        ui: &mut egui::Ui,
        cue: Cue,
        persistent: TimelinePersistent,
        proportional_scaling: f32,
    ) -> Self {
        let (resp, p) = ui.allocate_painter(ui.available_size(), egui::Sense::click());
        Self {
            painter: p,
            regions: Self::calculate_regions(&cue),
            resp,
            cue,
            pan: app.pan,
            time_head: 0,
            base_beat_width: app.zoom,
            proportional_scaling,
            beat_idx: -1,
            running_clips: vec![],
            persistent,
            style: ui.style().visuals.clone(),
        }
    }

    fn calculate_regions(cue: &Cue) -> Vec<(u16, u16, String)> {
        let mut regions: Vec<(u16, u16, String)> = vec![];
        for event in cue.events.iter() {
            if let Some(EventDescription::RehearsalMarkEvent { label }) = event.event {
                if let Some(last) = regions.last_mut() {
                    last.1 = event.location.saturating_sub(1);
                }
                regions.push((event.location, 0, label.str().to_string()));
            }
        }
        regions
    }

    fn calculate_region_for_beat(&self, beat: usize) -> Option<(u16, u16, String)> {
        for region in &self.regions {
            if ((region.0 as usize) < beat) && (beat < region.1 as usize) {
                return Some(region.clone());
            }
        }
        None
    }

    fn calculate_region_rect(&self, beat: usize) -> Rect {
        if let Some(region) = self.calculate_region_for_beat(beat) {
            return Rect::from_min_max(
                pos2(self.x(region.0 as usize), self.top()),
                pos2(self.x_end(region.1 as usize), self.bottom()),
            );
        }
        self.resp.rect
    }

    fn last_beat(&self) -> usize {
        self.cue.beats.len().saturating_sub(1)
    }

    fn x_size(&self, idx: usize) -> f32 {
        self.beat_width_from_length(self.cue.beats[idx].length)
    }
    fn x(&self, idx: usize) -> f32 {
        let mut offs = 0.0;
        for beat in &self.cue.beats[0..idx] {
            offs += self.beat_width_from_length(beat.length);
        }
        self.resp.rect.min.x + offs - self.pan.x
    }
    fn x_mid(&self, idx: usize) -> f32 {
        self.x(idx) + self.x_size(idx) * 0.5
    }
    fn x_end(&self, idx: usize) -> f32 {
        self.x(idx) + self.x_size(idx)
    }

    fn y_size(&self, idx: usize) -> f32 {
        let pair = self.persistent.lane_heights[idx];
        if self.persistent.lane_collapsed[idx] {
            pair.0
        } else {
            pair.1
        }
    }
    fn y(&self, lane: usize) -> f32 {
        let mut y = 0.0;
        for i in 0..lane {
            y += self.y_size(i)
        }
        self.resp.rect.min.y + y - self.pan.y
    }
    fn y_mid(&self, idx: usize) -> f32 {
        self.y(idx) + self.y_size(idx) * 0.5
    }
    fn y_end(&self, idx: usize) -> f32 {
        self.y(idx) + self.y_size(idx)
    }

    fn left(&self) -> f32 {
        self.resp.rect.left()
    }
    fn right(&self) -> f32 {
        self.resp.rect.right()
    }
    fn top(&self) -> f32 {
        self.resp.rect.top()
    }
    fn bottom(&self) -> f32 {
        self.resp.rect.bottom()
    }

    fn mm_rect(&self, x1: f32, y1: f32, x2: f32, y2: f32) -> Rect {
        Rect::from_min_max(
            self.resp.rect.min + vec2(x1, y1),
            self.resp.rect.min + vec2(x2, y2),
        )
    }

    fn lane_rect(&self, lane: usize, start: usize, end: usize) -> Rect {
        Rect::from_min_max(
            pos2(self.x(start), self.y(lane)),
            pos2(self.x_end(end), self.y_end(lane)),
        )
    }

    fn draw_dashed_rect(&self, rect: Rect, stroke: Stroke, fill: Color32, spacing: f32) {
        // Optional: draw the rounded rectangle border
        self.painter
            .rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);

        // Diagonal line parameters
        let stroke = Stroke::new(stroke.width, fill);

        let min = rect.min;
        let max = rect.max;

        let mut offset = 0.0;
        let max_dim = rect.width() + rect.height();
        let inset = stroke.width * 0.5;

        while offset < max_dim {
            let start = if offset < rect.width() {
                Pos2::new(min.x + offset, max.y - inset)
            } else {
                Pos2::new(max.x - inset, max.y - (offset - rect.width()))
            };

            let end = if offset < rect.height() {
                Pos2::new(min.x + inset, max.y - offset)
            } else {
                Pos2::new(min.x + (offset - rect.height()), min.y + inset)
            };

            self.painter.line_segment([start, end], stroke);

            offset += spacing;
        }
    }

    fn draw_text_in_box(
        &self,
        pos: Pos2,
        color: Color32,
        stroke: Stroke,
        fill: Color32,
        size: f32,
        text: impl Into<String>,
    ) {
        let text: String = text.into();

        let needed_size = self.calculate_text_size(size, &text) + Vec2::splat(size * 0.5);
        let rect = Rect::from_center_size(pos + needed_size * vec2(0.5, 0.0), needed_size);

        self.painter
            .rect(rect, 0.0, fill, stroke, egui::StrokeKind::Inside);

        self.draw_fit_text(rect, Align2::LEFT_CENTER, size, text, color, TextFit::Hide);
    }

    fn draw_fit_text(
        &self,
        rect: Rect,
        align: Align2,
        size: f32,
        text: impl Into<String>,
        c: Color32,
        fit: TextFit,
    ) -> bool {
        let text: String = text.into();

        let needed_size = self.calculate_text_size(size, &text);
        //let actual_rect = Rect::from_min_max(align.anchor_rect(rect).min, rect.max);
        let fits_x = needed_size.x <= rect.size().x;
        let fits_y = needed_size.y <= rect.size().y;

        let mut pos = align.pos_in_rect(&rect);

        let inset_dir = (pos - rect.center()).normalized();

        pos -= inset_dir * size * 0.25;

        if (fits_x && fits_y) || fit == TextFit::Ignore {
            self.painter
                .text(pos, align, text, FontId::proportional(size), c);
            return true;
        }

        if fit == TextFit::Shrink {
            let max = if fits_x { rect.size().y } else { rect.size().x };

            let overrun = (needed_size - rect.size()).max_elem();
            let scale_factor = max / overrun;

            self.painter.text(
                pos,
                align,
                text,
                FontId::proportional(size * scale_factor),
                c,
            );
        }

        // TODO: implement truncate

        false
    }

    fn calculate_text_size(&self, size: f32, text: &String) -> Vec2 {
        let galley =
            self.painter
                .layout_no_wrap(text.clone(), FontId::proportional(size), Color32::MAGENTA);
        galley.size()
    }

    fn draw_vertical_line(&self, x: f32, lane_start: usize, lane_end: usize, stroke: Stroke) {
        self.painter.line_segment(
            [pos2(x, self.y(lane_start)), pos2(x, self.y_end(lane_end))],
            stroke,
        );
    }

    fn draw_lane_separators(&self, stroke: Stroke) {
        for i in 0..NUM_LANES {
            let y = self.y(i);
            self.painter
                .line_segment([pos2(self.left(), y), pos2(self.right(), y)], stroke);
        }
    }

    fn draw_beat_separators(&self, stroke: Stroke) {
        for i in 0..self.cue.beats.len() {
            let x = self.x(i);
            self.draw_vertical_line(x, 1, 1, stroke);
            self.draw_vertical_line(x, 5, 34, stroke);
        }
    }

    fn draw_bar_separators(&self, stroke: Stroke) {
        for (i, beat) in self.cue.beats.iter().enumerate() {
            if beat.count == 1 {
                let x = self.x(i);
                self.draw_vertical_line(x, 1, 1, stroke);
                self.draw_vertical_line(x, 5, 34, stroke);
            }
        }
    }

    fn draw_header(&self, beat: usize, lane: usize, text: String) {
        self.painter.text(
            pos2(self.x(beat), self.y_mid(lane)),
            Align2::LEFT_CENTER,
            text,
            FontId::proportional(11.0),
            self.style.strong_text_color(),
        );
    }

    pub fn render_ruler(&self, beats: bool) {
        const MIN_POINTS_PER_STEP: f32 = 50.0;

        let mut dist_since_last = 0.0;
        let mut region_idx = 0;
        for (i, beat) in self.cue.beats.iter().enumerate() {
            let region_rect = if let Some(region) = self.regions.get(region_idx) {
                if region.0 as usize == i {
                    region_idx += 1;
                    dist_since_last = f32::MAX;
                }
                self.lane_rect(1, i, region.1 as usize)
            } else {
                self.lane_rect(1, i, self.last_beat())
            };

            if beats || (beat.count == 1 && dist_since_last >= MIN_POINTS_PER_STEP) {
                self.draw_fit_text(
                    region_rect,
                    Align2::LEFT_CENTER,
                    11.0,
                    if beats {
                        format!("{}.{}", beat.bar_number, beat.count)
                    } else {
                        format!("{}", beat.bar_number)
                    },
                    self.style.strong_text_color(),
                    TextFit::Hide,
                );
                dist_since_last = 0.0;
            }
            dist_since_last += self.beat_width_from_length(beat.length);
        }
    }

    pub fn render_regions(&self) {
        for region in &self.regions {
            let rect = self.lane_rect(0, region.0.into(), region.1.into());

            let stroke = Stroke::new(1.0, self.style.text_color());

            self.painter.rect(
                rect,
                5.0,
                self.style.code_bg_color,
                stroke,
                egui::StrokeKind::Inside,
            );

            self.draw_fit_text(
                rect,
                Align2::LEFT_CENTER,
                14.0,
                region.2.clone(),
                self.style.text_color(),
                TextFit::Hide,
            );

            self.draw_vertical_line(self.x(region.0.into()), 1, 3, stroke);
            self.draw_vertical_line(self.x(region.0.into()), 5, 34, stroke);
        }
    }

    pub fn render_jumps(&self) {
        for event in self.cue.events.iter() {
            if let Some(EventDescription::JumpEvent {
                destination,
                requirement,
                when_jumped,
                when_passed,
            }) = event.event
            {
                self.render_jump(
                    event.location,
                    destination,
                    when_jumped,
                    when_passed,
                    requirement,
                );
            }
        }
    }

    pub fn render_jump(
        &self,
        location: u16,
        destination: u16,
        when_jumped: JumpModeChange,
        when_passed: JumpModeChange,
        requirement: JumpRequirement,
    ) {
        if destination < location && when_jumped == JumpModeChange::SetOff {
            self.render_jump_repeat(location, destination);
        } else if destination < location {
            self.render_jump_vamp(location, destination)
        } else {
            self.render_jump_skip(location, destination)
        };
    }

    fn render_jump_repeat(&self, location: u16, destination: u16) {
        let rect = self.lane_rect(2, destination.into(), location.into());
        self.painter.rect(
            rect,
            5.0,
            Color32::YELLOW.gamma_multiply(0.5),
            Stroke::new(2.0, Color32::YELLOW),
            egui::StrokeKind::Inside,
        );
        self.draw_fit_text(
            rect,
            Align2::CENTER_CENTER,
            14.0,
            "Repeat",
            Color32::BLACK,
            TextFit::Hide,
        );
    }

    fn render_jump_vamp(&self, location: u16, destination: u16) {
        let rect = self.lane_rect(2, destination.into(), location.into());
        self.painter.rect(
            rect,
            5.0,
            Color32::YELLOW,
            Stroke::new(2.0, Color32::YELLOW),
            egui::StrokeKind::Inside,
        );
        self.draw_fit_text(
            rect,
            Align2::CENTER_CENTER,
            14.0,
            "Vamp",
            Color32::BLACK,
            TextFit::Hide,
        );
    }

    fn render_jump_skip(&self, location: u16, destination: u16) {
        if location + 1 == destination {
            return;
        }
        let rect = self.lane_rect(
            2,
            location.saturating_add(1).into(),
            destination.saturating_sub(1).into(),
        );
        self.draw_dashed_rect(
            rect,
            Stroke::new(2.0, Color32::YELLOW),
            Color32::YELLOW.gamma_multiply(0.5),
            10.0,
        );
        self.draw_fit_text(
            rect,
            Align2::CENTER_CENTER,
            14.0,
            "(Skip)",
            Color32::WHITE,
            TextFit::Hide,
        );
    }

    fn render_tempo_changes(&self) {
        for event in self.cue.events.iter() {
            if let Some(EventDescription::TempoChangeEvent { tempo }) = event.event {
                self.render_tempo_change(event.location, tempo);
            } else if let Some(EventDescription::GradualTempoChangeEvent {
                start_tempo,
                end_tempo,
                length,
            }) = event.event
            {
                self.draw_dashed_rect(
                    self.lane_rect(
                        2,
                        event.location as usize,
                        event.location as usize + length as usize - 1,
                    ),
                    self.style.window_stroke,
                    self.style.window_stroke.color,
                    10.0,
                );
                self.render_tempo_change(event.location, start_tempo);
                self.render_tempo_change(event.location + length, end_tempo);
            }
        }
    }

    fn render_tempo_change(&self, location: u16, tempo: u16) {
        let rect = self.lane_rect(2, location as usize, self.last_beat());
        self.draw_text_in_box(
            rect.left_center(),
            self.style.text_color(),
            self.style.window_stroke,
            self.style.extreme_bg_color,
            12.0,
            tempo.to_string(),
        );
    }

    fn render_edit_head(&self, position: f32) {
        self.draw_vertical_line(position, 2, 34, self.style.widgets.active.bg_stroke);
    }

    fn try_zoom(&self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        if ui.rect_contains_pointer(self.resp.rect) {
            let zoom_step = ui.input(|i| i.zoom_delta());
            app.zoom *= zoom_step;
            app.pan.x *= zoom_step;
            app.pan -= ui.input(|i| i.smooth_scroll_delta);
        }
    }

    fn beat_width_from_length(&self, length: u32) -> f32 {
        let mult = lerp(
            1.0..=length as f32 / 500000.0_f32,
            self.proportional_scaling,
        );
        self.base_beat_width * mult
    }

    //fn bar_numbers(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    //    self.blockout_lane(app, ui);
    //    let p = ui.painter();
    //    while let Some(beat) = self.next_beat() {
    //        if beat.count == 1 {
    //            p.text(
    //                self.head_text(),
    //                Align2::LEFT_TOP,
    //                beat.bar_number.to_string(),
    //                Self::FONT,
    //                Color32::GRAY,
    //            );
    //        }
    //    }
    //}

    //fn timecode(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    //    self.blockout_lane(app, ui);
    //    let p = ui.painter();
    //    let events = self.cue.events.clone();
    //    let mut cursor = EventCursor::new(&events);
    //    while let Some(_beat) = self.next_beat() {
    //        while cursor.at_or_before(self.beat_idx as u16) && let Some(event) = cursor.get_next() {
    //            if let Some(EventDescription::TimecodeEvent { time, properties }) = event.event {
    //                p.rect_filled(
    //                    Rect::from_min_size(
    //                        self.head,
    //                        vec2(Self::TEXT_SIZE * 7.0, Self::TEXT_SIZE),
    //                    ),
    //                    0.0,
    //                    Color32::BLACK,
    //                );
    //                p.text(
    //                    self.head_text(),
    //                    Align2::LEFT_TOP,
    //                    time.to_string(),
    //                    Self::FONT,
    //                    Color32::WHITE,
    //                );
    //            }
    //        }
    //    }
    //}

    //fn tempo(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    //    self.blockout_lane(app, ui);
    //    let p = ui.painter();
    //    let mut i: usize = 0;
    //    let events = self.cue.events.clone();
    //    let mut cursor = EventCursor::new(&events);
    //    while let Some(_beat) = self.next_beat() {
    //        while cursor.at_or_before(self.beat_idx as u16) && let Some(event) = cursor.get_next() {
    //            if let Some(EventDescription::TempoChangeEvent { tempo }) = event.event {
    //                p.text(
    //                    self.head_text(),
    //                    Align2::LEFT_TOP,
    //                    tempo,
    //                    Self::FONT,
    //                    Color32::YELLOW,
    //                );
    //            } else if let Some(EventDescription::GradualTempoChangeEvent {
    //                start_tempo,
    //                end_tempo,
    //                length,
    //            }) = event.event
    //            {
    //                p.text(
    //                    self.head_text(),
    //                    Align2::LEFT_TOP,
    //                    start_tempo,
    //                    Self::FONT,
    //                    Color32::YELLOW,
    //                );
    //                let mut line_length = 0.0;
    //                //line_length -= Self::TEXT_SIZE * 2.5;
    //                for beat_forward in
    //                    &app.project_file.show.cues[app.selected_cue_idx].beats[i..i + length as usize]
    //                {
    //                    line_length += self.beat_width_from_length(beat_forward.length);
    //                }
    //                p.line_segment(
    //                    [
    //                        self.head + vec2(Self::TEXT_SIZE * 2.5, Self::LANE_HEIGHT / 2.0),
    //                        self.head + vec2(line_length, Self::LANE_HEIGHT / 2.0),
    //                    ],
    //                    Stroke::new(2.0, Color32::YELLOW),
    //                );
    //                p.text(
    //                    self.head_text() + vec2(line_length, 0.0),
    //                    Align2::LEFT_TOP,
    //                    end_tempo,
    //                    Self::FONT,
    //                    Color32::YELLOW,
    //                );
    //            }
    //        }
    //        i += 1;
    //    }
    //}

    //fn rehearsal_marks(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    //    self.blockout_lane(app, ui);
    //    let p = ui.painter();
    //    let events = self.cue.events.clone();
    //    let mut cursor = EventCursor::new(&events);
    //    while let Some(_beat) = self.next_beat() {
    //        while cursor.at_or_before(self.beat_idx as u16) && let Some(event) = cursor.get_next() {
    //            if let Some(EventDescription::RehearsalMarkEvent { label }) = event.event {
    //                p.text(
    //                    self.head_text(),
    //                    Align2::LEFT_TOP,
    //                    label.str(),
    //                    Self::FONT,
    //                    Color32::RED,
    //                );
    //                p.line_segment(
    //                    [self.head, self.head + vec2(0.0, Self::LANE_HEIGHT)],
    //                    Stroke::new(2.0, Color32::RED),
    //                );
    //            }
    //        }
    //    }
    //}

    //fn jumps(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    //    self.blockout_lane(app, ui);
    //    let p = ui.painter();
    //    let events = self.cue.events.clone();
    //    let mut cursor = EventCursor::new(&events);
    //    while let Some(_beat) = self.next_beat() {
    //        while cursor.at_or_before(self.beat_idx as u16) && let Some(event) = cursor.get_next() {
    //            match event.event {
    //                Some(EventDescription::JumpEvent {
    //                    destination,
    //                    requirement,
    //                    when_jumped,
    //                    when_passed,
    //                }) => {
    //                    p.text(
    //                        self.head,
    //                        Align2::LEFT_TOP,
    //                        match (requirement, when_jumped, when_passed) {
    //                            (JumpRequirement::JumpModeOff, _, _) => {
    //                                // Volta
    //                                egui_material_icons::icons::ICON_STEP_OVER
    //                            }
    //                            (_, JumpModeChange::SetOff, _) => {
    //                                // Repeat
    //                                egui_material_icons::icons::ICON_REPEAT_ONE
    //                            }
    //                            (JumpRequirement::JumpModeOn, _, _) => {
    //                                // Repeat
    //                                egui_material_icons::icons::ICON_REPEAT
    //                            }
    //                            _ => egui_material_icons::icons::ICON_STEP_OUT,
    //                        },
    //                        FontId {
    //                            size: Self::FONT.size * 1.1,
    //                            family: egui::FontFamily::Monospace,
    //                        },
    //                        Color32::YELLOW,
    //                    );
    //                    p.text(
    //                        self.head_at_idx(destination as usize),
    //                        Align2::LEFT_TOP,
    //                        egui_material_icons::icons::ICON_STEP_INTO,
    //                        FontId {
    //                            size: Self::FONT.size * 1.1,
    //                            family: egui::FontFamily::Monospace,
    //                        },
    //                        Color32::YELLOW,
    //                    );
    //                }
    //                Some(EventDescription::PauseEvent { behaviour: _ }) => {
    //                    p.text(
    //                        self.head,
    //                        Align2::LEFT_TOP,
    //                        egui_material_icons::icons::ICON_PAUSE,
    //                        FontId {
    //                            size: Self::FONT.size * 1.1,
    //                            family: egui::FontFamily::Monospace,
    //                        },
    //                        Color32::YELLOW,
    //                    );
    //                }
    //                _ => {}
    //            }
    //        }
    //    }
    //}

    //fn playbacks(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    //    let clip_height = Self::LANE_HEIGHT * 2.4;

    //    let p = ui.painter();
    //    let events = self.cue.events.clone();
    //    let mut cursor = EventCursor::new(&events);
    //    while let Some(beat) = self.next_beat() {
    //        while cursor.at_or_before(self.beat_idx as u16) && let Some(event) = cursor.get_next() {
    //        // Events, i.e. playback start or playback stop get triggered once at the beat they
    //        // occur
    //            if let Some(EventDescription::PlaybackEvent {
    //                channel_idx,
    //                clip_idx,
    //                sample,
    //            }) = event.event
    //            {
    //                self.running_clips.push(RunningClip {
    //                    channel_idx: channel_idx.into(),
    //                    clip_idx: clip_idx.into(),
    //                    sample,
    //                    sample_offset_from_start: self.time_head * 48 / 1000,
    //                });
    //                p.line(
    //                    vec![
    //                        self.head + vec2(0.0, channel_idx as f32 * clip_height),
    //                        self.head + vec2(0.0, (channel_idx + 1) as f32 * clip_height),
    //                    ],
    //                    Stroke::new(3.0, Color32::GREEN),
    //                );
    //            } else if let Some(EventDescription::PlaybackStopEvent {
    //                channel_idx: stop_channel_idx,
    //            }) = event.event
    //            {
    //                self.running_clips
    //                    .retain(|e| e.channel_idx != stop_channel_idx as usize);
    //                p.line(
    //                    vec![
    //                        self.head + vec2(0.0, stop_channel_idx as f32 * clip_height),
    //                        self.head + vec2(0.0, (stop_channel_idx + 1) as f32 * clip_height),
    //                    ],
    //                    Stroke::new(3.0, Color32::DARK_RED),
    //                );
    //            }
    //        }

    //        // Clips running get triggered every beat until they end in a playback stop event, or
    //        // until the end of the cue
    //        for clip in self.running_clips.clone() {
    //            let beat_width = self.beat_width();
    //            p.rect_filled(
    //                Rect::from_min_max(
    //                    self.head + vec2(0.0, clip.channel_idx as f32 * clip_height),
    //                    self.head + vec2(beat_width, (clip.channel_idx + 1) as f32 * clip_height),
    //                ),
    //                0.0,
    //                Color32::BLUE.gamma_multiply(0.5),
    //            );

    //            // Waveform
    //            let sample_head =
    //                self.time_head * 48 / 1000 - clip.sample_offset_from_start + clip.sample as i64;
    //            let sample_len = beat.length as i64 * 48 / 1000;
    //            let start_bucket = sample_head / ClipManager::PEAK_BUCKET_SIZE as i64;
    //            let end_bucket = (sample_head + sample_len) / ClipManager::PEAK_BUCKET_SIZE as i64;
    //            let bucket_width =
    //                beat_width / (sample_len / ClipManager::PEAK_BUCKET_SIZE as i64) as f32;
    //            for bucket_idx in start_bucket..end_bucket {
    //                let bucket_head = self.head
    //                    + vec2(
    //                        bucket_width * (bucket_idx - start_bucket) as f32,
    //                        clip.channel_idx as f32 * clip_height,
    //                    );

    //                let bucket_val = match app
    //                    .clip_manager
    //                    .clips
    //                    .get(&(clip.channel_idx, clip.clip_idx))
    //                {
    //                    Some(clip) => {
    //                        if (bucket_idx as usize) < clip.peak_buckets.len() {
    //                            clip.peak_buckets[bucket_idx as usize]
    //                        } else {
    //                            0.0
    //                        }
    //                    }
    //                    None => 0.0,
    //                };
    //                let center_y = vec2(0.0, clip_height / 2.0);
    //                let height_push = clip_height / 2.0 * bucket_val.abs();
    //                p.line_segment(
    //                    [
    //                        bucket_head + center_y - vec2(0.0, height_push),
    //                        bucket_head + center_y + vec2(0.0, height_push),
    //                    ],
    //                    Stroke::new(bucket_width.ceil(), Color32::WHITE),
    //                );
    //            }
    //        }
    //    }
    //}

    //fn background(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    //    let sel_beat = self.cue.beats[app.selected_beat_idx].clone();
    //    let p = ui.painter();

    //    let mut beat_rect = Rect::from_min_max(self.rect.min, self.rect.min);
    //    for (i, beat) in self.cue.beats.iter().enumerate() {
    //        if beat.is_null() {
    //            break;
    //        }
    //        beat_rect.max =
    //            beat_rect.min + vec2(self.beat_width_from_length(beat.length), self.rect.height());

    //        // Selected beat marker
    //        if app.selected_beat_idx == i {
    //            p.rect_filled(beat_rect, 0.0, Color32::DARK_GREEN);
    //        }

    //        // Selected measure marker
    //        if beat.bar_number == sel_beat.bar_number {
    //            ui.scroll_to_rect(beat_rect, None);
    //            p.rect_filled(beat_rect, 0.0, Color32::DARK_GREEN.gamma_multiply(0.5));
    //        }

    //        // CI measure marker
    //        if beat.bar_number == 0 {
    //            p.rect_filled(beat_rect, 0.0, Color32::DARK_RED.gamma_multiply(0.5));
    //        }

    //        // Hovered beat marker
    //        if ui.rect_contains_pointer(beat_rect) {
    //            p.rect_filled(beat_rect, 0.0, Color32::GRAY.gamma_multiply(0.2));
    //            if self.resp.clicked() {
    //                app.selected_beat_idx = i;
    //            }
    //        }

    //        // Downbeat line and text
    //        if beat.count == 1 {
    //            p.line_segment(
    //                [beat_rect.left_top(), beat_rect.left_bottom()],
    //                Stroke::new(1.0, Color32::GRAY),
    //            );
    //        }
    //        // Other beats
    //        else {
    //            p.line_segment(
    //                [beat_rect.left_top(), beat_rect.left_bottom()],
    //                Stroke::new(1.0, Color32::DARK_GRAY),
    //            );
    //        }
    //        beat_rect.min.x = beat_rect.max.x;
    //    }
    //}

    //fn blockout_lane(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
    //    let p = ui.painter();

    //    p.rect_filled(
    //        Rect::from_min_max(
    //            self.head,
    //            pos2(self.rect.max.x, self.head.y + Self::LANE_HEIGHT),
    //        ),
    //        0,
    //        ui.style().visuals.window_fill().gamma_multiply(0.5),
    //    );
    //}
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

    let persistent = TimelinePersistent::default();
    let mut tlr = TimelineRenderer::new(
        app,
        ui,
        cue,
        persistent,
        app.ctx
            .animate_bool("proportional_scaling".into(), app.proportional_beat_length),
    );

    let stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;

    let show_individual_beats = tlr.beat_width_from_length(500000) > 20.0;
    if show_individual_beats {
        tlr.draw_beat_separators(stroke);
    } else {
        tlr.draw_bar_separators(stroke);
    }
    tlr.render_ruler(show_individual_beats);
    tlr.draw_lane_separators(stroke);
    tlr.render_regions();
    tlr.render_edit_head(ui.ctx().animate_value_with_time(
        "edit_cursor_x_location".into(),
        tlr.x(app.selected_beat_idx),
        0.05,
    ));
    tlr.render_jumps();
    tlr.render_tempo_changes();

    const DEADZONE: f32 = 150.0;
    if tlr.x(app.selected_beat_idx) > tlr.right() - DEADZONE {
        app.pan += Vec2::RIGHT * (tlr.x(app.selected_beat_idx) - tlr.right() + DEADZONE)
            / ui.style().animation_time
            * 0.02
    }
    if tlr.x(app.selected_beat_idx) < tlr.left() + DEADZONE {
        app.pan += Vec2::RIGHT * (tlr.x(app.selected_beat_idx) - tlr.left() - DEADZONE)
            / ui.style().animation_time
            * 0.02
    }

    //tlr.background(app, ui);

    //tlr.jumps(app, ui);
    //tlr.next_lane();
    //tlr.bar_numbers(app, ui);
    //tlr.next_lane();
    //tlr.timecode(app, ui);
    //tlr.next_lane();
    //tlr.tempo(app, ui);
    //tlr.next_lane();
    //tlr.rehearsal_marks(app, ui);
    //tlr.next_lane();
    //tlr.playbacks(app, ui);

    tlr.try_zoom(app, ui);
}
