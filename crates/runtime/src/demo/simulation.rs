use npc_engine_core::{AgentId, Context, ContextMut, MCTS, MCTSConfiguration, StateDiffRefMut};
use npc_engine_utils::GlobalDomain;
use pystral_core::log::{AvailableAbility, AvailableActions, AvailableJobActions, AvailableMove};
use pystral_games::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NpcPlanningPolicy {
    pub minimum_hit_probability: f32,
    pub allow_desperation: bool,
}

impl Default for NpcPlanningPolicy {
    fn default() -> Self {
        Self {
            minimum_hit_probability: 0.20,
            allow_desperation: false,
        }
    }
}

#[derive(Clone)]
pub struct TacticalSimulation {
    pub state: TacticalState,
    pub scheduler: CTScheduler,
    pub config: MCTSConfiguration,
    pub planning_policy: NpcPlanningPolicy,
    pub maximum_turn_count: u32,
    pub completed_rounds: u32,
    completed_turns: HashSet<AgentId>,
    ready_queue: VecDeque<AgentId>,
}

#[cfg(test)]
#[path = "simulation_tests.rs"]
mod tests;
impl TacticalSimulation {
    fn action_is_plannable(&self, agent: AgentId, action: &TacticalDisplayAction) -> bool {
        match action {
            TacticalDisplayAction::Ability { target, ability } => {
                let probability =
                    ability_success_probability(&self.state, agent, *target, *ability);
                probability >= self.planning_policy.minimum_hit_probability
                    || (self.planning_policy.allow_desperation
                        && ability_can_kill_with_any_modifier(
                            &self.state,
                            agent,
                            *target,
                            *ability,
                        ))
            }
            // Reactions are forced responses and must not be filtered by the
            // ordinary attack policy.
            TacticalDisplayAction::Reaction { .. }
            | TacticalDisplayAction::Move { .. }
            | TacticalDisplayAction::Wait => true,
        }
    }

    pub fn new(config: MCTSConfiguration) -> Self {
        Self::from_scenario(SkirmishConfig::new(42), config)
    }

    pub fn from_scenario(scenario: SkirmishConfig, config: MCTSConfiguration) -> Self {
        let mut state = scenario
            .build_state()
            .expect("validated skirmish configuration");
        let scheduler = CTScheduler::new(scenario.ct_threshold);
        scheduler.initialize_ct(&mut state);

        Self {
            state,
            scheduler,
            config,
            planning_policy: NpcPlanningPolicy::default(),
            maximum_turn_count: scenario.maximum_turn_count,
            completed_rounds: 0,
            completed_turns: HashSet::new(),
            ready_queue: VecDeque::new(),
        }
    }

    /// Advance the scheduler to the next control boundary without choosing an
    /// action. The runtime owns the decision that follows this boundary.
    pub fn advance_to_boundary(&mut self) -> Result<Vec<AgentId>, String> {
        loop {
            if let Some(ready) = self.advance_to_boundary_budgeted(usize::MAX)? {
                return Ok(ready);
            }
        }
    }

    pub fn advance_to_boundary_budgeted(
        &mut self,
        max_ticks: usize,
    ) -> Result<Option<Vec<AgentId>>, String> {
        if self.is_complete() {
            return Ok(Some(Vec::new()));
        }
        while let Some(agent) = self.ready_queue.pop_front() {
            if self
                .state
                .agents
                .get(&agent)
                .is_some_and(|unit| unit.health > 0)
            {
                return Ok(Some(vec![agent]));
            }
        }
        let ready_agents = self
            .scheduler
            .tick_until_ready_budgeted(&mut self.state, max_ticks);
        let Some(ready_agents) = ready_agents else {
            return Ok(None);
        };
        self.ready_queue.extend(ready_agents);
        while let Some(agent) = self.ready_queue.pop_front() {
            if self
                .state
                .agents
                .get(&agent)
                .is_some_and(|unit| unit.health > 0)
            {
                return Ok(Some(vec![agent]));
            }
        }
        Ok(None)
    }

