use crate::{
    AbilityDelivery, AbilityId, CollisionFilter, CollisionWorld, GridCell, Logger, QueryShape,
    ReactionId, TacticalAccess, TacticalAccessMut, TacticalDisplayAction, TacticalDomain,
    TagRegistry, TrajectoryQuery, ability_task::AbilityTask, reachable_cells, validate_move,
};
pub use npc_engine_core::{
    AgentId, Behavior, Context, ContextMut, Task, TaskDuration, impl_task_boxed_methods,
};

pub(crate) fn merged_state(ctx: Context<TacticalDomain>) -> crate::TacticalState {
    let mut state = ctx.state_diff.initial_state.clone();
    for (&id, changed) in &ctx.state_diff.diff.agents {
        state.agents.insert(id, changed.clone());
    }
    state
}

pub fn ability_target_is_legal(
    state: &crate::TacticalState,
    agent: AgentId,
    target: AgentId,
    ability_id: AbilityId,
) -> bool {
    legal_ability_targets(state, agent, ability_id).contains(&target)
}

/// Enumerate the canonical unit targets for an ability from one immutable
/// tactical snapshot. Menus and task generation should use this result so
/// projectile collision is evaluated against the same world and ordering.
pub fn legal_ability_targets(
    state: &crate::TacticalState,
    agent: AgentId,
    ability_id: AbilityId,
) -> Vec<AgentId> {
    let Some(attacker) = state.agents.get(&agent) else {
        return Vec::new();
    };
    let Some(ability) = state.ability_registry.get(&ability_id) else {
        return Vec::new();
    };
    let collision_world = matches!(
        ability.delivery,
        AbilityDelivery::StraightProjectile | AbilityDelivery::ArcProjectile
    )
    .then(|| state.collision.as_ref())
    .flatten()
    .and_then(|collision_map| collision_map.build_world(&state.grid, &state.agents).ok());
    let mut targets = state
        .agents
        .iter()
        .filter(|(target, unit)| {
            (**target == agent && matches!(ability.delivery, AbilityDelivery::SelfTarget))
                || (**target != agent && unit.health > 0 && unit.team_id != attacker.team_id)
        })
        .filter_map(|(target, _)| {
            ability_target_is_legal_with_world(
                state,
                agent,
                *target,
                ability_id,
                collision_world.as_ref(),
            )
            .then_some(*target)
        })
        .collect::<Vec<_>>();
    targets.sort();
    targets
}

pub(crate) fn ability_target_is_legal_with_world(
    state: &crate::TacticalState,
    agent: AgentId,
    target: AgentId,
    ability_id: AbilityId,
    collision_world: Option<&CollisionWorld>,
) -> bool {
    let Some(attacker) = state.agents.get(&agent) else {
        return false;
    };
    let Some(defender) = state.agents.get(&target) else {
        return false;
    };
    let Some(ability) = state.ability_registry.get(&ability_id) else {
        return false;
    };

    if matches!(ability.delivery, AbilityDelivery::SelfTarget) {
        return target == agent;
    }
    if target == agent || defender.team_id == attacker.team_id {
        return false;
    }

    let hex_distance = attacker.position.hex.distance_to(defender.position.hex) as u8;
    let layer_distance = attacker.position.layer.abs_diff(defender.position.layer) as u8;
    if hex_distance > ability.range || layer_distance > ability.range {
        return false;
    }

    match ability.delivery {
        AbilityDelivery::Melee => layer_distance == 0 && hex_distance <= ability.range,
        AbilityDelivery::Area => true,
        AbilityDelivery::StraightProjectile => {
            let Some(collision_map) = state.collision.as_ref() else {
                return false;
            };
            let Some(world) = collision_world else {
                return false;
            };
            let geometry = collision_map.geometry;
            let start = CollisionWorld::unit_world_position(attacker.position, geometry);
            let end = CollisionWorld::unit_world_position(defender.position, geometry);
            let shape = QueryShape::Ball {
                radius: geometry.hex_width() / 20.0,
            };
            let Some(hit) = world.cast_trajectory(TrajectoryQuery {
                start,
                end,
                shape,
                filter: CollisionFilter::default(),
                exclude_agent: Some(agent),
            }) else {
                return false;
            };
            matches!(hit.kind, crate::ColliderKind::Unit { agent: hit_agent } if hit_agent == target)
        }
        AbilityDelivery::ArcProjectile => {
            let Some(collision_map) = state.collision.as_ref() else {
                return false;
            };
            let Some(world) = collision_world else {
                return false;
            };
            let geometry = collision_map.geometry;
            let mut request = collision_map.trajectory.clone();
            request.start = CollisionWorld::unit_world_position(attacker.position, geometry);
            request.target = CollisionWorld::unit_world_position(defender.position, geometry);
            world.arc_reaches_target(&request, agent, target).is_ok()
        }
        AbilityDelivery::SelfTarget => unreachable!(),
    }
}

