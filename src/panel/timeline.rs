use crate::app::ClicksEditorApp;
use egui::{
    Align, Align2, Color32, CursorIcon, FontId, InputState, Painter, Pos2, Rect, Response, Stroke,
    Vec2, Visuals, lerp, pos2, vec2,
};
use ks_common_clicks::{
    beat::Beat,
    cue::Cue,
    event::{self, EventDescription, JumpModeChange, JumpRequirement},
};
use ks_common_generic::smpte::{FrameRate, Timecode, TimecodeOffset};
use std::{
    error::Error,
    fmt::Debug,
    hash::{self, Hash, Hasher},
};

const NUM_LANES: usize = 35;
const INTERACTION_HANDLE_SIZE: f32 = 10.0;

// rehearsal marks
// bar.beat ruler
// tempo changes
// jumps/vamps
// LTC ruler
// playback x30

#[derive(Debug, strum::Display)]
pub enum InteractionError {
    ArgumentOutOfBounds,
}

impl Error for InteractionError {}

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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum TextFit {
    _Truncate,
    Shrink,
    Hide,
    Ignore,
}

struct PlaybackClip {
    starter_event_idx: usize,
    start_beat: usize,
    stop_beat: usize,
    event_desc: EventDescription,
}

type InteractionFunctionDragX =
    Option<Box<dyn Fn(&mut Cue, u16) -> Result<bool, InteractionError>>>;
type InteractionFunctionDragY =
    Option<Box<dyn Fn(&mut Cue, usize) -> Result<bool, InteractionError>>>;
type InteractionFunctionClick = Option<Box<dyn Fn(&mut Cue) -> Result<bool, InteractionError>>>;
type InteractionResult = Result<bool, InteractionError>;

pub struct TimelineInteractable {
    rect: Rect,
    drag_x: InteractionFunctionDragX,
    drag_y: InteractionFunctionDragY,
    _click: InteractionFunctionClick,
    event_idx: Option<usize>,
    hash: u64,
}

impl Debug for TimelineInteractable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.hash.to_string())
    }
}

impl TimelineInteractable {
    pub fn new(
        salt: impl Into<String>,
        rect: Rect,
        event_idx: usize,
        drag_x: InteractionFunctionDragX,
        drag_y: InteractionFunctionDragY,
        click: InteractionFunctionClick,
    ) -> Self {
        Self::new_opt(salt, rect, Some(event_idx), drag_x, drag_y, click)
    }

    pub fn new_opt(
        salt: impl Into<String>,
        rect: Rect,
        event_idx: Option<usize>,
        drag_x: InteractionFunctionDragX,
        drag_y: InteractionFunctionDragY,
        click: InteractionFunctionClick,
    ) -> Self {
        Self {
            rect,
            drag_x,
            _click: click,
            drag_y,
            event_idx,
            hash: 0,
        }
        .hashed(&salt.into())
    }

    pub fn basic(salt: impl Into<String>, rect: Rect, event_idx: usize) -> Self {
        Self::new(salt, rect, event_idx, None, None, None)
    }

    pub fn drag(
        salt: impl Into<String>,
        rect: Rect,
        drag_x: InteractionFunctionDragX,
        drag_y: InteractionFunctionDragY,
    ) -> Self {
        Self::new_opt(salt, rect, None, drag_x, drag_y, None)
    }
    pub fn drag_x(salt: impl Into<String>, rect: Rect, drag_x: InteractionFunctionDragX) -> Self {
        Self::new_opt(salt, rect, None, drag_x, None, None)
    }
    pub fn drag_y(salt: impl Into<String>, rect: Rect, drag_y: InteractionFunctionDragY) -> Self {
        Self::new_opt(salt, rect, None, None, drag_y, None)
    }
    pub fn click(salt: impl Into<String>, rect: Rect, click: InteractionFunctionClick) -> Self {
        Self::new_opt(salt, rect, None, None, None, click)
    }
    pub fn event_move(salt: impl Into<String>, rect: Rect, event_idx: usize) -> Self {
        Self::new_opt(
            salt,
            rect,
            Some(event_idx),
            Self::make_event_location_drag(event_idx),
            None,
            None,
        )
    }

    fn hash(&self, salt: &str) -> u64 {
        let mut h = hash::DefaultHasher::new();
        self.event_idx.hash(&mut h);
        self.drag_x.is_some().hash(&mut h);
        self.drag_y.is_some().hash(&mut h);
        salt.hash(&mut h);
        h.finish()
    }

    fn hashed(self, salt: &str) -> Self {
        Self {
            hash: self.hash(salt),
            ..self
        }
    }

