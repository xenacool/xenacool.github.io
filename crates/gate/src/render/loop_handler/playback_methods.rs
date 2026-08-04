use super::*;

fn forward_event_range(
    previous_index: Option<usize>,
    current_index: usize,
    log_len: usize,
) -> Option<std::ops::Range<usize>> {
    let start = previous_index.unwrap_or(0);
    (current_index > start).then(|| start..current_index.min(log_len))
}

impl LoopHandler {
    pub(crate) fn get_current_state(&mut self, now: f64, is_playing_anims: bool) -> WorldState {
        // Completed property tweens must not keep the history clock paused.
        // Movement tweens are pruned by scene rendering; property tweens are
        // owned here because they are applied to the state before drawing.
        self.ctx
            .property_tweens
            .retain(|_, tween| now - tween.start_time_ms < tween.duration_ms);
        let current_idx = self.history_manager.current_index;
        let state = self.history_manager.current_state.clone();

        if self.ctx.last_index != Some(current_idx) {
            if let Some(event_range) = forward_event_range(
                self.ctx.last_index,
                current_idx,
                self.history_manager.log.len(),
            ) {
                let mut previous_state = if self.ctx.last_index == Some(event_range.start) {
                    self.ctx
                        .tween_state
                        .clone()
                        .unwrap_or_else(|| Self::state_at(&self.history_manager, event_range.start))
                } else {
                    Self::state_at(&self.history_manager, event_range.start)
                };
                for event_index in event_range {
                    let event = &self.history_manager.log[event_index];
                    Self::handle_event_tweens_static(&mut self.ctx, event, &previous_state, now);
                    previous_state.apply_event(event);
                }
                self.ctx.tween_state = Some(previous_state);
            } else {
                self.ctx.movement_tweens.clear();
                self.ctx.property_tweens.clear();
                self.ctx.tween_state = None;
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

        Self::apply_fsm_properties(&self.ctx, &mut state);
        Self::apply_movement_tweens(&self.ctx, &mut state, now);
        Self::apply_property_tweens(&self.ctx, &mut state, now);

        state
    }

    fn handle_event_tweens_static(
        ctx: &mut RenderContext,
        event: &Event,
        previous_state: &WorldState,
        now: f64,
    ) {
        if let Event::MoveSprite {
            id,
            destination,
            transition,
        } = event
        {
            if let Some(transition) = transition {
                let from_hex = previous_state
                    .entities
                    .iter()
                    .find(|entity| entity.id == *id)
                    .map(|entity| entity.hex)
                    .unwrap_or(*destination);
                ctx.movement_tweens.insert(
                    *id,
                    MovementTween {
                        from_hex,
                        to_hex: *destination,
                        start_time_ms: now,
                        duration_ms: f64::from(transition.duration_ms),
                        transition: transition.clone(),
                        tweeners: None,
                    },
                );
            }
        } else if let Event::TweenProperty {
            id,
            property,
            value,
            transition,
        } = event
        {
            let from_value = previous_state
                .entities
                .iter()
                .find(|entity| entity.id == *id)
                .and_then(|entity| entity.properties.get(property).cloned())
                .unwrap_or_else(|| value.clone());
            ctx.property_tweens.insert(
                (*id, property.clone()),
                PropertyTween {
                    property: property.clone(),
                    from_value,
                    to_value: value.clone(),
                    start_time_ms: now,
                    duration_ms: f64::from(transition.duration_ms),
                },
            );
        }
    }

    fn state_at(history: &HistoryManager, target_index: usize) -> WorldState {
        let mut prev_state = WorldState::default();
        let mut temp_idx = 0;
        let checkpoint = history
            .checkpoints
            .iter()
            .rfind(|c| c.event_index <= target_index);
        if let Some(cp) = checkpoint {
            temp_idx = cp.event_index;
            prev_state = cp.state.clone();
        }
        for i in temp_idx..target_index {
            prev_state.apply_event(&history.log[i]);
        }
        prev_state
    }

    fn update_active_fsms(&mut self, state: &WorldState, now: f64) {
        for entity in &state.entities {
            if let Some(fsm_name) = &entity.fsm_name {
                if let Some(fsm_def) = state.fsms.get(fsm_name) {
                    self.ctx
                        .active_fsms
                        .entry(entity.id)
                        .and_modify(|f| f.transition_to(entity.animation_state.clone(), now))
                        .or_insert_with(|| {
                            ActiveFSM::new(fsm_def.clone(), entity.animation_state.clone(), now)
                        });
                } else {
                    let msg = format!(
                        "FSM definition {} not found for entity {}",
                        fsm_name, entity.id
                    );
                    let _ = self.worker_tx.unbounded_send(WorkerInput::LogError(msg));
                }
            }
        }
    }

    fn apply_fsm_properties(ctx: &RenderContext, state: &mut WorldState) {
        for entity in &mut state.entities {
            if let Some(fsm) = ctx.active_fsms.get(&entity.id) {
                for (prop, val) in &fsm.current_properties {
                    entity.properties.insert(prop.clone(), val.clone());
                }
            }
        }
    }

    fn apply_movement_tweens(_ctx: &RenderContext, _state: &mut WorldState, _now: f64) {
        // Movement tweens are applied during drawing in scene.rs
    }

    fn apply_property_tweens(ctx: &RenderContext, state: &mut WorldState, now: f64) {
        for ((id, _), tween) in &ctx.property_tweens {
            if (now - tween.start_time_ms) < tween.duration_ms
                && let Some(entity) = state.entities.iter_mut().find(|e| e.id == *id)
            {
                let t = ((now - tween.start_time_ms) / tween.duration_ms).clamp(0.0, 1.0) as f32;
                let val = interpolate_property(&tween.from_value, &tween.to_value, t);
                entity.properties.insert(tween.property.clone(), val);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::forward_event_range;

    #[test]
    fn initial_history_delta_includes_the_first_event() {
        assert_eq!(forward_event_range(None, 1, 4), Some(0..1));
    }

    #[test]
    fn batched_history_delta_includes_every_new_event() {
        assert_eq!(forward_event_range(Some(2), 6, 10), Some(2..6));
    }

    #[test]
    fn backward_scrubs_do_not_start_forward_tweens() {
        assert_eq!(forward_event_range(Some(6), 2, 10), None);
    }
}