fn reaction_queue(ctx: Context<TacticalDomain>) -> Vec<(AgentId, ReactionId, AgentId)> {
    if let Some(queue) = &ctx.state_diff.diff.reaction_queue_replace {
        return queue.clone();
    }
    ctx.state_diff
        .initial_state
        .reaction_queue
        .iter()
        .chain(ctx.state_diff.diff.reaction_queue.iter())
        .cloned()
        .collect()
}

fn has_pending_reaction(ctx: Context<TacticalDomain>, agent: AgentId) -> bool {
    reaction_queue(ctx)
        .iter()
        .any(|(reaction_agent, _, _)| *reaction_agent == agent)
}

pub struct ReactionBehavior;
impl Behavior<TacticalDomain> for ReactionBehavior {
    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        !ctx.state_diff.initial_state.reaction_queue.is_empty()
            || !ctx.state_diff.diff.reaction_queue.is_empty()
    }

    fn add_own_tasks(
        &self,
        ctx: Context<TacticalDomain>,
        tasks: &mut Vec<Box<dyn Task<TacticalDomain>>>,
    ) {
        let queue = reaction_queue(ctx);
        let reaction = queue.iter().find(|(agent_id, _, _)| *agent_id == ctx.agent);

        if let Some((_, reaction_id, target_id)) = reaction {
            tasks.push(Box::new(ReactionTask {
                agent: ctx.agent,
                reaction_id: *reaction_id,
                target: *target_id,
            }));
        }
    }
}

pub struct MoveBehavior;

fn nearest_enemy_distance(state: &crate::TacticalState, agent: AgentId, cell: GridCell) -> i32 {
    let Some(team_id) = state.agents.get(&agent).map(|unit| unit.team_id) else {
        return i32::MAX;
    };
    state
        .agents
        .values()
        .filter(|unit| unit.health > 0 && unit.team_id != team_id)
        .map(|unit| cell.hex.distance_to(unit.position.hex))
        .min()
        .unwrap_or(i32::MAX)
}

impl Behavior<TacticalDomain> for MoveBehavior {
    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        if has_pending_reaction(ctx, ctx.agent) {
            return false;
        }
        if let Some(unit) = ctx.state_diff.get_agent(ctx.agent) {
            unit.action_points > 0
        } else {
            false
        }
    }

    fn add_own_tasks(
        &self,
        ctx: Context<TacticalDomain>,
        tasks: &mut Vec<Box<dyn Task<TacticalDomain>>>,
    ) {
        let state = merged_state(ctx);
        if let Ok(destinations) = reachable_cells(&state, ctx.agent) {
            // Keep the MCTS branching factor bounded: ordinary movement tasks
            // advance one minimum-cost frontier step, while teleport programs
            // expose their full same-cost range. The complete reachable field
            // remains available through `reachable_cells` for UI previews.
            let minimum_cost = destinations.values().copied().min();
            let mut candidates = destinations
                .into_iter()
                .filter(|(_, ap_cost)| minimum_cost == Some(*ap_cost))
                .collect::<Vec<_>>();
            candidates.sort_by_key(|(cell, _)| {
                (
                    nearest_enemy_distance(&state, ctx.agent, *cell),
                    cell.layer,
                    cell.hex.x,
                    cell.hex.y,
                )
            });
            // Keep both aggressive and retreating positions in the root set.
            // Evenly sampling the sorted list avoids spending all root tasks
            // on nearly identical neighboring cells.
            let limit = candidates.len().min(6);
            let selected = if candidates.len() <= limit {
                candidates
            } else {
                (0..limit)
                    .map(|index| candidates[index * (candidates.len() - 1) / (limit - 1)])
                    .collect()
            };
            for (target_pos, ap_cost) in selected {
                tasks.push(Box::new(MoveTask {
                    agent: ctx.agent,
                    to: target_pos,
                    ap_cost,
                }));
            }
        }
    }
}