    pub fn make_event_location_drag(event_idx: usize) -> InteractionFunctionDragX {
        let event_idx = event_idx.try_into().ok()?;
        Some(Box::new(move |cue, beat| {
            if let Some(event) = cue.events.get_mut(event_idx) {
                event.location = beat;
                return Ok(true);
            }
            Ok(false)
        }))
    }
}

struct TimelineRenderer {
    style: Visuals,
    resp: Response,
    painter: Painter,
    cue: Cue,
    regions: Vec<(u16, u16, String, usize)>,
    pan: Vec2,
    base_beat_width: f32,
    proportional_scaling: f32,
    persistent: TimelinePersistent,
    interactions: Vec<TimelineInteractable>,
}

impl TimelineRenderer {
    fn new(
        app: &mut ClicksEditorApp,
        ui: &mut egui::Ui,
        cue: Cue,
        persistent: TimelinePersistent,
    ) -> Self {
        let (resp, p) = ui.allocate_painter(ui.available_size(), egui::Sense::click());
        Self {
            painter: p,
            regions: Self::calculate_regions(&cue),
            resp,
            cue,
            pan: app.pan,
            base_beat_width: app.zoom,
            proportional_scaling: 0.0,
            persistent,
            style: ui.style().visuals.clone(),
            interactions: vec![],
        }
    }

    pub fn show(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        let show_individual_beats = self.beat_width_from_length(500000) > 20.0;
        if show_individual_beats {
            self.draw_beat_separators();
        } else {
            self.draw_bar_separators();
        }
        self.render_ruler(show_individual_beats);
        self.draw_lane_separators();
        self.render_regions();
        self.render_ltc_events(app.selected_beat_idx);

        self.render_playback();

        self.render_edit_head_animated(app, ui);
        self.render_jumps();
        self.render_tempo_changes();

        self.render_lane_list();

        self.try_autopan(app, ui);
        self.try_zoom(app, ui);

        self.handle_interaction(app, ui);
    }

    fn try_autopan(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        const DEADZONE: f32 = 150.0;
        if self.x(app.selected_beat_idx) > self.right() - DEADZONE {
            app.pan += Vec2::RIGHT * (self.x(app.selected_beat_idx) - self.right() + DEADZONE)
                / ui.style().animation_time
                * 0.02;
        }
        if self.x(app.selected_beat_idx) < self.left() + DEADZONE {
            app.pan += Vec2::RIGHT * (self.x(app.selected_beat_idx) - self.left() - DEADZONE)
                / ui.style().animation_time
                * 0.02;
        }
    }

    pub fn with_proportional_scaling(mut self, proportional_scaling: f32) -> Self {
        self.proportional_scaling = proportional_scaling;
        self
    }

    fn register_interaction_rect(&mut self, inter: TimelineInteractable) {
        self.interactions.push(inter);
    }

    fn calculate_regions(cue: &Cue) -> Vec<(u16, u16, String, usize)> {
        let mut regions: Vec<(u16, u16, String, usize)> = vec![];
        for (i, event) in cue.events.iter().enumerate() {
            if let Some(EventDescription::RehearsalMarkEvent { label }) = event.event {
                if let Some(last) = regions.last_mut() {
                    last.1 = event.location.saturating_sub(1);
                }
                regions.push((event.location, 0, label.str().to_string(), i));
            }
        }
        regions
    }

