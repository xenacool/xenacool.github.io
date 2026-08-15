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

    pub fn list_agents(&self) -> Vec<i64> {
        self.state.agents.keys().map(|id| id.0 as i64).collect()
    }
}
