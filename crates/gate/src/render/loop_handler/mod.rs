mod assets;
pub mod camera;
mod playback_methods;
pub mod scene;
use self::camera::{setup_camera, viewport_size};
use self::playback_methods::playback_events_due;
use self::scene::resolved_entity_world_positions;
use crate::AppCommand;
use crate::render::RenderFrame;
use crate::render::context::RenderContext;
use crate::render::state::{MovementTween, PlaybackState, PropertyTween, sequence_ack_due};
use crate::render::update_ui_slider;
use crate::render::utils::interpolate_property;
use crate::{TransientState, WorkerInput};
use pystral_core::animation::ActiveFSM;
use pystral_core::history::HistoryManager;
use pystral_core::log::{Event, WorldState};
use pystral_games::ActionError;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
pub struct LoopHandler {
    pub ctx: RenderContext,
    pub history_manager: HistoryManager,
    pub playback_state: PlaybackState,
    pub app_rx: Receiver<AppCommand>,
    pub worker_tx: futures::channel::mpsc::UnboundedSender<crate::WorkerInput>,
    pub accumulator: f64,
    pub transient_state: TransientState,
    pub last_action_rejection: Option<(u64, ActionError)>,
    manual_history_index: Option<usize>,
    render_tick: u64,
    profile_last_tick_at: f64,
    profile_samples: u64,
    profile_total_ms: f64,
    profile_max_ms: f64,
    profile_tick_samples: Vec<f64>,
}
impl LoopHandler {
    pub fn new(
        ctx: RenderContext,
        history_manager: HistoryManager,
        app_rx: Receiver<AppCommand>,
        worker_tx: futures::channel::mpsc::UnboundedSender<crate::WorkerInput>,
    ) -> Self {
        Self {
            ctx,
            history_manager,
            playback_state: PlaybackState::default(),
            app_rx,
            worker_tx,
            accumulator: 0.0,
            transient_state: TransientState::default(),
            last_action_rejection: None,
            manual_history_index: None,
            render_tick: 0,
            profile_last_tick_at: 0.0,
            profile_samples: 0,
            profile_total_ms: 0.0,
            profile_max_ms: 0.0,
            profile_tick_samples: Vec::new(),
        }
    }
    pub fn tick(&mut self) {
        let tick_started_at = web_sys::window()
            .expect("No global window found")
            .performance()
            .expect("Performance object not found")
            .now();
        let now = web_sys::window()
            .expect("No global window found")
            .performance()
            .expect("Performance object not found")
            .now();
        let phase_started_at = |performance: &web_sys::Performance| performance.now();
        let performance = web_sys::window()
            .expect("No global window found")
            .performance()
            .expect("Performance object not found");
        let mut phase_ms = [0.0; 7];
        // 0. Process Commands
        let phase_at = phase_started_at(&performance);
        self.process_commands();
        phase_ms[0] = performance.now() - phase_at;
        // 1. Playback & History Update
        let phase_at = phase_started_at(&performance);
        let (is_playing_anims, debug_mode, delta) = self.update_playback_and_history(now);
        phase_ms[1] = performance.now() - phase_at;
        // 2. Get State & Update Logic
        let phase_at = phase_started_at(&performance);
        let state = self.get_current_state(now, is_playing_anims);
        self.sync_camera_selection(&state);
        phase_ms[2] = performance.now() - phase_at;
        // 3. Canvas & Viewport
        let phase_at = phase_started_at(&performance);
        let (width, height) = viewport_size();
        // Publish the resolved pose after camera tween advancement. A native
        // browser renderer consumes this authoritative projection rather than
        // reimplementing Rust camera interpolation and aspect math.
        let (view, proj) =
            setup_camera(&mut self.ctx, &self.worker_tx, &state, width, height, delta);
        let positions =
            resolved_entity_world_positions(&mut self.ctx, &self.worker_tx, &state, now);
        let animation_times = self
            .ctx
            .active_fsms
            .iter()
            .map(|(id, fsm)| {
                (
                    *id,
                    if is_playing_anims {
                        fsm.elapsed_ms(now)
                    } else {
                        0.0
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        self.publish_render_frame(
            &state,
            Some((view, proj)),
            Some(&positions),
            Some(&animation_times),
            now,
        );
        phase_ms[3] = performance.now() - phase_at;
        // 4. HUD state and animation-barrier acknowledgement.
        let phase_at = phase_started_at(&performance);
        // Update Nav Buttons based on current camera neighbors
        self.sync_nav_buttons(&state);
        // Update Action Buttons based on current prompt entity
        self.sync_action_buttons(&state, debug_mode);
        // Acknowledging the visible barrier is latency-sensitive. Do this
        // before optional diagnostics serialization, which may be large.
        self.handle_sequence_number_acks(now);
        phase_ms[4] = performance.now() - phase_at;

        // Sync Debug Panels
        let phase_at = phase_started_at(&performance);
        self.sync_debug_panels(&state);
        phase_ms[5] = performance.now() - phase_at;
        phase_ms[6] = performance.now() - tick_started_at;
        self.publish_profile(tick_started_at, phase_ms);
    }

    fn publish_profile(&mut self, tick_started_at: f64, phase_ms: [f64; 7]) {
        let now = web_sys::window()
            .expect("No global window found")
            .performance()
            .expect("Performance object not found")
            .now();
        let duration_ms = now - tick_started_at;
        self.profile_samples += 1;
        self.profile_total_ms += duration_ms;
        self.profile_max_ms = self.profile_max_ms.max(duration_ms);
        self.profile_tick_samples.push(duration_ms);
        if self.profile_tick_samples.len() > 120 {
            self.profile_tick_samples.remove(0);
        }
        let mut sorted_samples = self.profile_tick_samples.clone();
        sorted_samples.sort_by(f64::total_cmp);
        let p95_index = ((sorted_samples.len() as f64 * 0.95).ceil() as usize)
            .saturating_sub(1)
            .min(sorted_samples.len().saturating_sub(1));
        let profile = serde_json::json!({
            "tick": self.render_tick,
            "samples": self.profile_samples,
            "tick_ms": duration_ms,
            "average_tick_ms": self.profile_total_ms / self.profile_samples as f64,
            "max_tick_ms": self.profile_max_ms,
            "p95_tick_ms": sorted_samples[p95_index],
            "interval_ms": if self.profile_last_tick_at > 0.0 {
                now - self.profile_last_tick_at
            } else {
                0.0
            },
            "phases_ms": {
                "commands": phase_ms[0],
                "playback_history": phase_ms[1],
                "state_logic": phase_ms[2],
                "frame_build_publish": phase_ms[3],
                "hud_and_ack": phase_ms[4],
                "debug_panels": phase_ms[5],
                "measured_total": phase_ms[6]
            },
        });
        self.profile_last_tick_at = now;
        if self.profile_samples == 1 || self.profile_samples.is_multiple_of(30) {
            if let Ok(json) = serde_json::to_string(&profile) {
                crate::render::publish_render_worker_profile(&json);
            }
        }
    }

    fn publish_render_frame(
        &mut self,
        state: &WorldState,
        camera_pose: Option<(glam::Mat4, glam::Mat4)>,
        positions: Option<&HashMap<u64, [f32; 3]>>,
        animation_times: Option<&HashMap<u64, f32>>,
        now: f64,
    ) {
        let camera_pose =
            camera_pose.map(|(view, projection)| crate::render::RenderCameraPoseFrame {
                view: view.to_cols_array(),
                projection: projection.to_cols_array(),
            });
        let mut frame = RenderFrame::from_world_state_with_camera_positions_and_animation(
            state,
            self.render_tick,
            camera_pose,
            positions,
            animation_times,
        );
        for entity in &mut frame.entities {
            if let Some(tween) = self
                .ctx
                .movement_tweens
                .get(&entity.id)
                .filter(|tween| now - tween.start_time_ms < tween.duration_ms)
            {
                entity.animation_state = "walk".to_string();
                entity.animation_clip = entity.walk_animation_clip.clone();
                if let Some((from, to, _)) = tween.segment_at(now - tween.start_time_ms)
                    && let Some(facing) = pystral_games::Facing::from_step(from.hex, to.hex)
                {
                    entity.facing = facing.as_property().to_string();
                }
            }
        }
        frame.waypoint_preview = self
            .transient_state
            .preview
            .as_ref()
            .map(|preview| crate::render::waypoint_preview(state, preview));
        let mut movement_tweens = self
            .ctx
            .movement_tweens
            .iter()
            .map(
                |(entity_id, tween)| crate::render::RenderMovementTweenDebug {
                    entity_id: *entity_id,
                    event_index: tween.event_index,
                    playback_epoch: tween.playback_epoch,
                    duration_ms: tween.duration_ms,
                    completed: now - tween.start_time_ms >= tween.duration_ms,
                    path: tween
                        .path
                        .iter()
                        .map(|waypoint| [waypoint.hex.x, waypoint.hex.y, waypoint.layer])
                        .collect(),
                },
            )
            .collect::<Vec<_>>();
        movement_tweens.sort_by_key(|tween| tween.entity_id);
        frame.presentation_debug = Some(crate::render::RenderPlaybackDebugFrame {
            presentation_clock: self.render_tick,
            history_index: self.history_manager.current_index,
            playback_epoch: self.playback_state.playback_epoch,
            playing_log: self.playback_state.playing_log,
            last_sent_animation_ack: self
                .playback_state
                .last_sequence_ack_sent
                .map(|(sequence, _)| sequence),
            movement_tweens,
        });
        self.render_tick = self.render_tick.saturating_add(1);
        if let Ok(json) = serde_json::to_string(&frame) {
            crate::render::publish_render_frame(&json);
        }
    }

    fn process_commands(&mut self) {
        // Process App Commands
        while let Ok(cmd) = self.app_rx.try_recv() {
            match cmd {
                AppCommand::SetHistoryIndex(index) => {
                    self.history_manager.jump_to(index as usize);
                    // A direct scrub is an explicit request to inspect a
                    // stable point in history.  Stop the log clock before
                    // publishing the cursor so the next render tick cannot
                    // immediately move the highlight away from the chosen
                    // entry.
                    self.playback_state.playing_log = false;
                    self.accumulator = 0.0;
                    self.manual_history_index = Some(self.history_manager.current_index);
                    self.invalidate_presentation_overlays();
                    update_ui_slider(index);
                }
                AppCommand::TogglePlayLog => {
                    self.playback_state.playing_log = !self.playback_state.playing_log;
                    self.accumulator = 0.0;
                    self.invalidate_presentation_overlays();
                    if self.playback_state.playing_log {
                        // Explicit playback resumes live-follow behavior.
                        self.manual_history_index = None;
                    }
                }
                AppCommand::TogglePlayAnimations => {
                    self.playback_state.playing_animations =
                        !self.playback_state.playing_animations;
                }
                AppCommand::SetDebugMode(enabled) => {
                    self.playback_state.debug_mode = enabled;
                }
                AppCommand::SetHistoryStepMs(value) => {
                    self.playback_state.history_step_ms = value.clamp(1.0, 10_000.0);
                    self.accumulator = 0.0;
                }
                AppCommand::UpdateHistory(history) => {
                    self.history_manager = *history;
                    if let Ok(json) = serde_json::to_string(&assets::initial_character_assets(
                        &self.history_manager.log,
                    )) {
                        crate::render::set_initial_actor_assets(&json);
                    }
                    // A new match starts a new static-scene publication
                    // epoch; the first frame must carry map/material data.
                    self.render_tick = 0;
                    self.ctx.last_index = None;
                    self.ctx.tween_state = None;
                    self.ctx.movement_tweens.clear();
                    self.ctx.property_tweens.clear();
                    self.ctx.active_camera_id = None;
                    self.ctx.camera_tween = None;
                    self.ctx.camera_pose = None;
                    self.ctx.camera_ids.clear();
                    self.playback_state.last_sequence_ack_sent = None;
                    self.playback_state.playback_epoch =
                        self.playback_state.playback_epoch.saturating_add(1);
                    self.manual_history_index = None;
                    self.history_manager.jump_to(0);
                    crate::render::set_ui_slider_max(self.history_manager.log.len() as u32);
                    crate::render::update_ui_slider(0);
                    if let Ok(json) = serde_json::to_string(&self.history_manager.log) {
                        crate::render::update_action_log(&json);
                    }
                }
                AppCommand::AppendHistory(history) => {
                    self.history_manager.append_events(history.log);
                    // Every appended batch is live gameplay output. Follow
                    // its authoritative tail before considering ACKs; a
                    // stale history cursor can make the worker appear stalled
                    // after a lethal action even though the player boundary
                    // has already been published. Manual scrubbing remains
                    // available after this batch is rendered.
                    if self.manual_history_index.is_none() {
                        self.history_manager.jump_to(self.history_manager.log.len());
                    } else if let Some(index) = self.manual_history_index {
                        self.history_manager
                            .jump_to(index.min(self.history_manager.log.len()));
                    }
                    crate::render::set_ui_slider_max(self.history_manager.log.len() as u32);
                    crate::render::update_ui_slider(self.history_manager.current_index as u32);
                    if let Ok(json) = serde_json::to_string(&self.history_manager.log) {
                        crate::render::update_action_log(&json);
                    }
                }
                AppCommand::CameraNav(direction) => {
                    let mut target_cam_id = None;

                    if self.ctx.active_camera_id.is_none() {
                        if let Some(first_cam) = self
                            .history_manager
                            .current_state
                            .entities
                            .iter()
                            .find(|e| e.kind == "camera")
                        {
                            self.ctx.active_camera_id = Some(first_cam.id);
                        }
                    }

                    let cam = if let Some(id) = self.ctx.active_camera_id {
                        self.history_manager
                            .current_state
                            .entities
                            .iter()
                            .find(|e| e.id == id && e.kind == "camera")
                    } else {
                        None
                    };

                    if let Some(cam) = cam {
                        let prop_name = format!("neighbor_{}", direction);
                        if let Some(val) = cam.properties.get(&prop_name) {
                            match val {
                                pystral_core::log::PropertyValue::Float(id) => {
                                    target_cam_id = Some(*id as u64)
                                }
                                pystral_core::log::PropertyValue::String(id_str) => {
                                    if let Ok(id) = id_str.parse::<u64>() {
                                        target_cam_id = Some(id);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    if let Some(id) = target_cam_id {
                        let msg = format!("Switched to camera {}", id);
                        let _ = self.worker_tx.unbounded_send(WorkerInput::LogInfo(msg));
                        self.ctx.active_camera_id = Some(id);
                    } else {
                        let msg = format!(
                            "Camera navigation error: No {} neighbor found for camera {:?}",
                            direction, self.ctx.active_camera_id
                        );
                        let _ = self.worker_tx.unbounded_send(WorkerInput::LogError(msg));
                    }
                }
                AppCommand::ActionNav(direction) => {
                    let _ = self
                        .worker_tx
                        .unbounded_send(WorkerInput::ActionNav(direction));
                }
                AppCommand::UpdateTransientState(state) => {
                    self.transient_state = *state;
                }
                AppCommand::ActionRejected { request_id, reason } => {
                    self.last_action_rejection = Some((request_id, reason));
                }
                AppCommand::AnimationCompleted(barrier_id) => {
                    // Completion is an edge from the browser mixer, not a
                    // frame-count guess. Keep the ACK watermark monotonic.
                    if self
                        .playback_state
                        .last_sequence_ack_sent
                        .is_none_or(|(sent, _)| barrier_id > sent)
                    {
                        self.playback_state.last_sequence_ack_sent =
                            Some((barrier_id, self.playback_state.last_tick_ms));
                        let _ = self.worker_tx.unbounded_send(WorkerInput::Ack(barrier_id));
                    }
                }
            }
        }
    }
    /// A scrub or play-state transition starts a new presentation timeline.
    /// Authoritative history remains intact; only wall-clock visual overlays
    /// are discarded, so an old path cannot resume after a cursor change.
    fn invalidate_presentation_overlays(&mut self) {
        self.playback_state.playback_epoch = self.playback_state.playback_epoch.saturating_add(1);
        self.ctx.movement_tweens.clear();
        self.ctx.property_tweens.clear();
        self.ctx.tween_state = None;
        self.ctx.last_index = Some(self.history_manager.current_index);
    }

    fn update_playback_and_history(&mut self, now: f64) -> (bool, bool, f64) {
        let delta = now - self.playback_state.last_tick_ms;
        self.playback_state.last_tick_ms = now;
        let is_playing_anims = self.playback_state.playing_animations;
        let debug_mode = self.playback_state.debug_mode;

        let transition_active = self
            .ctx
            .movement_tweens
            .values()
            .any(|tween| now - tween.start_time_ms < tween.duration_ms)
            || self
                .ctx
                .property_tweens
                .values()
                .any(|tween| now - tween.start_time_ms < tween.duration_ms)
            || self.ctx.camera_tween.is_some();
        // History playback is the authoritative clock.  Presentation tweens
        // are overlays and must not throttle it: a long simulation log can
        // otherwise spend 500 ms on every movement and leave the worker
        // waiting behind an animation that is no longer relevant to the
        // current state.
        if self.playback_state.playing_log {
            self.accumulator += delta;
            // A catch-up loop can consume an entire movement transition after
            // one delayed render tick. Playback is a presentation clock, so
            // preserve the happens-before edge between each history event and
            // its visible tween instead of skipping over it.
            if playback_events_due(
                self.accumulator,
                self.playback_state.history_step_ms,
                self.history_manager.current_index,
                self.history_manager.log.len(),
            ) == 1
            {
                self.history_manager
                    .jump_to(self.history_manager.current_index + 1);
                self.accumulator -= self.playback_state.history_step_ms;
                update_ui_slider(self.history_manager.current_index as u32);
            }
        } else if transition_active {
            self.accumulator = 0.0;
        }
        (is_playing_anims, debug_mode, delta)
    }
    fn sync_nav_buttons(&self, state: &WorldState) {
        let mut up = false;
        let mut down = false;
        let mut left = false;
        let mut right = false;
        let cam = if let Some(id) = self.ctx.active_camera_id {
            state
                .entities
                .iter()
                .find(|e| e.id == id && e.kind == "camera")
        } else {
            state.entities.iter().find(|e| e.kind == "camera")
        };

        if let Some(cam) = cam {
            up = cam.properties.contains_key("neighbor_up");
            down = cam.properties.contains_key("neighbor_down");
            left = cam.properties.contains_key("neighbor_left");
            right = cam.properties.contains_key("neighbor_right");
        }

        crate::render::update_nav_buttons(up, down, left, right);
    }

    fn sync_camera_selection(&mut self, state: &WorldState) {
        let present: Vec<u64> = state
            .entities
            .iter()
            .filter(|entity| entity.kind == "camera")
            .map(|entity| entity.id)
            .collect();

        self.ctx.camera_ids.retain(|id| present.contains(id));
        for id in present {
            if !self.ctx.camera_ids.contains(&id) {
                self.ctx.camera_ids.push(id);
            }
        }

        if self
            .ctx
            .active_camera_id
            .is_none_or(|id| !self.ctx.camera_ids.contains(&id))
        {
            self.ctx.active_camera_id = self.ctx.camera_ids.first().copied();
        }

        if let Some(id) = self.ctx.active_camera_id {
            self.ctx.camera_ids.retain(|candidate| *candidate != id);
            self.ctx.camera_ids.insert(0, id);
        }

        if self.ctx.camera_ids.is_empty() {
            self.ctx.active_camera_id = None;
            self.ctx.camera_tween = None;
            self.ctx.camera_pose = None;
        }
    }

    fn sync_action_buttons(&self, state: &WorldState, debug_mode: bool) {
        let mut visible = false;
        let mut up = false;
        let mut down = false;
        let mut left = false;
        let mut right = false;
        let mut layer_up = false;
        let mut layer_down = false;
        let mut confirm = false;
        let mut ret = false;
        let mut wait = false;
        let mut face = self.transient_state.facing_pending;

        if let Some(prompt) = state.entities.iter().find(|e| e.kind == "prompt") {
            visible = match prompt.properties.get("visible") {
                Some(pystral_core::log::PropertyValue::String(s)) => s == "true",
                _ => false,
            };
            if visible {
                let get_bool = |name: &str| -> bool {
                    match prompt.properties.get(name) {
                        Some(pystral_core::log::PropertyValue::String(s)) => s == "true",
                        _ => false,
                    }
                };
                up = get_bool("up");
                down = get_bool("down");
                left = get_bool("left");
                right = get_bool("right");
                confirm = get_bool("confirm");
                ret = get_bool("return");
                wait = get_bool("wait");
            }
        }

        if self.transient_state.preview.is_some() {
            visible = true;
            up = true;
            down = true;
            left = true;
            right = true;
            layer_up = true;
            layer_down = true;
            confirm = true;
            ret = true;
        }

        if self.transient_state.ability_targets.is_some() {
            visible = true;
            up = true;
            down = true;
            left = true;
            right = true;
            layer_up = true;
            layer_down = true;
            ret = true;
        }

        if self.transient_state.facing_pending {
            visible = true;
            wait = false;
        }

        if debug_mode && self.transient_state.available_actions.is_some() {
            visible = true;
        }

        if self.transient_state.game_completed {
            visible = false;
        }

        if !self.transient_state.input_enabled {
            visible = false;
            up = false;
            down = false;
            left = false;
            right = false;
            layer_up = false;
            layer_down = false;
            confirm = false;
            ret = false;
            wait = false;
            face = false;
        }

        if self.transient_state.action_pending {
            visible = true;
            up = false;
            down = false;
            left = false;
            right = false;
            layer_up = false;
            layer_down = false;
            confirm = false;
            ret = false;
            wait = false;
            face = false;
        }

        crate::render::update_action_buttons(
            visible, up, down, left, right, layer_up, layer_down, confirm, ret, wait, face,
        );
    }

    fn sync_debug_panels(&mut self, state: &WorldState) {
        let debug_enabled = self.playback_state.debug_mode;
        let index_changed =
            self.playback_state.last_debug_index != self.history_manager.current_index;
        let log_len_changed =
            self.playback_state.last_history_log_len != self.history_manager.log.len();
        let mode_toggled = self.playback_state.last_debug_mode != debug_enabled;
        // Diagnostics are a read-only projection of the authoritative frame.
        // Keep publishing them while an action/animation is pending so a
        // selected Diagnostics panel never becomes visible-but-empty during
        // the NPC-engine trampoline.
        if debug_enabled && (index_changed || log_len_changed || mode_toggled) {
            // Push Entity Data
            if let Ok(json) = serde_json::to_string(&state.entities) {
                crate::render::update_entity_viewer(&json);
            }

            // Push History Log Data
            if log_len_changed || mode_toggled {
                if let Ok(json) = serde_json::to_string(&self.history_manager.log) {
                    crate::render::update_history_log(&json);
                }
            }

            self.playback_state.last_debug_index = self.history_manager.current_index;

            // When mode is toggled or log changed, ensure highlighting is correct
            if mode_toggled || log_len_changed {
                crate::render::update_ui_slider(self.history_manager.current_index as u32);
            }
        }

        // Export invalidation is independent of whether the debug viewers are
        // visible.  Leaving this watermark at zero while debug was closed
        // made `log_len_changed` true on every frame and serialized the full
        // HistoryManager continuously.  Opening debug happened to update the
        // watermark, which is why it appeared to make the front end faster.
        self.playback_state.last_history_log_len = self.history_manager.log.len();
        self.playback_state.last_debug_mode = debug_enabled;
    }

    fn handle_sequence_number_acks(&mut self, now: f64) {
        let current_idx = self.history_manager.current_index;
        if current_idx == 0 {
            return;
        }

        // A history update can contain state/log events after its animation
        // barrier.  Acknowledging only `log[current_idx - 1]` then leaves the
        // runtime waiting forever even though the history is fully visible.
        // Find the newest barrier at or before the rendered history index.
        let barrier = self.history_manager.log[..current_idx]
            .iter()
            .rev()
            .find_map(|event| match event {
                pystral_core::log::Event::SequenceNumber(n) => Some(*n),
                _ => None,
            });
        if let Some(n) = barrier {
            // Movement barriers release only after the movement tween has
            // completed. AppendHistory is processed before state/tween
            // resolution in a render tick; acknowledging here would trigger
            // the next gameplay action before the tween completes.
            if self
                .ctx
                .movement_tweens
                .values()
                .any(|tween| now - tween.start_time_ms < tween.duration_ms)
            {
                return;
            }
            let one_shot_pending =
                self.history_manager
                    .current_state
                    .entities
                    .iter()
                    .any(|entity| {
                        matches!(entity.properties.get("animation_barrier"),
                    Some(pystral_core::log::PropertyValue::Float(value)) if *value as u64 == n)
                    });
            if one_shot_pending {
                return;
            }
            if !sequence_ack_due(self.playback_state.last_sequence_ack_sent, n, now) {
                return;
            }
            self.playback_state.last_sequence_ack_sent = Some((n, now));
            let _ = self.worker_tx.unbounded_send(WorkerInput::Ack(n));
            self.ctx
                .movement_tweens
                .retain(|_, tween| now - tween.start_time_ms < tween.duration_ms);
        }
    }
}