    fn calculate_region_for_beat(&self, beat: usize) -> Option<(u16, u16, String, usize)> {
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
        self.resp.rect.min.x + offs - self.pan.x + Self::TRACK_PANEL_WIDTH
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
            y += self.y_size(i);
        }
        self.resp.rect.min.y + y - self.pan.y
    }
    fn y_mid(&self, idx: usize) -> f32 {
        self.y(idx) + self.y_size(idx) * 0.5
    }
    fn y_end(&self, idx: usize) -> f32 {
        self.y(idx) + self.y_size(idx)
    }

    const TRACK_PANEL_WIDTH: f32 = 150.0;

    fn left(&self) -> f32 {
        self.resp.rect.left() + Self::TRACK_PANEL_WIDTH
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

    fn lane_at_y(&self, y: f32) -> usize {
        for lane in 0..NUM_LANES {
            if self.y(lane) > y {
                return lane;
            }
        }
        0
    }

    fn beat_at_x(&self, x: f32) -> usize {
        for i in 0..self.cue.beats.len() {
            if self.x(i) > x {
                return i;
            }
        }
        0
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

    fn make_edge_rect(rect: Rect, align: Align2) -> Rect {
        let center = align.pos_in_rect(&rect);
        let size_x = if align.y() == Align::Center {
            INTERACTION_HANDLE_SIZE
        } else {
            rect.width() + INTERACTION_HANDLE_SIZE
        };
        let size_y = if align.x() == Align::Center {
            INTERACTION_HANDLE_SIZE
        } else {
            rect.height() + INTERACTION_HANDLE_SIZE
        };
        Rect::from_center_size(center, vec2(size_x, size_y))
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
    ) -> Rect {
        let text: String = text.into();

        let needed_size = self.calculate_text_size(size, &text) + Vec2::splat(size * 0.5);
        let rect = Rect::from_center_size(pos + needed_size * vec2(0.5, 0.0), needed_size);

        self.painter
            .rect(rect, 0.0, fill, stroke, egui::StrokeKind::Inside);

        self.draw_fit_text(rect, Align2::LEFT_CENTER, size, text, color, TextFit::Hide);

        rect
    }

    fn draw_text_basic(&self, rect: Rect, text: impl Into<String>) {
        self.draw_fit_text(
            rect,
            Align2::LEFT_TOP,
            12.0,
            text,
            self.style.text_color(),
            TextFit::Hide,
        );
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

    fn calculate_text_size(&self, size: f32, text: &str) -> Vec2 {
        let galley = self.painter.layout_no_wrap(
            text.to_string(),
            FontId::proportional(size),
            Color32::MAGENTA,
        );
        galley.size()
    }

    fn draw_vertical_line(&self, x: f32, lane_start: usize, lane_end: usize, stroke: Stroke) {
        self.painter.line_segment(
            [pos2(x, self.y(lane_start)), pos2(x, self.y_end(lane_end))],
            stroke,
        );
    }

    fn draw_lane_separators(&self) {
        let stroke = self.style.widgets.noninteractive.bg_stroke;
        for i in 0..NUM_LANES {
            let y = self.y(i);
            self.painter
                .line_segment([pos2(self.left(), y), pos2(self.right(), y)], stroke);
        }
    }

    fn draw_beat_separators(&self) {
        let stroke = self.style.widgets.noninteractive.bg_stroke;
        for i in 0..self.cue.beats.len() {
            let x = self.x(i);
            self.draw_vertical_line(x, 1, 1, stroke);
            self.draw_vertical_line(x, 5, 34, stroke);
        }
    }

    fn draw_bar_separators(&self) {
        let stroke = self.style.widgets.noninteractive.bg_stroke;
        for (i, beat) in self.cue.beats.iter().enumerate() {
            if beat.count == 1 {
                let x = self.x(i);
                self.draw_vertical_line(x, 1, 1, stroke);
                self.draw_vertical_line(x, 5, 34, stroke);
            }
        }
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
                self.draw_ruler_marking(region_rect, *beat, beats);
                dist_since_last = 0.0;
            }
            dist_since_last += self.beat_width_from_length(beat.length);
        }
    }

    fn draw_ruler_marking(&self, region_rect: Rect, beat: Beat, with_beat_count: bool) {
        let strong = beat.count == 1;
        self.draw_fit_text(
            region_rect,
            Align2::LEFT_CENTER,
            11.0,
            if with_beat_count {
                format!("{}.{}", beat.bar_number, beat.count)
            } else {
                format!("{}", beat.bar_number)
            },
            self.text_color_strong_weak(strong),
            TextFit::Hide,
        );
    }

    fn text_color_strong_weak(&self, strong: bool) -> Color32 {
        if strong {
            self.style.strong_text_color()
        } else {
            self.style.weak_text_color()
        }
    }

    pub fn render_regions(&mut self) {
        for region in self.regions.clone() {
            let rect = self.lane_rect(0, region.0.into(), region.1.into());

            self.render_region(region.2.clone(), rect);
            self.register_interaction_rect(TimelineInteractable::new(
                "region_drag",
                TimelineRenderer::make_edge_rect(rect, Align2::LEFT_CENTER),
                region.3,
                TimelineInteractable::make_event_location_drag(region.3),
                None,
                None,
            ));
        }
    }

    fn render_region(&self, label: String, rect: Rect) {
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
            label,
            self.style.text_color(),
            TextFit::Hide,
        );

        self.draw_vertical_line(rect.left(), 1, 3, stroke);
        self.draw_vertical_line(rect.left(), 5, 34, stroke);
    }

    pub fn render_playback(&mut self) {
        // (event_idx, start, end, event_desc)
        let clips = self.calculate_playback_clips();

        for (i, channel) in clips.iter().enumerate() {
            for clip in channel {
                let event_idx = clip.starter_event_idx;
                let rect = self.lane_rect(i + 5, clip.start_beat, clip.stop_beat - 1);
                self.render_playback_clip(event_idx, rect, clip.event_desc);
            }
        }

        for (i, event) in self.cue.events.clone().iter().enumerate() {
            if let Some(EventDescription::PlaybackStopEvent { channel_idx }) = event.event {
                self.render_playback_stop(i, channel_idx.into(), event.location);
            }
        }
    }

    fn calculate_playback_clips(&mut self) -> Vec<Vec<PlaybackClip>> {
        let mut clips = Vec::<Vec<PlaybackClip>>::new();
        clips.resize_with(32, Vec::new);
        for (i, event) in self.cue.events.iter().enumerate() {
            if let Some(EventDescription::PlaybackEvent {
                sample: _,
                channel_idx,
                clip_idx: _,
            }) = event.event
            {
                if let Some(clip) = clips[channel_idx as usize].last_mut() {
                    clip.stop_beat = event.location.into();
                }
                clips[channel_idx as usize].push(PlaybackClip {
                    starter_event_idx: i,
                    start_beat: event.location.into(),
                    stop_beat: self.last_beat(),
                    event_desc: event.event.expect("We are inside the if"),
                });
            } else if let Some(EventDescription::PlaybackStopEvent { channel_idx }) = event.event
                && let Some(clip) = clips[channel_idx as usize].last_mut()
            {
                clip.stop_beat = event.location.into();
            }
        }
        clips
    }

    fn render_playback_clip(
        &mut self,
        event_idx: usize,
        rect: Rect,
        description: EventDescription,
    ) {
        if let EventDescription::PlaybackEvent {
            sample: _,
            channel_idx: _,
            clip_idx,
        } = description
        {
            self.painter.rect(
                rect,
                8.0,
                Color32::BLUE,
                self.style.window_stroke,
                egui::StrokeKind::Inside,
            );
            self.draw_text_basic(rect, format!("Clip #{clip_idx}"));
            self.register_interaction_rect(TimelineInteractable::event_move(
                "playback_start_drag",
                TimelineRenderer::make_edge_rect(rect, Align2::LEFT_CENTER),
                event_idx,
            ));
            self.register_interaction_rect(TimelineInteractable::drag_y(
                "playback_drag_channel",
                rect,
                Some(Box::new(move |cue, lane| {
                    if let Some(event) = cue.events.get_mut(
                        event_idx
                            .try_into()
                            .map_err(|_| InteractionError::ArgumentOutOfBounds)?,
                    ) && let Some(EventDescription::PlaybackEvent { channel_idx, .. }) =
                        event.event.as_mut()
                    {
                        *channel_idx = u16::try_from(lane.saturating_sub(6)).unwrap_or(0);
                        return Ok(true);
                    }
                    Ok(false)
                })),
            ));
        }
    }

    fn render_playback_stop(&mut self, event_idx: usize, channel_idx: usize, location: u16) {
        let rect = self.lane_rect(channel_idx + 5, location.into(), self.last_beat());
        let act_rect = self.draw_playback_stop_marker(rect);
        self.register_interaction_rect(TimelineInteractable::new(
            "playback_stop_drag",
            act_rect,
            event_idx,
            TimelineInteractable::make_event_location_drag(event_idx),
            Some(Box::new(move |cue, lane| {
                if let Some(event) = cue.events.get_mut(
                    event_idx
                        .try_into()
                        .map_err(|_| InteractionError::ArgumentOutOfBounds)?,
                ) && let Some(EventDescription::PlaybackStopEvent { channel_idx }) =
                    event.event.as_mut()
                {
                    *channel_idx = u16::try_from(lane.saturating_sub(6)).unwrap_or(0);
                    return Ok(true);
                }
                Ok(false)
            })),
            None,
        ));
    }

    fn draw_playback_stop_marker(&mut self, rect: Rect) -> Rect {
        self.draw_text_in_box(
            rect.left_center(),
            self.style.text_color(),
            self.style.window_stroke,
            self.style.window_fill,
            12.0,
            "STOP",
        )
    }

    pub fn render_jumps(&mut self) {
        for (i, event) in self.cue.events.clone().iter().enumerate() {
            if let Some(EventDescription::JumpEvent {
                destination,
                requirement,
                when_jumped,
                when_passed,
            }) = event.event
            {
                self.render_jump(i, event, destination, requirement, when_jumped, when_passed);
            }
        }
    }

    fn render_jump(
        &mut self,
        i: usize,
        event: &event::Event,
        destination: u16,
        requirement: JumpRequirement,
        when_jumped: JumpModeChange,
        when_passed: JumpModeChange,
    ) {
        let rect = self.draw_jump(
            event.location,
            destination,
            when_jumped,
            when_passed,
            requirement,
        );

        let (dest_side, loc_side) = if destination > event.location {
            (Align2::RIGHT_CENTER, Align2::LEFT_CENTER)
        } else {
            (Align2::LEFT_CENTER, Align2::RIGHT_CENTER)
        };

        self.register_interaction_rect(TimelineInteractable::event_move(
            "jump_drag_loc",
            TimelineRenderer::make_edge_rect(rect, loc_side),
            i,
        ));

        self.register_interaction_rect(TimelineInteractable::new(
            "jump_event_drag_destination",
            TimelineRenderer::make_edge_rect(rect, dest_side),
            i,
            jump_event_destination_drag_interaction(i),
            None,
            None,
        ));
    }

    pub fn draw_jump(
        &mut self,
        location: u16,
        destination: u16,
        when_jumped: JumpModeChange,
        _when_passed: JumpModeChange,
        requirement: JumpRequirement,
    ) -> Rect {
        if destination < location && when_jumped == JumpModeChange::SetOff {
            self.draw_jump_repeat(location, destination)
        } else if destination < location {
            self.draw_jump_vamp(location, destination)
        } else if requirement == JumpRequirement::JumpModeOff {
            self.draw_jump_volta(location, destination)
        } else {
            self.draw_jump_skip(location, destination)
        }
    }

    fn draw_jump_repeat(&mut self, location: u16, destination: u16) -> Rect {
        let rect = self.lane_rect(3, destination.into(), location.into());
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

        rect
    }

    fn draw_jump_vamp(&mut self, location: u16, destination: u16) -> Rect {
        let rect = self.lane_rect(3, destination.into(), location.into());
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

        rect
    }

    fn draw_jump_skip(&self, location: u16, destination: u16) -> Rect {
        if location + 1 == destination {
            return Rect::ZERO;
        }
        let rect = self.lane_rect(
            3,
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
        rect
    }

    fn draw_jump_volta(&self, location: u16, destination: u16) -> Rect {
        if location + 1 == destination {
            return Rect::ZERO;
        }
        let rect = self
            .lane_rect(
                3,
                location.saturating_add(1).into(),
                destination.saturating_sub(1).into(),
            )
            .with_min_y(self.y_mid(3));
        self.draw_dashed_rect(
            rect,
            Stroke::new(2.0, Color32::YELLOW),
            Color32::YELLOW.gamma_multiply(0.5),
            10.0,
        );
        self.draw_fit_text(
            rect,
            Align2::CENTER_BOTTOM,
            14.0,
            "(Skip)",
            Color32::WHITE,
            TextFit::Hide,
        );
        rect
    }

    fn render_tempo_changes(&mut self) {
        for (i, event) in self.cue.events.clone().iter().enumerate() {
            if let Some(EventDescription::TempoChangeEvent { tempo }) = event.event {
                self.render_tempo_change(i, event, tempo);
            } else if let Some(EventDescription::GradualTempoChangeEvent {
                start_tempo,
                end_tempo,
                length,
            }) = event.event
            {
                self.render_gradual_tempo_change(i, event, start_tempo, end_tempo, length);
            }
        }
    }

    fn render_gradual_tempo_change(
        &mut self,
        i: usize,
        event: &event::Event,
        start_tempo: u16,
        end_tempo: u16,
        length: u16,
    ) {
        let (act_rect_a, act_rect_b) =
            self.draw_gradual_tempo_change(event, start_tempo, end_tempo, length);
        self.register_interaction_rect(TimelineInteractable::event_move(
            "grad_tempo_marker_drag_main",
            act_rect_a,
            i,
        ));
        self.register_interaction_rect(TimelineInteractable::new(
            "grad_tempo_marker_drag_end",
            act_rect_b,
            i,
            grad_tempo_event_length_drag_interaction(i),
            None,
            None,
        ));
    }

    fn draw_gradual_tempo_change(
        &mut self,
        event: &event::Event,
        start_tempo: u16,
        end_tempo: u16,
        length: u16,
    ) -> (Rect, Rect) {
        let rect_a = self.lane_rect(2, event.location as usize, self.last_beat());
        let rect_b = self.lane_rect(2, (length + event.location) as usize, self.last_beat());
        let rect_mid = self.lane_rect(
            2,
            event.location as usize,
            event.location as usize + length as usize - 1,
        );
        self.draw_dashed_rect(
            rect_mid,
            self.style.window_stroke,
            self.style.window_stroke.color,
            10.0,
        );
        let act_rect_a = self.draw_tempo_change(rect_a, start_tempo);
        let act_rect_b = self.draw_tempo_change(rect_b, end_tempo);
        (act_rect_a, act_rect_b)
    }

    fn render_tempo_change(&mut self, i: usize, event: &event::Event, tempo: u16) {
        let rect = self.lane_rect(2, event.location as usize, self.last_beat());
        let act_rect = self.draw_tempo_change(rect, tempo);
        self.register_interaction_rect(TimelineInteractable::new(
            "tempo_marker_drag",
            act_rect,
            i,
            TimelineInteractable::make_event_location_drag(i),
            None,
            None,
        ));
    }

    fn draw_tempo_change(&self, rect: Rect, tempo: u16) -> Rect {
        self.draw_text_in_box(
            rect.left_center(),
            self.style.text_color(),
            self.style.window_stroke,
            self.style.extreme_bg_color,
            12.0,
            tempo.to_string(),
        )
    }

    fn render_edit_head_animated(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        self.render_edit_head(ui.ctx().animate_value_with_time(
            "edit_cursor_x_location".into(),
            self.x(app.selected_beat_idx),
            0.05,
        ));
    }

    fn render_edit_head(&self, position: f32) {
        self.draw_vertical_line(position, 2, 34, self.style.widgets.active.bg_stroke);
    }

    fn render_ltc_events(&mut self, cursor_pos: usize) {
        for (i, event) in self.cue.events.clone().iter().enumerate() {
            if let Some(EventDescription::TimecodeEvent { time }) = event.event {
                self.render_ltc_marker(i, event, time);
            } else if let Some(EventDescription::TimecodeStopEvent) = event.event {
                self.render_ltc_stop_marker(i, event);
            }
        }

        self.render_ltc_running_blocks();
        self.render_ltc_at_cursor(cursor_pos);
    }

    fn render_ltc_stop_marker(&mut self, i: usize, event: &event::Event) {
        let rect = self.draw_timestop(event);

        self.register_interaction_rect(TimelineInteractable::new(
            "ltc_stop_marker_drag",
            rect,
            i,
            TimelineInteractable::make_event_location_drag(i),
            None,
            None,
        ));
    }

    fn render_ltc_marker(&mut self, i: usize, event: &event::Event, time: Timecode) {
        let rect = self.draw_timestamp(event.location.into(), time);
        self.register_interaction_rect(TimelineInteractable::new(
            "ltc_marker_drag",
            rect,
            i,
            TimelineInteractable::make_event_location_drag(i),
            None,
            None,
        ));
    }

    fn draw_timestop(&mut self, event: &event::Event) -> Rect {
        self.draw_text_in_box(
            pos2(self.x(event.location as usize), self.y_mid(4)),
            self.style.text_color(),
            self.style.window_stroke,
            self.style.extreme_bg_color,
            12.0,
            "LTC STOP",
        )
    }

    fn render_ltc_running_blocks(&mut self) {
        let mut prev_pos = u16::MAX;
        for event in self.cue.events.iter() {
            if let Some(EventDescription::TimecodeEvent { time: _ }) = event.event {
                if prev_pos != u16::MAX {
                    let end = event.location as usize - 1;
                    self.draw_timecode_block(prev_pos, end);
                }
                prev_pos = event.location;
            } else if let Some(EventDescription::TimecodeStopEvent) = event.event {
                if prev_pos != u16::MAX {
                    let end = event.location as usize - 1;
                    self.draw_timecode_block(prev_pos, end);
                }
                prev_pos = u16::MAX;
            }
        }

        if prev_pos != u16::MAX {
            self.draw_timecode_block(prev_pos, self.last_beat());
        }
    }

    fn draw_timecode_block(&self, prev_pos: u16, end: usize) {
        self.draw_dashed_rect(
            self.lane_rect(4, prev_pos as usize, end),
            self.style.window_stroke,
            self.style.window_stroke.color,
            10.0,
        );
    }

    fn render_ltc_at_cursor(&mut self, cursor_pos: usize) {
        let mut time_at_cursor = Timecode::from_frames(0, FrameRate::Fps25).expect("tc 0");
        let mut is_running = false;
        let mut last_before_cursor = 0;
        for event in self.cue.events.iter() {
            if let Some(EventDescription::TimecodeEvent { time }) = event.event {
                if (event.location as usize) < cursor_pos {
                    time_at_cursor = time;
                    last_before_cursor = event.location as usize;
                }
                is_running = true;
            } else if let Some(EventDescription::TimecodeStopEvent) = event.event {
                is_running = false;
            }
        }

        if is_running {
            for beat in &self.cue.beats[last_before_cursor..cursor_pos] {
                time_at_cursor = (time_at_cursor
                    + self
                        .tc_offset_from_seconds(FrameRate::Fps25, beat.length as f64 / 1000000.0))
                .expect("addition");
            }
            self.draw_timestamp(cursor_pos, time_at_cursor);
        }
    }

    // FIXME: this should live in ks-common
    fn tc_offset_from_seconds(&self, fr: FrameRate, seconds_total: f64) -> TimecodeOffset {
        let fps = fr.as_float();
        let hours = seconds_total / 3600.0;
        let minutes = (seconds_total / 60.0) % 60.0;
        let seconds = seconds_total % 60.0;
        let frames = seconds_total.fract() * fps;
        TimecodeOffset::from_raw_fields(
            false,
            hours as u8,
            minutes as u8,
            seconds as u8,
            frames as u8,
            fr,
        )
        .unwrap_or(TimecodeOffset {
            abs_time: Timecode::default(),
            is_negative: false,
        })
    }

    fn draw_timestamp(&mut self, location: usize, time: Timecode) -> Rect {
        self.draw_text_in_box(
            pos2(self.x(location), self.y_mid(4)),
            self.style.text_color(),
            self.style.window_stroke,
            self.style.extreme_bg_color,
            12.0,
            time.to_string(),
        )
    }

    const TRACK_NAMES: [&'static str; 35] = [
        "Regions",
        "Beat ruler",
        "Tempo",
        "Jumps & Repeats",
        "SMPTE Time",
        "Playback channel 1",
        "Playback channel 2",
        "Playback channel 3",
        "Playback channel 4",
        "Playback channel 5",
        "Playback channel 6",
        "Playback channel 7",
        "Playback channel 8",
        "Playback channel 9",
        "Playback channel 10",
        "Playback channel 11",
        "Playback channel 12",
        "Playback channel 13",
        "Playback channel 14",
        "Playback channel 15",
        "Playback channel 16",
        "Playback channel 17",
        "Playback channel 18",
        "Playback channel 19",
        "Playback channel 20",
        "Playback channel 21",
        "Playback channel 22",
        "Playback channel 23",
        "Playback channel 24",
        "Playback channel 25",
        "Playback channel 26",
        "Playback channel 27",
        "Playback channel 28",
        "Playback channel 29",
        "Playback channel 30",
    ];

    fn render_lane_list(&mut self) {
        for (i, text) in Self::TRACK_NAMES.iter().enumerate() {
            let rect = Rect::from_min_max(
                pos2(self.resp.rect.min.x, self.y(i)),
                pos2(self.left(), self.y_end(i)),
            );
            self.painter.rect(
                rect,
                0.0,
                self.style.window_fill,
                self.style.window_stroke,
                egui::StrokeKind::Inside,
            );
            self.draw_fit_text(
                rect,
                Align2::LEFT_CENTER,
                12.0,
                *text,
                self.style.text_color(),
                TextFit::Hide,
            );

            let ccenter = rect.right_top() + rect.height() * vec2(-0.5, 0.5);
            let stroke = self.style.window_stroke();
            self.draw_fold_arrow(i, ccenter, 6.0, stroke);
        }
    }

    fn draw_fold_arrow(&mut self, i: usize, ccenter: Pos2, radius: f32, stroke: Stroke) {
        self.painter.circle_stroke(ccenter, radius, stroke);
        let midpoint_offset = if self.persistent.lane_collapsed[i] {
            Vec2::DOWN
        } else {
            Vec2::UP
        };
        self.painter.line(
            vec![
                ccenter + radius * 0.5 * Vec2::RIGHT,
                ccenter + radius * 0.5 * Vec2::LEFT,
                ccenter + radius * 0.5 * midpoint_offset,
                ccenter + radius * 0.5 * Vec2::RIGHT,
            ],
            stroke,
        );
    }

    fn try_zoom(&self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) {
        if ui.rect_contains_pointer(self.resp.rect) {
            let zoom_step = ui.input(InputState::zoom_delta);
            app.zoom *= zoom_step;
            app.pan.x *= zoom_step;
            app.pan -= ui.input(|i| i.smooth_scroll_delta);
        }

        app.pan = app.pan.clamp(vec2(0.0, 0.0), Vec2::INFINITY);
    }

    fn beat_width_from_length(&self, length: u32) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        let mult = lerp(
            1.0..=length as f32 / 500_000.0_f32,
            self.proportional_scaling,
        );
        self.base_beat_width * mult
    }

    fn handle_interaction(&mut self, app: &mut ClicksEditorApp, ui: &mut egui::Ui) -> Option<()> {
        let clicked = ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary));
        let mouse_just_down = ui.input(|i| i.pointer.primary_pressed());
        let mouse_down = ui.input(|i| i.pointer.primary_down());
        let dragged = mouse_down && ui.input(|i| i.pointer.is_moving());
        let pos = ui.input(|i| i.pointer.interact_pos())?;

        let ongoing = self
            .find_hashed_interaction(app.current_interaction_hash)
            .or(self.handle_hover_click(app, clicked, mouse_just_down, pos));

        let cue = &mut app.project_file.show.cues[app.selected_cue_idx];
        if let Some(interaction) = ongoing {
            ui.ctx().set_cursor_icon(cursor_icon_from_drag(interaction));
            let _res = self.handle_drag(cue, dragged, interaction, pos);
        }

        if !mouse_down {
            app.current_interaction_hash = None;
            cue.events.sort();
            cue.recalculate_tempo_changes();
        }
        None
    }

    fn handle_hover_click(
        &self,
        app: &mut ClicksEditorApp,
        clicked: bool,
        mouse_just_down: bool,
        pos: Pos2,
    ) -> Option<&TimelineInteractable> {
        let hovered = self.find_hovered_interaction(pos);

        if let Some(interaction) = hovered {
            if mouse_just_down {
                app.current_interaction_hash = Some(interaction.hash);
            }
            if clicked && let Some(event_idx) = interaction.event_idx {
                app.selected_event_idx = event_idx;
            }
        }
        hovered
    }

    fn find_hashed_interaction(&self, hash: Option<u64>) -> Option<&TimelineInteractable> {
        let hash = hash?;
        self.interactions.iter().find(|&inter| inter.hash == hash)
    }

    fn find_hovered_interaction(&self, pointer_pos: Pos2) -> Option<&TimelineInteractable> {
        self.interactions
            .iter()
            .find(|&inter| inter.rect.contains(pointer_pos))
    }

    fn handle_drag(
        &self,
        cue: &mut Cue,
        dragged: bool,
        interaction: &TimelineInteractable,
        pos: Pos2,
    ) -> Result<bool, InteractionError> {
        let mut action_happened = false;
        if dragged && let Some(drag_x) = &interaction.drag_x {
            let beat = self
                .beat_at_x(pos.x)
                .saturating_sub(1)
                .try_into()
                .map_err(|_| InteractionError::ArgumentOutOfBounds)?;
            action_happened |= (drag_x)(cue, beat)?;
        }
        if dragged && let Some(drag_y) = &interaction.drag_y {
            let lane = self.lane_at_y(pos.y);
            action_happened = (drag_y)(cue, lane)?;
        }
        Ok(action_happened)
    }
}

