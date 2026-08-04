use crate::tasks::merged_state;
use crate::{
    AbilityDelivery, AbilityId, CollisionWorld, DerivedStat, Logger, ModifierCard,
    ModifierStacking, TacticalAccess, TacticalAccessMut, TacticalDisplayAction, TacticalDomain,
    TagRegistry, TimedModifier, calculate_damage,
};
pub use npc_engine_core::{
    AgentId, Context, ContextMut, Task, TaskDuration, impl_task_boxed_methods,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AbilityTask {
    pub agent: AgentId,
    pub target: AgentId,
    pub ability_id: AbilityId,
    pub ap_cost: u8,
    pub collision_world: Option<Arc<CollisionWorld>>,
}

impl std::fmt::Debug for AbilityTask {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AbilityTask")
            .field("agent", &self.agent)
            .field("target", &self.target)
            .field("ability_id", &self.ability_id)
            .field("ap_cost", &self.ap_cost)
            .finish()
    }
}

impl std::hash::Hash for AbilityTask {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.agent.hash(state);
        self.target.hash(state);
        self.ability_id.hash(state);
        self.ap_cost.hash(state);
    }
}

impl PartialEq for AbilityTask {
    fn eq(&self, other: &Self) -> bool {
        self.agent == other.agent
            && self.target == other.target
            && self.ability_id == other.ability_id
            && self.ap_cost == other.ap_cost
    }
}

impl Eq for AbilityTask {}

impl Task<TacticalDomain> for AbilityTask {
    fn duration(&self, _ctx: Context<TacticalDomain>) -> TaskDuration {
        0
    }

    fn execute(
        &self,
        mut ctx: ContextMut<TacticalDomain>,
    ) -> Option<Box<dyn Task<TacticalDomain>>> {
        let mut state = ctx.state_diff.initial_state.clone();
        for (&id, changed) in &ctx.state_diff.diff.agents {
            state.agents.insert(id, changed.clone());
        }
        if !crate::tasks::ability_target_is_legal_with_world(
            &state,
            self.agent,
            self.target,
            self.ability_id,
            self.collision_world.as_deref(),
        ) {
            return None;
        }
        let ability_def = ctx
            .state_diff
            .initial_state
            .ability_registry
            .get(&self.ability_id)
            .cloned();
        let tag_registry = ctx.state_diff.initial_state.tag_registry.clone();

        if let Some(ability_def) = ability_def {
            let mut card = ModifierCard::Plus0;
            let mut rng = ctx.state_diff.initial_state.rng.clone();
            if let Some(attacker) = ctx.state_diff.get_agent_mut(self.agent) {
                card = attacker.modifier_deck.draw(&mut rng);
                attacker.action_points -= self.ap_cost as i32;
                let mut logger = Logger::default();
                for (tag, n) in &ability_def.emit_tags {
                    attacker.turn_tags.emit(
                        *tag,
                        *n,
                        &TagRegistry {
                            defs: tag_registry.defs.clone(),
                        },
                        &mut logger,
                    );
                }
            }
            ctx.state_diff.diff.rng_update = Some(rng);
            if matches!(ability_def.delivery, AbilityDelivery::SelfTarget) {
                if ability_def.name == "Arcane Shield" {
                    if let Some(attacker) = ctx.state_diff.get_agent_mut(self.agent) {
                        let _ = attacker.add_timed_modifier(TimedModifier {
                            stat: DerivedStat::ArmorClass,
                            amount: 4,
                            remaining_turns: 2,
                            stacking: ModifierStacking::RefreshReplace,
                        });
                    }
                }
            } else {
                let attacker_unit = ctx.state_diff.get_agent(self.agent).unwrap();
                let defender_unit = ctx.state_diff.get_agent(self.target).unwrap();
                let spell_echo = attacker_unit.passive_abilities.iter().any(|p| p.0 == 201);
                let death_embrace = attacker_unit.passive_abilities.iter().any(|p| p.0 == 301);
                let mut logger = Logger::default();
                let mut health_change = -calculate_damage(
                    attacker_unit,
                    defender_unit,
                    &ability_def,
                    card,
                    "CON",
                    &mut logger,
                );
                if spell_echo {
                    health_change *= 2;
                }

                let mut reaction = None;
                if let Some(defender) = ctx.state_diff.get_agent_mut(self.target) {
                    defender.health += health_change;
                    if health_change < 0 && !defender.reaction_abilities.is_empty() {
                        reaction = Some((self.target, defender.reaction_abilities[0], self.agent));
                    }
                    if defender.health <= 0 && death_embrace {
                        if let Some(attacker) = ctx.state_diff.get_agent_mut(self.agent) {
                            attacker.health =
                                (attacker.health + 20).min(attacker.derived_stats.health_max);
                        }
                    }
                }
                if let Some(reaction) = reaction {
                    ctx.state_diff.diff.reaction_queue.push(reaction);
                }
            }
        }
        None
    }

    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        ctx.state_diff.get_agent(self.agent).is_some_and(|unit| {
            unit.action_points >= self.ap_cost as i32
                && crate::tasks::ability_target_is_legal_with_world(
                    &merged_state(ctx),
                    self.agent,
                    self.target,
                    self.ability_id,
                    self.collision_world.as_deref(),
                )
        })
    }

    fn display_action(&self) -> TacticalDisplayAction {
        TacticalDisplayAction::Ability {
            target: self.target,
            ability: self.ability_id,
        }
    }

    impl_task_boxed_methods!(TacticalDomain);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::ability_target_is_legal;
    use crate::{GridCell, SkirmishConfig};
    use hexx::Hex;

    #[test]
    fn projectile_candidates_respect_intervening_units() {
        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Caveman", GridCell::new(Hex::new(0, 0), 0))
            .unwrap();
        scenario
            .add_unit(2, 1, "Caveman", GridCell::new(Hex::new(1, 0), 0))
            .unwrap();
        scenario
            .add_unit(3, 2, "Mage", GridCell::new(Hex::new(2, 0), 0))
            .unwrap();
        let mut state = scenario.build_state().unwrap();
        let rock_throw = state
            .ability_registry
            .values()
            .find(|ability| ability.name == "Rock Throw")
            .map(|ability| ability.id)
            .unwrap();
        assert!(!ability_target_is_legal(
            &state,
            AgentId(1),
            AgentId(3),
            rock_throw
        ));
        state.agents.remove(&AgentId(2));
        assert!(ability_target_is_legal(
            &state,
            AgentId(1),
            AgentId(3),
            rock_throw
        ));
    }

    #[test]
    fn melee_candidates_respect_hex_range() {
        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Caveman", GridCell::new(Hex::ZERO, 0))
            .unwrap();
        scenario
            .add_unit(2, 2, "Mage", GridCell::new(Hex::new(2, 0), 0))
            .unwrap();
        let state = scenario.build_state().unwrap();
        let club_smash = state
            .ability_registry
            .values()
            .find(|ability| ability.name == "Club Smash")
            .map(|ability| ability.id)
            .unwrap();
        assert!(!ability_target_is_legal(
            &state,
            AgentId(1),
            AgentId(2),
            club_smash
        ));
    }
}