    /// Return only a serializable gameplay action. Engine task objects never
    /// cross the runtime/controller boundary.
    pub fn request_npc_decision(&self, agent: AgentId) -> Option<TacticalDisplayAction> {
        let diff = TacticalDiff::default();
        let context = Context::with_state_and_diff(0, &self.state, &diff, agent);
        let tasks = TacticalDomain::get_tasks(context);
        let mut planning_tasks = tasks
            .iter()
            .filter(|task| self.action_is_plannable(agent, &task.display_action()))
            .collect::<Vec<_>>();
        if planning_tasks.is_empty() {
            planning_tasks = tasks.iter().collect();
        }
        // Multiple engine tasks can represent the same gameplay action (for
        // example, equivalent composite movement/action variants). MCTS only
        // returns a display action and the runtime revalidates that action, so
        // retaining duplicate display actions creates redundant root edges,
        // task clones, and rollouts. Keep the first task to preserve the
        // deterministic ordering supplied by TacticalDomain::get_tasks.
        let mut seen_actions = HashSet::new();
        planning_tasks.retain(|task| seen_actions.insert(task.display_action()));
        let root_tasks = planning_tasks.iter().map(|task| task.box_clone()).collect();
        // TODO: Late-game branching can otherwise monopolize the simulation worker?
        // need to reduce symmetry of search options.
        let mut search_config = self.config.clone();
        let snapshot_seed = self.snapshot_fingerprint() ^ (agent.0 as u64).rotate_left(17);
        search_config.seed = Some(search_config.seed.unwrap_or(0) ^ snapshot_seed);
        if self.completed_turns.len() > 2 {
            search_config.visits = search_config.visits.min(1);
            search_config.depth = search_config.depth.min(1);
        }
        // A one-visit/one-ply late-turn search cannot learn anything beyond
        // the immediate root-action heuristic below, but constructing MCTS
        // still clones the complete state and allocates a search tree. Skip
        // that setup in the explicitly capped mode. The root scorer remains
        // deterministic and the same action validation/fallback path applies.
        let candidate = if search_config.visits <= 1 && search_config.depth <= 1 {
            None
        } else {
            let mut mcts = MCTS::<TacticalDomain>::new_with_root_tasks(
                self.state.clone(),
                agent,
                root_tasks,
                search_config,
            );
            mcts.run()
        };
        let score_after = |task: &Box<dyn npc_engine_core::Task<TacticalDomain>>| {
            let mut task_diff = TacticalDiff::default();
            task.execute(ContextMut {
                tick: 0,
                state_diff: StateDiffRefMut {
                    initial_state: &self.state,
                    diff: &mut task_diff,
                },
                agent,
            });
            // The value function reads through StateDiffRef, which overlays
            // changed agents on the immutable snapshot. Applying the diff to
            // a cloned TacticalState here duplicated that overlay work and
            // cloned the grid, registries, collision map, RNG, and logger for
            // every root task. Score directly against the diff instead.
            TacticalDomain::get_current_value(
                0,
                Context::with_state_and_diff(0, &self.state, &task_diff, agent).state_diff,
                agent,
            )
        };
        let scored_tasks = planning_tasks
            .iter()
            .map(|task| (task.display_action(), score_after(task)))
            .collect::<Vec<_>>();
        let best = scored_tasks
            .iter()
            .max_by(|left, right| left.1.cmp(&right.1))?;
        let baseline = best.1 - 5.0;
        let candidate_action = candidate.as_ref().map(|task| task.display_action());
        let current_root_task = candidate_action.as_ref().and_then(|action| {
            planning_tasks
                .iter()
                .find(|task| task.display_action() == *action && task.is_valid(context))
        });
        let candidate_score = candidate_action.as_ref().and_then(|action| {
            scored_tasks
                .iter()
                .find(|(legal_action, _)| legal_action == action)
                .map(|(_, score)| score)
        });
        let candidate_is_acceptable =
            current_root_task.is_some() && candidate_score.is_some_and(|score| *score >= baseline);
        if candidate_is_acceptable {
            // MCTS may return a task object reached through a deeper search
            // node. Return the equivalent task from the immutable root task
            // set so embedded state (notably projectile collision data) is
            // from the snapshot that will be revalidated and committed.
            current_root_task.map(|task| task.display_action())
        } else {
            Some(best.0.clone())
        }
    }