fn grad_tempo_event_length_drag_interaction(i: usize) -> InteractionFunctionDragX {
    let event_idx = u8::try_from(i).ok()?;
    Some(Box::new(move |cue, beat| {
        if let Some(event) = cue.events.get_mut(event_idx)
            && let Some(EventDescription::GradualTempoChangeEvent {
                start_tempo: _,
                end_tempo: _,
                length,
            }) = event.event.as_mut()
        {
            *length = beat - event.location;
            return Ok(true);
        }
        Ok(false)
    }))
}

fn jump_event_destination_drag_interaction(i: usize) -> InteractionFunctionDragX {
    let event_idx = i.try_into().ok()?;
    Some(Box::new(move |cue, beat| {
        if let Some(event) = cue.events.get_mut(event_idx)
            && let Some(EventDescription::JumpEvent {
                destination,
                requirement: _,
                when_jumped: _,
                when_passed: _,
            }) = event.event.as_mut()
        {
            *destination = beat;
            return Ok(true);
        }
        Ok(false)
    }))
}

fn cursor_icon_from_drag(interaction: &TimelineInteractable) -> CursorIcon {
    match (interaction.drag_x.is_some(), interaction.drag_y.is_some()) {
        (true, true) => CursorIcon::Move,
        (false, true) => CursorIcon::ResizeVertical,
        (true, false) => CursorIcon::ResizeHorizontal,
        (false, false) => CursorIcon::Default,
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

    let persistent = TimelinePersistent::default();
    let proportional_scaling = app
        .ctx
        .animate_bool("proportional_scaling".into(), app.proportional_beat_length);
    TimelineRenderer::new(app, ui, cue, persistent)
        .with_proportional_scaling(proportional_scaling)
        .show(app, ui);
}
