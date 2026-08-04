impl TacticalSimulation {
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

    pub fn get_agent_kind(&self, agent_id: i64) -> String {
        self.state
            .agents
            .get(&AgentId(agent_id as u32))
            .and_then(|unit| self.state.job_registry.get(&unit.primary_job))
            .map(|job| job.name.clone())
            .unwrap_or_default()
    }

    pub fn set_agent_health(&mut self, agent_id: i64, health: i64) -> Result<(), String> {
        let agent =
            AgentId(u32::try_from(agent_id).map_err(|_| format!("Invalid agent id {agent_id}"))?);
        let unit = self
            .state
            .agents
            .get_mut(&agent)
            .ok_or_else(|| format!("Unknown agent {}", agent.0))?;
        unit.health = (health as i32).clamp(0, unit.derived_stats.health_max);
        Ok(())
    }

    pub fn set_agent_position(
        &mut self,
        agent_id: i64,
        position: GridCell,
    ) -> Result<(), String> {
        let agent =
            AgentId(u32::try_from(agent_id).map_err(|_| format!("Invalid agent id {agent_id}"))?);
        if !self.state.grid.contains(position) {
            return Err(format!("Agent position is outside the tactical grid: {position:?}"));
        }
        if self.state.agents.iter().any(|(id, unit)| {
            *id != agent && unit.health > 0 && unit.position == position
        }) {
            return Err(format!("Agent position is occupied: {position:?}"));
        }
        self.state
            .agents
            .get_mut(&agent)
            .ok_or_else(|| format!("Unknown agent {}", agent.0))?
            .position = position;
        Ok(())
    }

    pub fn set_agent_controller(
        &mut self,
        agent_id: i64,
        controller: UnitController,
    ) -> Result<(), String> {
        let agent =
            AgentId(u32::try_from(agent_id).map_err(|_| format!("Invalid agent id {agent_id}"))?);
        if !self.state.agents.contains_key(&agent) {
            return Err(format!("Unknown agent {}", agent.0));
        }
        self.state.controllers.insert(agent, controller);
        Ok(())
    }

    pub fn set_agent_summon_controller(
        &mut self,
        agent_id: i64,
        controller: UnitController,
    ) -> Result<(), String> {
        let agent =
            AgentId(u32::try_from(agent_id).map_err(|_| format!("Invalid agent id {agent_id}"))?);
        if !self.state.agents.contains_key(&agent) {
            return Err(format!("Unknown agent {}", agent.0));
        }
        if controller == UnitController::Player {
            return Err("Summons cannot use a player controller".to_string());
        }
        self.state.summon_controllers.insert(agent, controller);
        Ok(())
    }

    pub fn list_agents(&self) -> Vec<i64> {
        let mut agents = self
            .state
            .agents
            .keys()
            .map(|id| id.0 as i64)
            .collect::<Vec<_>>();
        agents.sort_unstable();
        agents
    }
}
