pub mod camera;
pub mod entity;
mod playback_methods;
pub mod scene;

use self::camera::{setup_camera, update_canvas_size};
use self::scene::draw_scene;
use crate::AppCommand;
use crate::render::context::RenderContext;
use crate::render::state::{MovementTween, PlaybackState, PropertyTween, sequence_ack_due};
use crate::render::update_ui_slider;
use crate::render::utils::interpolate_property;
use crate::{TransientState, WorkerInput};
use pystral_core::animation::ActiveFSM;
use pystral_core::history::HistoryManager;
use pystral_core::log::{Event, WorldState};
use pystral_games::ActionError;
use std::sync::mpsc::Receiver;
use web_sys::WebGlRenderingContext as GL;

pub struct LoopHandler {
    pub ctx: RenderContext,
    pub history_manager: HistoryManager,
    pub playback_state: PlaybackState,
    pub app_rx: Receiver<AppCommand>,
    pub worker_tx: futures::channel::mpsc::UnboundedSender<crate::WorkerInput>,
    pub accumulator: f64,
    pub transient_state: TransientState,
    pub last_action_rejection: Option<(u64, ActionError)>,
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
        }
    }

    pub fn tick(&mut self) {
        let now = web_sys::window()
            .expect("No global window found")
            .performance()
            .expect("Performance object not found")
            .now();

        // 0. Process Commands
        self.process_commands();

        // 1. Playback & History Update
        let (is_playing_anims, debug_mode, delta) = self.update_playback_and_history(now);

        // 2. Get State & Update Logic
        let state = self.get_current_state(now, is_playing_anims);
        self.sync_camera_selection(&state);
        // 3. Canvas & Viewport
        let (width, height) = update_canvas_size(&self.ctx);

        // 4. Clear
        self.ctx
            .gl
            .clear(GL::COLOR_BUFFER_BIT | GL::DEPTH_BUFFER_BIT);

        // 5. Camera & View Matrices
        let (_view, _proj, cam_right, cam_up, cam_forward) =
            setup_camera(&mut self.ctx, &self.worker_tx, &state, width, height, delta);

        // Update Nav Buttons based on current camera neighbors
        self.sync_nav_buttons(&state);

        // Update Action Buttons based on current prompt entity
        self.sync_action_buttons(&state, debug_mode);

        // Acknowledging the visible barrier is latency-sensitive. Do this
        // before optional diagnostics serialization, which may be large.
        self.handle_sequence_number_acks(now);

        // Sync Debug Panels
        self.sync_debug_panels(&state);

        // 6. Draw Scene
        draw_scene(
            &mut self.ctx,
            &self.worker_tx,
            &state,
            &self.transient_state,
            cam_right,
            cam_up,
            cam_forward,
            debug_mode,
            now,
            is_playing_anims,
        );
    }

    fn process_commands(&mut self) {
        // Process App Commands
        while let Ok(cmd) = self.app_rx.try_recv() {
            match cmd {
                AppCommand::SetHistoryIndex(index) => {
                    self.history_manager.jump_to(index as usize);
                    update_ui_slider(index);
                }
                AppCommand::TogglePlayLog => {
                    self.playback_state.playing_log = !self.playback_state.playing_log;
                    self.accumulator = 0.0;
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
                    self.ctx.last_index = None;
                    self.ctx.tween_state = None;
                    self.ctx.movement_tweens.clear();
                    self.ctx.property_tweens.clear();
                    self.ctx.active_camera_id = None;
                    self.ctx.camera_tween = None;
                    self.ctx.camera_pose = None;
                    self.ctx.camera_ids.clear();
                    self.playback_state.last_sequence_ack_sent = None;
                    self.history_manager.jump_to(0);
                    crate::render::set_ui_slider_max(self.history_manager.log.len() as u32);
                    crate::render::update_ui_slider(0);
                }
                AppCommand::AppendHistory(history) => {
                    self.history_manager.append_events(history.log);
                    crate::render::set_ui_slider_max(self.history_manager.log.len() as u32);
                    crate::render::update_ui_slider(self.history_manager.current_index as u32);
                    // A batch can arrive while the render loop is between
                    // ticks.  Acknowledge its visible barrier immediately so
                    // the worker is not dependent on a later animation frame.
                    self.handle_sequence_number_acks(self.playback_state.last_tick_ms);
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
            }
        }
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
            while self.accumulator >= self.playback_state.history_step_ms
                && self.history_manager.current_index < self.history_manager.log.len()
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

        if debug_mode && self.transient_state.available_actions.is_some() {
            visible = true;
        }

        if self.transient_state.game_completed {
            visible = false;
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
        }

        crate::render::update_action_buttons(
            visible, up, down, left, right, layer_up, layer_down, confirm, ret, wait,
        );
    }

    fn sync_debug_panels(&mut self, state: &WorldState) {
        let debug_enabled = self.playback_state.debug_mode;
        let index_changed =
            self.playback_state.last_debug_index != self.history_manager.current_index;
        let log_len_changed =
            self.playback_state.last_history_log_len != self.history_manager.log.len();
        let mode_toggled = self.playback_state.last_debug_mode != debug_enabled;
        let can_publish_diagnostics = !self.transient_state.action_pending;

        if can_publish_diagnostics
            && debug_enabled
            && (index_changed || log_len_changed || mode_toggled)
        {
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
        if can_publish_diagnostics {
            self.playback_state.last_history_log_len = self.history_manager.log.len();
            self.playback_state.last_debug_mode = debug_enabled;
        }
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
            // ACKs are monotonic watermarks. Retry a missing ACK slowly, while
            // avoiding the per-frame flood that can starve simulation.
            if !sequence_ack_due(self.playback_state.last_sequence_ack_sent, n, now) {
                return;
            }
            self.playback_state.last_sequence_ack_sent = Some((n, now));
            let _ = self.worker_tx.unbounded_send(WorkerInput::Ack(n));
        }
    }
}