pub struct AbilityBehavior;
impl Behavior<TacticalDomain> for AbilityBehavior {
    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        if has_pending_reaction(ctx, ctx.agent) {
            return false;
        }
        if let Some(unit) = ctx.state_diff.get_agent(ctx.agent) {
            unit.action_points > 0
        } else {
            false
        }
    }

    fn add_own_tasks(
        &self,
        ctx: Context<TacticalDomain>,
        tasks: &mut Vec<Box<dyn Task<TacticalDomain>>>,
    ) {
        let state = merged_state(ctx);
        let unit = state.agents.get(&ctx.agent).unwrap();
        let abilities = unit
            .available_action_abilities(&state.job_registry)
            .expect("validated unit jobs have definitions");
        let needs_collision_world = abilities.iter().any(|ability_id| {
            matches!(
                state.ability_registry[ability_id].delivery,
                AbilityDelivery::StraightProjectile | AbilityDelivery::ArcProjectile
            )
        });
        let collision_world = needs_collision_world
            .then(|| state.collision.as_ref())
            .flatten()
            .and_then(|collision_map| collision_map.build_world(&state.grid, &state.agents).ok())
            .map(std::sync::Arc::new);
        for ability_id in abilities {
            let ability_def = &state.ability_registry[&ability_id];
            let mut tag_bag = unit.turn_tags.clone();
            let cost = ability_def.get_ap_cost(&mut tag_bag);

            if unit.action_points >= cost as i32 {
                for target_id in legal_ability_targets(&state, ctx.agent, ability_id) {
                    tasks.push(Box::new(AbilityTask {
                        agent: ctx.agent,
                        target: target_id,
                        ability_id,
                        ap_cost: cost,
                        collision_world: collision_world.clone(),
                    }));
                }
            }
        }
    }
}