    pub fn wait_decision(&self, _agent: AgentId) -> TacticalDisplayAction {
        TacticalDisplayAction::Wait
    }

    /// Select a valid emergency action from the current snapshot. Reactions
    /// must win over Wait because a unit with a pending reaction is not
    /// allowed to advance its turn until that reaction is resolved.
    pub fn fallback_npc_action(&self, agent: AgentId) -> Option<TacticalDisplayAction> {
        let diff = TacticalDiff::default();
        TacticalDomain::get_tasks(Context::with_state_and_diff(0, &self.state, &diff, agent))
            .into_iter()
            .find(|task| task.is_valid(Context::with_state_and_diff(0, &self.state, &diff, agent)))
            .map(|task| task.display_action())
    }

    /// Stable fingerprint for the gameplay snapshot used by target queries.
    /// Hash-map iteration order is excluded so menu/commit comparisons remain
    /// deterministic across independently rebuilt snapshots.
    pub fn snapshot_fingerprint(&self) -> u64 {
        let mut agents = self.state.agents.iter().collect::<Vec<_>>();
        agents.sort_by_key(|(id, _)| **id);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for (id, unit) in agents {
            id.hash(&mut hasher);
            unit.hash(&mut hasher);
        }
        self.state.grid.tiles.len().hash(&mut hasher);
        self.state.ability_registry.len().hash(&mut hasher);
        hasher.finish()
    }

    /// Rebuild and revalidate a typed candidate against the current state.
    /// MCTS tasks are intentionally not transported across the runtime
    /// boundary; the action is matched against the current legal task set.
    pub fn apply_npc_action(
        &mut self,
        agent: AgentId,
        action: TacticalDisplayAction,
    ) -> Result<TacticalDisplayAction, String> {
        let mut diff = TacticalDiff::default();
        let tasks =
            TacticalDomain::get_tasks(Context::with_state_and_diff(0, &self.state, &diff, agent));
        let context = Context::with_state_and_diff(0, &self.state, &diff, agent);
        let legal_actions = tasks
            .iter()
            .filter(|task| task.is_valid(context))
            .map(|task| task.display_action())
            .collect::<Vec<_>>();
        let Some(task) = tasks
            .into_iter()
            .find(|task| task.display_action() == action && task.is_valid(context))
        else {
            let actor = self
                .state
                .agents
                .get(&agent)
                .map(|unit| {
                    format!(
                        "position={:?}, layer={}, health={}, ap={}",
                        unit.position.hex, unit.position.layer, unit.health, unit.action_points
                    )
                })
                .unwrap_or_else(|| "missing actor".to_string());
            return Err(format!(
                "NPC candidate {:?} failed revalidation for agent {} (snapshot={}, {}, legal_actions={:?})",
                action,
                agent.0,
                self.snapshot_fingerprint(),
                actor,
                legal_actions
            ));
        };
        let context = ContextMut {
            tick: 0,
            state_diff: StateDiffRefMut {
                initial_state: &self.state,
                diff: &mut diff,
            },
            agent,
        };
        task.execute(context);
        let previous = self.state.clone();
        TacticalDomain::apply(&mut self.state, &previous, &diff);
        // Every committed NPC action ends that unit's turn. Restricting this
        // to Wait left move/ability turns out of the round ledger, so the
        // late-turn MCTS cap never activated during real combat and the
        // worker could monopolize the browser on repeated NPC actions.
        self.record_completed_turn(agent);
        Ok(action)
    }

