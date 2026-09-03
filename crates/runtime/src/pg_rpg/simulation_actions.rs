impl TacticalSimulation {
    pub fn turn_limit_reached(&self) -> bool {
        self.maximum_turn_count != 0 && self.completed_rounds >= self.maximum_turn_count
    }

    pub fn living_team_count(&self) -> usize {
        self.state
            .agents
            .iter()
            .filter(|(id, unit)| unit.health > 0 && !self.state.summons.contains_key(id))
            .map(|(_, unit)| unit.team_id)
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
            .iter()
            .filter(|(id, unit)| unit.health > 0 && !self.state.summons.contains_key(id))
            .map(|(_, unit)| unit.team_id)
            .collect();
        (teams.len() <= 1)
            .then(|| teams.into_iter().next())
            .flatten()
    }

    pub fn get_prompts(&self, agent_id: i64) -> HashMap<String, bool> {
        let mut prompts = HashMap::new();
        if let Some(_unit) = self.state.agents.get(&AgentId(agent_id as u32)) {
            // In a real game, this would depend on the unit's available actions
            // For the pg_rpg, we'll just show some buttons for the active unit
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
                                    let cost = ability.resolve_cost(&unit.turn_tags);
                                    AvailableAbility {
                                        id: ability.id.0,
                                        name: ability.name.clone(),
                                        base_ap_cost: cost.base_ap,
                                        ap_cost: cost.ap,
                                        health_cost: cost.health,
                                        mana_cost: cost.mana,
                                        health_floor: cost.health_floor,
                                        affordable: cost.can_pay(unit),
                                        legal_target_count: u16::try_from(
                                            pystral_games::tasks::legal_ability_targets(
                                                &self.state, agent, *id,
                                            ).len(),
                                        ).unwrap_or(u16::MAX),
                                        ap_discount: cost.base_ap.saturating_sub(cost.ap),
                                        discount_label: cost.consumed_tags.first().and_then(
                                            |(tag, _)| self.state.tag_names.get(tag).cloned(),
                                        ),
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
        if let Some(facing) = pystral_games::Facing::from_step(unit.position.hex, validated.destination.hex) {
            unit.facing = facing;
        }
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
        if unit.health <= 0 {
            return Err(ActionError::DeadAgent(agent));
        }
        unit.action_points = unit.derived_stats.action_points_max;
        unit.turn_tags.counts.clear();
        unit.ct = 0;
        self.record_completed_turn(agent);
        Ok(())
    }

    pub fn commit_facing(&mut self, agent_id: u64, facing: &str) -> Result<(), ActionError> {
        let agent = AgentId(
            u32::try_from(agent_id).map_err(|_| ActionError::UnknownAgent(AgentId(u32::MAX)))?,
        );
        let facing = pystral_games::Facing::from_name(facing)
            .ok_or_else(|| ActionError::InvalidFacing(facing.to_string()))?;
        let unit = self
            .state
            .agents
            .get_mut(&agent)
            .ok_or(ActionError::UnknownAgent(agent))?;
        if unit.health <= 0 {
            return Err(ActionError::DeadAgent(agent));
        }
        unit.facing = facing;
        Ok(())
    }
}
