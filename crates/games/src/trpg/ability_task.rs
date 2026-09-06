use crate::tasks::merged_state;
use crate::{
    AbilityId, AbilityTargetRule, CollisionWorld, Logger, ModifierCard, RPGBytecode, RPGHook,
    ResolvedAbilityCost, TacticalAccess, TacticalAccessMut, TacticalDisplayAction, TacticalDomain,
    TagRegistry, calculate_damage,
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
    pub cost: ResolvedAbilityCost,
    pub collision_world: Option<Arc<CollisionWorld>>,
}

impl std::fmt::Debug for AbilityTask {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AbilityTask")
            .field("agent", &self.agent)
            .field("target", &self.target)
            .field("ability_id", &self.ability_id)
            .field("cost", &self.cost)
            .finish()
    }
}

impl std::hash::Hash for AbilityTask {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.agent.hash(state);
        self.target.hash(state);
        self.ability_id.hash(state);
        self.cost.hash(state);
    }
}

impl PartialEq for AbilityTask {
    fn eq(&self, other: &Self) -> bool {
        self.agent == other.agent
            && self.target == other.target
            && self.ability_id == other.ability_id
            && self.cost == other.cost
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
            let Some(current_attacker) = state.agents.get(&self.agent) else {
                return None;
            };
            let live_cost = ability_def.resolve_cost(&current_attacker.turn_tags);
            if live_cost != self.cost
                || !live_cost.spends_resource()
                || !live_cost.can_pay(current_attacker)
            {
                return None;
            }
            let mut rng = ctx.state_diff.initial_state.rng.clone();
            let damage_ability = matches!(ability_def.target_rule, AbilityTargetRule::EnemyUnit);
            let mut card = ModifierCard::Plus0;
            if let Some(attacker) = ctx.state_diff.get_agent_mut(self.agent) {
                if damage_ability {
                    card = attacker.modifier_deck.draw(&mut rng);
                }
                if live_cost.apply_to(attacker).is_err() {
                    return None;
                }
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
            if damage_ability {
                ctx.state_diff.diff.rng_update = Some(rng);
                let attacker_unit = ctx.state_diff.get_agent(self.agent).unwrap();
                let defender_unit = ctx.state_diff.get_agent(self.target).unwrap();
                let passive_registry = &ctx.state_diff.initial_state.passive_registry;
                let on_kill_heal = attacker_unit
                    .passive_abilities
                    .iter()
                    .filter_map(|id| passive_registry.get(id))
                    .map(|passive| passive.on_kill_heal)
                    .sum::<i32>();
                let mut logger = Logger::default();
                let health_change = -calculate_damage(
                    attacker_unit,
                    defender_unit,
                    &ability_def,
                    passive_registry,
                    card,
                    &mut logger,
                );
                let mut reaction = None;
                let mut actual_damage = 0;
                if let Some(defender) = ctx.state_diff.get_agent_mut(self.target) {
                    let health_before = defender.health;
                    if health_change < 0 {
                        defender.apply_damage(-health_change);
                    } else {
                        defender.apply_healing(health_change);
                    }
                    if health_change < 0 && !defender.reaction_abilities.is_empty() {
                        reaction = Some((self.target, defender.reaction_abilities[0], self.agent));
                    }
                    actual_damage = (health_before - defender.health).max(0);
                    if defender.health <= 0 && on_kill_heal > 0 {
                        if let Some(attacker) = ctx.state_diff.get_agent_mut(self.agent) {
                            attacker.apply_healing(on_kill_heal);
                        }
                    }
                }
                if let Some(reaction) = reaction {
                    ctx.state_diff.diff.reaction_queue.push(reaction);
                }
                execute_effect_program(
                    &ability_def,
                    &state,
                    &mut ctx,
                    self.agent,
                    self.target,
                    actual_damage,
                );
            } else {
                execute_effect_program(&ability_def, &state, &mut ctx, self.agent, self.target, 0);
            }
        }
        None
    }

    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        let state = merged_state(ctx);
        ctx.state_diff.get_agent(self.agent).is_some_and(|unit| {
            state
                .ability_registry
                .get(&self.ability_id)
                .is_some_and(|ability| {
                    let cost = ability.resolve_cost(&unit.turn_tags);
                    cost == self.cost && cost.spends_resource() && cost.can_pay(unit)
                })
                && crate::tasks::ability_target_is_legal_with_world(
                    &state,
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

fn execute_effect_program(
    ability: &crate::AbilityDef,
    state: &crate::TacticalState,
    ctx: &mut ContextMut<TacticalDomain>,
    agent: AgentId,
    target: AgentId,
    actual_damage: i32,
) {
    let Some(program) = ability
        .programs
        .get(&RPGHook::OnAbilityResolve)
        .and_then(|programs| programs.first())
    else {
        return;
    };
    for op in &program.ops {
        match op {
            RPGBytecode::AddTimedModifier {
                stat,
                amount,
                duration_turns,
                stacking,
            } => {
                if let Some(attacker) = ctx.state_diff.get_agent_mut(agent) {
                    attacker
                        .add_timed_modifier(crate::TimedModifier {
                            stat: *stat,
                            amount: *amount,
                            remaining_turns: *duration_turns,
                            stacking: *stacking,
                        })
                        .expect("validated timed modifier effect");
                }
            }
            RPGBytecode::HealCaster { amount } => {
                if let Some(attacker) = ctx.state_diff.get_agent_mut(agent) {
                    attacker.apply_healing(*amount);
                }
            }
            RPGBytecode::ModifyCasterMana { amount } => {
                if let Some(attacker) = ctx.state_diff.get_agent_mut(agent) {
                    attacker.mana = attacker
                        .mana
                        .saturating_add(*amount)
                        .clamp(0, attacker.derived_stats.mana_max);
                }
            }
            RPGBytecode::HealCasterFromActualDamage {
                numerator,
                denominator,
            } => {
                let healing =
                    actual_damage.saturating_mul(i32::from(*numerator)) / i32::from(*denominator);
                if let Some(attacker) = ctx.state_diff.get_agent_mut(agent) {
                    attacker.apply_healing(healing);
                }
            }
            RPGBytecode::DestroyOwnedSummon => {
                if state
                    .summons
                    .get(&target)
                    .is_some_and(|summon| summon.summoner == agent)
                {
                    if let Some(summon) = ctx.state_diff.get_agent_mut(target) {
                        summon.apply_damage(summon.health);
                    }
                }
            }
            RPGBytecode::SpawnOwnedUnit { .. } => {
                // Cell-targeted spawn programs execute at the simulation
                // boundary, where the selected cell and allocator are owned.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::ability_target_is_legal;
    use crate::{
        Facing, FacingRelation, GridCell, Logger, ModifierCard, SkirmishConfig, calculate_damage,
    };
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
        state.agents.get_mut(&AgentId(2)).unwrap().health = 0;
        assert!(ability_target_is_legal(
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

    #[test]
    fn authored_facing_rule_and_damage_multiplier_are_authoritative() {
        let mut scenario = SkirmishConfig::new(42);
        scenario
            .add_unit(1, 1, "Caveman", GridCell::new(Hex::ZERO, 0))
            .unwrap();
        scenario
            .add_unit(2, 2, "Mage", GridCell::new(Hex::new(0, 1), 0))
            .unwrap();
        let mut state = scenario.build_state().unwrap();
        let ability_id = state
            .ability_registry
            .values()
            .find(|ability| ability.name == "Club Smash")
            .map(|ability| ability.id)
            .unwrap();
        {
            let ability = state.ability_registry.get_mut(&ability_id).unwrap();
            ability.facing_rule = Some(FacingRelation::Front);
            ability.rear_damage_multiplier = (2, 1);
        }
        assert!(ability_target_is_legal(
            &state,
            AgentId(1),
            AgentId(2),
            ability_id
        ));
        state.agents.get_mut(&AgentId(1)).unwrap().facing = Facing::North;
        assert!(!ability_target_is_legal(
            &state,
            AgentId(1),
            AgentId(2),
            ability_id
        ));
        state.agents.get_mut(&AgentId(1)).unwrap().facing = Facing::South;
        let attacker = state.agents[&AgentId(1)].clone();
        let defender = state.agents[&AgentId(2)].clone();
        let ability = state.ability_registry[&ability_id].clone();
        let base = calculate_damage(
            &attacker,
            &defender,
            &ability,
            &state.passive_registry,
            ModifierCard::Plus0,
            &mut Logger::default(),
        );
        state.agents.get_mut(&AgentId(2)).unwrap().position.hex = Hex::new(0, -1);
        let rear = calculate_damage(
            &attacker,
            &state.agents[&AgentId(2)],
            &ability,
            &state.passive_registry,
            ModifierCard::Plus0,
            &mut Logger::default(),
        );
        assert_eq!(rear, base * 2);
    }
}