    /// Resolve a cell-centered area ability atomically. The selected cell is
    /// validated against the live snapshot before spending resources or
    /// applying any affected-unit changes.
    pub fn commit_area_ability(
        &mut self,
        agent: AgentId,
        ability: AbilityId,
        center: GridCell,
    ) -> Result<Vec<AgentId>, String> {
        let attacker = self
            .state
            .agents
            .get(&agent)
            .cloned()
            .ok_or_else(|| format!("Unknown unit {}", agent.0))?;
        let definition = self
            .state
            .ability_registry
            .get(&ability)
            .cloned()
            .ok_or_else(|| format!("Unknown ability {}", ability.0))?;
        if !matches!(definition.delivery, AbilityDelivery::Area) {
            return Err(format!("Ability {} is not cell-targeted", ability.0));
        }
        if !self.state.grid.contains(center)
            || attacker.position.layer.abs_diff(center.layer) > u32::from(definition.range)
            || attacker.position.hex.distance_to(center.hex) > i32::from(definition.range)
        {
            return Err("Area center is outside the ability range".to_string());
        }
        let mut tags = attacker.turn_tags.clone();
        let ap_cost = definition.get_ap_cost(&mut tags);
        if attacker.action_points < i32::from(ap_cost) {
            return Err("Insufficient action points".to_string());
        }
        let affected = self
            .state
            .agents
            .iter()
            .filter(|(id, unit)| {
                **id != agent
                    && unit.health > 0
                    && unit.team_id != attacker.team_id
                    && unit.position.layer == center.layer
                    && unit.position.hex.distance_to(center.hex)
                        <= i32::from(definition.area_radius)
            })
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        if affected.is_empty() {
            return Err("Area center has no legal enemy targets".to_string());
        }
        let mut rng = self.state.rng.clone();
        let modifier = self
            .state
            .agents
            .get_mut(&agent)
            .expect("validated attacker");
        let card = modifier.modifier_deck.draw(&mut rng);
        modifier.action_points -= i32::from(ap_cost);
        modifier.turn_tags = tags;
        self.state.rng = rng;
        let attacker_snapshot = self.state.agents[&agent].clone();
        for target in &affected {
            let defender = self.state.agents[target].clone();
            let mut logger = Logger::default();
            let damage = calculate_damage(
                &attacker_snapshot,
                &defender,
                &definition,
                card,
                "CON",
                &mut logger,
            );
            if let Some(unit) = self.state.agents.get_mut(target) {
                unit.health -= damage;
            }
        }
        Ok(affected)
    }

    pub fn is_complete(&self) -> bool {
        self.living_team_count() <= 1 || self.turn_limit_reached()
    }

    pub fn turn_limit_reached(&self) -> bool {
        self.maximum_turn_count != 0 && self.completed_rounds >= self.maximum_turn_count
    }

    pub fn living_team_count(&self) -> usize {
        self.state
            .agents
            .values()
            .filter(|unit| unit.health > 0)
            .map(|unit| unit.team_id)
            .collect::<HashSet<_>>()
            .len()
    }

    pub fn record_completed_turn(&mut self, agent: AgentId) {
        let Some(unit) = self.state.agents.get(&agent) else {
            return;
        };
        if unit.health <= 0 {
            return;
        }
        self.completed_turns.insert(agent);
        let living_agents = self
            .state
            .agents
            .iter()
            .filter(|(_, unit)| unit.health > 0)
            .map(|(id, _)| *id)
            .collect::<HashSet<_>>();
        if !living_agents.is_empty() && living_agents.is_subset(&self.completed_turns) {
            self.completed_rounds = self.completed_rounds.saturating_add(1);
            self.completed_turns.clear();
        }
    }

    pub fn winning_team(&self) -> Option<u8> {
        let teams: std::collections::HashSet<u8> = self
            .state
            .agents
            .values()
            .filter(|unit| unit.health > 0)
            .map(|unit| unit.team_id)
            .collect();
        (teams.len() <= 1)
            .then(|| teams.into_iter().next())
            .flatten()
    }

    pub fn get_prompts(&self, agent_id: i64) -> HashMap<String, bool> {
        let mut prompts = HashMap::new();
        if let Some(_unit) = self.state.agents.get(&AgentId(agent_id as u32)) {
            // In a real game, this would depend on the unit's available actions
            // For the demo, we'll just show some buttons for the active unit
            prompts.insert("up".to_string(), true);
            prompts.insert("down".to_string(), true);
            prompts.insert("left".to_string(), true);
            prompts.insert("right".to_string(), true);
            prompts.insert("confirm".to_string(), true);
            prompts.insert("return".to_string(), false);
            prompts.insert("wait".to_string(), true);
        }
        prompts
    }