pub struct WaitBehavior;
impl Behavior<TacticalDomain> for WaitBehavior {
    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        !has_pending_reaction(ctx, ctx.agent)
    }

    fn add_own_tasks(
        &self,
        ctx: Context<TacticalDomain>,
        tasks: &mut Vec<Box<dyn Task<TacticalDomain>>>,
    ) {
        tasks.push(Box::new(WaitTask { agent: ctx.agent }));
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ReactionTask {
    pub agent: AgentId,
    pub reaction_id: ReactionId,
    pub target: AgentId,
}

impl Task<TacticalDomain> for ReactionTask {
    fn duration(&self, _ctx: Context<TacticalDomain>) -> TaskDuration {
        0
    }

    fn execute(
        &self,
        mut ctx: ContextMut<TacticalDomain>,
    ) -> Option<Box<dyn Task<TacticalDomain>>> {
        let reaction_def = ctx
            .state_diff
            .initial_state
            .reaction_registry
            .get(&self.reaction_id)
            .cloned();
        let mut queue = ctx.state_diff.initial_state.reaction_queue.clone();
        queue.extend(ctx.state_diff.diff.reaction_queue.iter().cloned());

        if let Some(pos) = queue
            .iter()
            .position(|r| *r == (self.agent, self.reaction_id, self.target))
        {
            queue.remove(pos);
        }
        ctx.state_diff.diff.reaction_queue_replace = Some(queue);

        // Reaction logic
        if let Some(unit) = ctx.state_diff.get_agent_mut(self.agent) {
            if let Some(def) = reaction_def {
                unit.action_points -= def.ap_cost as i32;

                let attacker_id = self.target;
                match self.reaction_id.0 {
                    101 => {
                        // Counter-Swing (Caveman)
                        if let Some(target_unit) = ctx.state_diff.get_agent_mut(attacker_id) {
                            target_unit.health -= 10;
                        }
                    }
                    201 => {
                        // Mana Shield (Mage)
                        unit.health += 5;
                        unit.mana -= 10;
                    }
                    301 => {
                        // Vengeful Spirit (Necromancer)
                        if let Some(target_unit) = ctx.state_diff.get_agent_mut(attacker_id) {
                            target_unit.health -= 15;
                        }
                    }
                    401 => {
                        // Bone Splinter (Skeleton)
                        if let Some(target_unit) = ctx.state_diff.get_agent_mut(attacker_id) {
                            target_unit.health -= 5;
                        }
                    }
                    _ => {}
                }
            }
        }

        None
    }

    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        reaction_queue(ctx)
            .iter()
            .any(|r| *r == (self.agent, self.reaction_id, self.target))
    }

    fn display_action(&self) -> TacticalDisplayAction {
        TacticalDisplayAction::Reaction {
            reaction: self.reaction_id,
            target: self.target,
        }
    }

    impl_task_boxed_methods!(TacticalDomain);
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MoveTask {
    pub agent: AgentId,
    pub to: GridCell,
    pub ap_cost: u8,
}

impl Task<TacticalDomain> for MoveTask {
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
        if validate_move(&state, self.agent, self.to)
            .map(|validated| validated.ap_cost == self.ap_cost)
            != Ok(true)
        {
            return None;
        }
        let tag_registry = ctx.state_diff.initial_state.tag_registry.clone();
        let movement_registry = ctx.state_diff.initial_state.movement_registry.clone();

        if let Some(unit) = ctx.state_diff.get_agent_mut(self.agent) {
            unit.position = self.to;
            unit.action_points -= self.ap_cost as i32;

            let move_ability_id = unit.movement_ability;
            let prog = movement_registry.get(&move_ability_id).cloned();
            if let Some(prog) = prog {
                for (tag, n) in prog.emit_tags {
                    let mut dummy_logger = Logger::default();
                    unit.turn_tags.emit(
                        tag,
                        n,
                        &TagRegistry {
                            defs: tag_registry.defs.clone(),
                        },
                        &mut dummy_logger,
                    );

                    // Manafeet logic: if tag 10 is emitted, grant MP
                    if tag.0 == 10 {
                        unit.mana = (unit.mana + 5).min(unit.derived_stats.mana_max);
                    }
                }
            }
        }
        None
    }

    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        let Some(unit) = ctx.state_diff.get_agent(self.agent) else {
            return false;
        };
        if unit.action_points < self.ap_cost as i32 {
            return false;
        }

        let mut state = ctx.state_diff.initial_state.clone();
        for (&id, changed) in &ctx.state_diff.diff.agents {
            state.agents.insert(id, changed.clone());
        }
        validate_move(&state, self.agent, self.to)
            .is_ok_and(|validated| validated.ap_cost == self.ap_cost)
    }

    fn display_action(&self) -> TacticalDisplayAction {
        TacticalDisplayAction::Move { to: self.to }
    }

    impl_task_boxed_methods!(TacticalDomain);
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct WaitTask {
    pub agent: AgentId,
}

impl Task<TacticalDomain> for WaitTask {
    fn duration(&self, _ctx: Context<TacticalDomain>) -> TaskDuration {
        10 // Duration until next CT fire
    }

    fn execute(
        &self,
        mut ctx: ContextMut<TacticalDomain>,
    ) -> Option<Box<dyn Task<TacticalDomain>>> {
        if let Some(unit) = ctx.state_diff.get_agent_mut(self.agent) {
            unit.advance_owner_turn();
            unit.action_points = unit.derived_stats.action_points_max;
            unit.turn_tags.counts.clear();
            unit.ct = 0;
            ctx.state_diff.diff.turn_completed = true;
        }
        None
    }

    fn is_valid(&self, _ctx: Context<TacticalDomain>) -> bool {
        true
    }

    fn display_action(&self) -> TacticalDisplayAction {
        TacticalDisplayAction::Wait
    }

    impl_task_boxed_methods!(TacticalDomain);
}
