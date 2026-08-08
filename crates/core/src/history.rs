use crate::log::{Checkpoint, EntityState, Event, WorldState};
use serde::{Deserialize, Serialize};

impl WorldState {
    pub fn apply_event(&mut self, event: &Event) {
        match event {
            Event::SpawnEntity { id, kind, hex } => {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(
                    &format!("apply_event: SpawnEntity id={} kind={}", id, kind).into(),
                );
                self.entities
                    .push(EntityState::new(*id, kind.clone(), *hex));
            }
            Event::DespawnEntity { id } => {
                self.entities.retain(|e| e.id != *id);
            }
            Event::MoveSprite {
                id, destination, ..
            } => {
                if let Some(entity) = self.entities.iter_mut().find(|e| e.id == *id) {
                    entity.hex = *destination;
                }
            }
            Event::ConfigureTransition { id, config } => {
                self.transition_configs.insert(*id, config.clone());
            }
            Event::UpdateProperty {
                id,
                property,
                value,
            } => {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(
                    &format!(
                        "apply_event: UpdateProperty id={} prop={} val={:?}",
                        id, property, value
                    )
                    .into(),
                );
                if let Some(entity) = self.entities.iter_mut().find(|e| e.id == *id) {
                    entity.properties.insert(property.clone(), value.clone());
                    if property == "fsm"
                        && let crate::log::PropertyValue::String(s) = value
                    {
                        entity.fsm_name = Some(s.clone());
                    }
                }
            }
            Event::SetAnimationState { id, state } => {
                if let Some(entity) = self.entities.iter_mut().find(|e| e.id == *id) {
                    entity.animation_state.clone_from(state);
                }
            }
            Event::DefineMaterial { name, material } => {
                self.materials.insert(name.clone(), material.clone());
            }
            Event::DefineFSM { name, definition } => {
                self.fsms.insert(name.clone(), definition.clone());
            }
            Event::TweenProperty {
                id,
                property,
                value,
                ..
            } => {
                if let Some(entity) = self.entities.iter_mut().find(|e| e.id == *id) {
                    entity.properties.insert(property.clone(), value.clone());
                }
            }
            Event::DefineAssetCollection { name, data } => {
                self.asset_collections.insert(name.clone(), data.clone());
            }
            Event::Log { .. } => {}
            Event::Segno(_) => {}
        }
    }

    pub fn revert_event(
        &mut self,
        event: &Event,
        previous_entity_state: Option<EntityState>,
        previous_material: Option<crate::domain::Material>,
        previous_fsm: Option<crate::animation::InactiveFSMDefinition>,
    ) {
        match event {
            Event::SpawnEntity { id, .. } => {
                self.entities.retain(|e| e.id != *id);
            }
            Event::DespawnEntity { .. } => {
                if let Some(prev) = previous_entity_state {
                    self.entities.push(prev);
                }
            }
            Event::MoveSprite { id, .. } => {
                if let (Some(prev), Some(entity)) = (
                    &previous_entity_state,
                    self.entities.iter_mut().find(|e| e.id == *id),
                ) {
                    entity.hex = prev.hex;
                }
            }
            Event::ConfigureTransition { id, .. } => {
                self.transition_configs.remove(id);
            }
            Event::UpdateProperty { id, property, .. } => {
                if let (Some(entity), Some(prev)) = (
                    self.entities.iter_mut().find(|e| e.id == *id),
                    &previous_entity_state,
                ) {
                    if let Some(prev_val) = prev.properties.get(property) {
                        entity.properties.insert(property.clone(), prev_val.clone());
                    } else {
                        entity.properties.remove(property);
                    }
                    if property == "fsm" {
                        entity.fsm_name = prev.fsm_name.clone();
                    }
                }
            }
            Event::SetAnimationState { id, .. } => {
                if let (Some(entity), Some(prev)) = (
                    self.entities.iter_mut().find(|e| e.id == *id),
                    &previous_entity_state,
                ) {
                    entity.animation_state.clone_from(&prev.animation_state);
                }
            }
            Event::DefineMaterial { name, .. } => {
                if let Some(prev) = previous_material {
                    self.materials.insert(name.clone(), prev);
                } else {
                    self.materials.remove(name);
                }
            }
            Event::DefineFSM { name, .. } => {
                if let Some(prev) = previous_fsm {
                    self.fsms.insert(name.clone(), prev);
                } else {
                    self.fsms.remove(name);
                }
            }
            Event::TweenProperty { id, property, .. } => {
                if let (Some(entity), Some(prev)) = (
                    self.entities.iter_mut().find(|e| e.id == *id),
                    &previous_entity_state,
                ) {
                    if let Some(prev_val) = prev.properties.get(property) {
                        entity.properties.insert(property.clone(), prev_val.clone());
                    } else {
                        entity.properties.remove(property);
                    }
                }
            }
            Event::DefineAssetCollection { name, .. } => {
                self.asset_collections.remove(name);
            }
            Event::Log { .. } => {}
            Event::Segno(_) => {}
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryManager {
    pub current_state: WorldState,
    pub log: Vec<Event>,
    pub checkpoints: Vec<Checkpoint>,
    pub current_index: usize,
    pub checkpoint_interval: usize,
}

impl HistoryManager {
    pub fn new() -> Self {
        Self {
            current_state: WorldState::default(),
            log: Vec::new(),
            checkpoints: Vec::new(),
            current_index: 0,
            checkpoint_interval: 10,
        }
    }

    pub fn push_and_apply(&mut self, event: Event) {
        #[cfg(target_arch = "wasm32")]
        if let Event::UpdateProperty {
            id,
            property,
            value,
        } = &event
        {
            if *id >= 100 && *id <= 101 && property == "angle" {
                web_sys::console::log_1(
                    &format!("push_and_apply: angle for {} set to {:?}", id, value).into(),
                );
            }
        }
        if self.current_index < self.log.len() {
            self.log.truncate(self.current_index);
            self.checkpoints
                .retain(|c| c.event_index <= self.current_index);
        }

        self.current_state.apply_event(&event);
        self.log.push(event);
        self.current_index += 1;

        if self.current_index % self.checkpoint_interval == 0 {
            self.checkpoints.push(Checkpoint {
                event_index: self.current_index,
                state: self.current_state.clone(),
            });
        }
    }

    pub fn jump_to(&mut self, target_index: usize) {
        let target_index = target_index.min(self.log.len());

        // Find the nearest checkpoint before or at the target index
        let checkpoint = self
            .checkpoints
            .iter()
            .rfind(|c| c.event_index <= target_index);

        let mut state = if let Some(cp) = checkpoint {
            self.current_index = cp.event_index;
            cp.state.clone()
        } else {
            self.current_index = 0;
            WorldState::default()
        };

        // Replay from the checkpoint to the target index
        for i in self.current_index..target_index {
            state.apply_event(&self.log[i]);
        }

        self.current_state = state;
        self.current_index = target_index;
    }

    pub fn undo(&mut self) {
        if self.current_index > 0 {
            self.jump_to(self.current_index - 1);
        }
    }

    pub fn redo(&mut self) {
        if self.current_index < self.log.len() {
            self.jump_to(self.current_index + 1);
        }
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}