    pub fn get_available_actions(&self, agent_id: i64) -> Option<AvailableActions> {
        let agent = AgentId(u32::try_from(agent_id).ok()?);
        let unit = self.state.agents.get(&agent)?;
        let mut movement = reachable_cells(&self.state, agent)
            .ok()?
            .into_iter()
            .map(|(cell, ap_cost)| AvailableMove {
                hex: cell.hex,
                layer: cell.layer,
                ap_cost,
            })
            .collect::<Vec<_>>();
        movement.sort_by_key(|movement| (movement.layer, movement.hex.x, movement.hex.y));

        let job_actions = |job_id: JobId| {
            let job = self.state.job_registry.get(&job_id);
            AvailableJobActions {
                name: job.map_or_else(
                    || "Unknown".to_string(),
                    |definition| definition.name.clone(),
                ),
                abilities: job
                    .map(|job| {
                        job.abilities
                            .iter()
                            .filter_map(|id| {
                                self.state.ability_registry.get(id).map(|ability| {
                                    AvailableAbility {
                                        id: ability.id.0,
                                        name: ability.name.clone(),
                                    }
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        };

        Some(AvailableActions {
            unit_id: agent.0 as u64,
            movement,
            primary_job: job_actions(unit.primary_job),
            secondary_jobs: unit
                .secondary_jobs
                .iter()
                .copied()
                .map(job_actions)
                .collect(),
        })
    }

    pub fn move_preview(
        &self,
        agent_id: u64,
    ) -> Result<(AvailableMove, Vec<AvailableMove>), ActionError> {
        let agent = AgentId(
            u32::try_from(agent_id).map_err(|_| ActionError::UnknownAgent(AgentId(u32::MAX)))?,
        );
        let unit = self
            .state
            .agents
            .get(&agent)
            .ok_or(ActionError::UnknownAgent(agent))?;
        let mut reachable = reachable_cells(&self.state, agent)
            .map_err(|_| ActionError::UnknownMovement(unit.movement_ability))?
            .into_iter()
            .map(|(cell, ap_cost)| AvailableMove {
                hex: cell.hex,
                layer: cell.layer,
                ap_cost,
            })
            .collect::<Vec<_>>();
        reachable.sort_by_key(|cell| (cell.layer, cell.hex.x, cell.hex.y));
        Ok((
            AvailableMove {
                hex: unit.position.hex,
                layer: unit.position.layer,
                ap_cost: 0,
            },
            reachable,
        ))
    }

    pub fn commit_move(
        &mut self,
        agent_id: u64,
        destination: GridCell,
    ) -> Result<ValidatedMove, ActionError> {
        let validated = validate_move(
            &self.state,
            AgentId(
                u32::try_from(agent_id)
                    .map_err(|_| ActionError::UnknownAgent(AgentId(u32::MAX)))?,
            ),
            destination,
        )?;
        let unit = self
            .state
            .agents
            .get_mut(&validated.agent)
            .expect("validated agent exists");
        unit.position = validated.destination;
        unit.action_points -= i32::from(validated.ap_cost);
        Ok(validated)
    }

    pub fn commit_wait(&mut self, agent_id: u64) -> Result<(), ActionError> {
        let agent = AgentId(
            u32::try_from(agent_id).map_err(|_| ActionError::UnknownAgent(AgentId(u32::MAX)))?,
        );
        let unit = self
            .state
            .agents
            .get_mut(&agent)
            .ok_or(ActionError::UnknownAgent(agent))?;
        unit.action_points = unit.derived_stats.action_points_max;
        unit.turn_tags.counts.clear();
        unit.ct = 0;
        self.record_completed_turn(agent);
        Ok(())
    }

    pub fn get_agent_position(&self, agent_id: i64) -> GridCell {
        self.state
            .agents
            .get(&AgentId(agent_id as u32))
            .map(|u| u.position)
            .unwrap_or_default()
    }

    pub fn get_agent_health(&self, agent_id: i64) -> i32 {
        self.state
            .agents
            .get(&AgentId(agent_id as u32))
            .map(|u| u.health)
            .unwrap_or(0)
    }

    pub fn list_agents(&self) -> Vec<i64> {
        self.state.agents.keys().map(|id| id.0 as i64).collect()
    }
}
