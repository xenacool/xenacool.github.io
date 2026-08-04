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
use self::camera::{update_canvas_size, setup_camera};
use self::scene::draw_scene;

pub struct LoopHandler {
    pub ctx: RenderContext,
    pub history_manager: std::sync::Arc<std::sync::Mutex<Option<HistoryManager>>>,
    pub playback_state: std::sync::Arc<std::sync::Mutex<PlaybackState>>,
    pub accumulator: f64,
}

impl LoopHandler {
    pub fn new(
        ctx: RenderContext,
        history_manager: std::sync::Arc<std::sync::Mutex<Option<HistoryManager>>>,
        playback_state: std::sync::Arc<std::sync::Mutex<PlaybackState>>,
    ) -> Self {
        Self {
            ctx,
            history_manager,
            playback_state,
            accumulator: 0.0,
        }
    }

    pub fn tick(&mut self) {
        let now = web_sys::window().unwrap().performance().unwrap().now();

        // 1. Playback & History Update
        let (is_playing_anims, debug_mode, _delta) = self.update_playback_and_history(now);

        // 2. Get State & Update Logic
        if let Some(state) = self.get_current_state(now, is_playing_anims) {
            // 3. Canvas & Viewport
            let (width, height) = update_canvas_size(&self.ctx);

            // 4. Clear
            self.ctx.gl.clear(GL::COLOR_BUFFER_BIT | GL::DEPTH_BUFFER_BIT);

            // 5. Camera & View Matrices
            let (_view, _proj, cam_right, cam_up, cam_forward) = setup_camera(&self.ctx, &state, width, height);

            // 6. Draw Scene
            draw_scene(&mut self.ctx, state, cam_right, cam_up, cam_forward, debug_mode, now, is_playing_anims);
        }
    }

    fn update_playback_and_history(&mut self, now: f64) -> (bool, bool, f64) {
        let mut is_playing_anims = false;
        let mut debug_mode = false;
        let mut delta = 0.0;

        if let Ok(mut pb) = self.playback_state.lock() {
            delta = now - pb.last_tick_ms;
            pb.last_tick_ms = now;
            is_playing_anims = pb.playing_animations;
            debug_mode = pb.debug_mode;

            if pb.playing_log {
                if let Ok(mut history_lock) = self.history_manager.lock() {
                    if let Some(history) = history_lock.as_mut() {
                        self.accumulator += delta;
                        if self.accumulator > 100.0 {
                            let steps = (self.accumulator / 100.0) as usize;
                            history.jump_to(history.current_index + steps);
                            self.accumulator %= 100.0;
                            update_ui_slider(history.current_index as u32);
                        }
                    }
                }
            }
        }
        (is_playing_anims, debug_mode, delta)
    }

    fn get_current_state(&mut self, now: f64, is_playing_anims: bool) -> Option<WorldState> {
        let (state, current_idx, event) = {
            let history_lock = self.history_manager.lock().ok()?;
            let history = history_lock.as_ref()?;
            let state = history.current_state.clone();
            let current_idx = history.current_index;
            let event = if self.ctx.last_index != current_idx && current_idx == self.ctx.last_index + 1 {
                Some(history.log[self.ctx.last_index].clone())
            } else {
                None
            };
            (state, current_idx, event)
        };

        if self.ctx.last_index != current_idx {
            if let Some(e) = event {
                let last_index = self.ctx.last_index;
                let history_lock = self.history_manager.lock().ok()?;
                let history = history_lock.as_ref()?;
                Self::handle_event_tweens_static(&mut self.ctx, &e, history, last_index, now);
            } else {
                self.ctx.movement_tweens.clear();
                self.ctx.property_tweens.clear();
            }
            self.ctx.last_index = current_idx;
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
                    crate::ui_log::ui_log(format!("FSM definition {} not found for entity {}", fsm_name, entity.id));
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
