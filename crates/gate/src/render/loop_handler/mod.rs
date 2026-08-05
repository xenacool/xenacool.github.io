pub mod camera;
pub mod scene;

use web_sys::WebGlRenderingContext as GL;
use pystral_core::history::HistoryManager;
use pystral_core::log::{WorldState, Event};
use pystral_core::animation::ActiveFSM;
use crate::render::context::RenderContext;
use crate::render::state::{PlaybackState, MovementTween, PropertyTween};
use crate::render::update_ui_slider;
use crate::render::utils::interpolate_property;
use crate::AppCommand;
use self::camera::{update_canvas_size, setup_camera};
use self::scene::draw_scene;
use crate::WorkerInput;
use std::sync::mpsc::Receiver;

pub struct LoopHandler {
    pub ctx: RenderContext,
    pub history_manager: HistoryManager,
    pub playback_state: PlaybackState,
    pub app_rx: Receiver<AppCommand>,
    pub worker_tx: futures::channel::mpsc::UnboundedSender<crate::WorkerInput>,
    pub accumulator: f64,
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
        }
    }

    pub fn tick(&mut self) {
        let now = web_sys::window().unwrap().performance().unwrap().now();

        // 0. Process Commands
        self.process_commands();

        // 1. Playback & History Update
        let (is_playing_anims, debug_mode, _delta) = self.update_playback_and_history(now);

        // 2. Get State & Update Logic
        if let Some(state) = self.get_current_state(now, is_playing_anims) {
            // 3. Canvas & Viewport
            let (width, height) = update_canvas_size(&self.ctx);

            // 4. Clear
            self.ctx.gl.clear(GL::COLOR_BUFFER_BIT | GL::DEPTH_BUFFER_BIT);

            // 5. Camera & View Matrices
            let (_view, _proj, cam_right, cam_up, cam_forward) = setup_camera(&mut self.ctx, &self.worker_tx, &state, width, height);

            // Update Nav Buttons based on current camera neighbors
            self.sync_nav_buttons(&state);

            // 6. Draw Scene
            draw_scene(&mut self.ctx, &self.worker_tx, state, cam_right, cam_up, cam_forward, debug_mode, now, is_playing_anims);
        }
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
                }
                AppCommand::TogglePlayAnimations => {
                    self.playback_state.playing_animations = !self.playback_state.playing_animations;
                }
                AppCommand::SetDebugMode(enabled) => {
                    self.playback_state.debug_mode = enabled;
                }
                AppCommand::UpdateHistory(history) => {
                    self.history_manager = history;
                    self.ctx.active_camera_id = None;
                    crate::render::set_ui_slider_max(self.history_manager.log.len() as u32);
                    crate::render::update_ui_slider(self.history_manager.current_index as u32);
                }
                AppCommand::CameraNav(direction) => {
                    let mut target_cam_id = None;
                    
                    let cam = if let Some(id) = self.ctx.active_camera_id {
                        self.history_manager.current_state.entities.iter().find(|e| e.id == id && e.kind == "camera")
                    } else {
                        self.history_manager.current_state.entities.iter().find(|e| e.kind == "camera")
                    };

                    if let Some(cam) = cam {
                        let prop_name = format!("neighbor_{}", direction);
                        if let Some(pystral_core::log::PropertyValue::Float(id)) = cam.properties.get(&prop_name) {
                            target_cam_id = Some(*id as u64);
                        }
                    }

                    if let Some(id) = target_cam_id {
                        self.ctx.active_camera_id = Some(id);
                    } else {
                        let msg = format!("Camera navigation error: No {} neighbor found", direction);
                        let _ = self.worker_tx.unbounded_send(WorkerInput::Log(msg));
                    }
                }
            }
        }
    }


    fn update_playback_and_history(&mut self, now: f64) -> (bool, bool, f64) {
        let delta = now - self.playback_state.last_tick_ms;
        self.playback_state.last_tick_ms = now;
        let is_playing_anims = self.playback_state.playing_animations;
        let debug_mode = self.playback_state.debug_mode;

        if self.playback_state.playing_log {
            self.accumulator += delta;
            if self.accumulator > 100.0 {
                let steps = (self.accumulator / 100.0) as usize;
                self.history_manager.jump_to(self.history_manager.current_index + steps);
                self.accumulator %= 100.0;
                update_ui_slider(self.history_manager.current_index as u32);
            }
        }
        (is_playing_anims, debug_mode, delta)
    }

    fn sync_nav_buttons(&self, state: &WorldState) {
        let mut up = false;
        let mut down = false;
        let mut left = false;
        let mut right = false;

        let cam = if let Some(id) = self.ctx.active_camera_id {
            state.entities.iter().find(|e| e.id == id && e.kind == "camera")
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

    fn get_current_state(&mut self, now: f64, is_playing_anims: bool) -> Option<WorldState> {
        let current_idx = self.history_manager.current_index;
        let state = self.history_manager.current_state.clone();
        
        let event = if let Some(last_idx) = self.ctx.last_index {
            if current_idx == last_idx + 1 {
                Some(self.history_manager.log[last_idx].clone())
            } else {
                None
            }
        } else {
            None
        };

        if self.ctx.last_index != Some(current_idx) {
            if let Some(e) = event {
                let last_index = self.ctx.last_index.unwrap();
                Self::handle_event_tweens_static(&mut self.ctx, &e, &self.history_manager, last_index, now);
            } else {
                self.ctx.movement_tweens.clear();
                self.ctx.property_tweens.clear();
            }
            self.ctx.last_index = Some(current_idx);
            self.update_active_fsms(&state, now);
        }

        let mut state = state;

        if is_playing_anims {
            for fsm in self.ctx.active_fsms.values_mut() {
                fsm.update(now);
            }
        }

        self.apply_fsm_properties(&mut state);
        self.apply_movement_tweens(&mut state, now);
        self.apply_property_tweens(&mut state, now);

        Some(state)
    }

    fn handle_event_tweens_static(ctx: &mut RenderContext, event: &Event, history: &HistoryManager, prev_idx: usize, now: f64) {
        if let Event::MoveSprite { id, destination, duration_ms } = event {
            if let Some(duration) = duration_ms {
                let from_hex = Self::get_prev_entity_hex_static(history, prev_idx, *id).unwrap_or(*destination);
                ctx.movement_tweens.insert(*id, MovementTween {
                    from_hex,
                    to_hex: *destination,
                    start_time_ms: now,
                    duration_ms: *duration as f64,
                });
            }
        } else if let Event::TweenProperty { id, property, value, duration_ms } = event {
            let from_value = Self::get_prev_property_value_static(history, prev_idx, *id, property).unwrap_or_else(|| value.clone());
            ctx.property_tweens.insert((*id, property.clone()), PropertyTween {
                property: property.clone(),
                from_value,
                to_value: value.clone(),
                start_time_ms: now,
                duration_ms: *duration_ms as f64,
            });
        }
    }

    fn get_prev_entity_hex_static(history: &HistoryManager, prev_idx: usize, id: u64) -> Option<hexx::Hex> {
        let mut prev_state = WorldState::default();
        let mut temp_idx = 0;
        let checkpoint = history.checkpoints.iter().filter(|c| c.event_index <= prev_idx).last();
        if let Some(cp) = checkpoint {
            temp_idx = cp.event_index;
            prev_state = cp.state.clone();
        }
        for i in temp_idx..prev_idx {
            prev_state.apply_event(&history.log[i]);
        }
        prev_state.entities.iter().find(|e| e.id == id).map(|e| e.hex)
    }

    fn get_prev_property_value_static(history: &HistoryManager, prev_idx: usize, id: u64, property: &str) -> Option<pystral_core::log::PropertyValue> {
        let mut prev_state = WorldState::default();
        let mut temp_idx = 0;
        let checkpoint = history.checkpoints.iter().filter(|c| c.event_index <= prev_idx).last();
        if let Some(cp) = checkpoint {
            temp_idx = cp.event_index;
            prev_state = cp.state.clone();
        }
        for i in temp_idx..prev_idx {
            prev_state.apply_event(&history.log[i]);
        }
        prev_state.entities.iter().find(|e| e.id == id).and_then(|e| e.properties.get(property).cloned())
    }

    fn update_active_fsms(&mut self, state: &WorldState, now: f64) {
        for entity in &state.entities {
            if let Some(fsm_name) = &entity.fsm_name {
                if let Some(fsm_def) = state.fsms.get(fsm_name) {
                    self.ctx.active_fsms.entry(entity.id)
                        .and_modify(|f| f.transition_to(entity.animation_state.clone(), now))
                        .or_insert_with(|| ActiveFSM::new(fsm_def.clone(), entity.animation_state.clone(), now));
                } else {
                    let msg = format!("FSM definition {} not found for entity {}", fsm_name, entity.id);
                    let _ = self.worker_tx.unbounded_send(WorkerInput::Log(msg));
                }
            }
        }
    }

    fn apply_fsm_properties(&self, state: &mut WorldState) {
        for entity in &mut state.entities {
            if let Some(fsm) = self.ctx.active_fsms.get(&entity.id) {
                for (prop, val) in &fsm.current_properties {
                    entity.properties.insert(prop.clone(), val.clone());
                }
            }
        }
    }

    fn apply_movement_tweens(&self, _state: &mut WorldState, _now: f64) {
        // Movement tweens are applied during drawing in scene.rs
    }

    fn apply_property_tweens(&self, state: &mut WorldState, now: f64) {
        for ((id, _), tween) in &self.ctx.property_tweens {
            if (now - tween.start_time_ms) < tween.duration_ms {
                if let Some(entity) = state.entities.iter_mut().find(|e| e.id == *id) {
                    let t = ((now - tween.start_time_ms) / tween.duration_ms).clamp(0.0, 1.0) as f32;
                    let val = interpolate_property(&tween.from_value, &tween.to_value, t);
                    entity.properties.insert(tween.property.clone(), val);
                }
            }
        }
    }
}
