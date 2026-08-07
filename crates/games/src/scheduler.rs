use crate::AgentId;
use crate::TacticalState;

#[derive(Clone)]
pub struct CTScheduler {
    pub agents: Vec<AgentId>,
    pub ct_threshold: i32,
}

impl CTScheduler {
    pub fn new(ct_threshold: i32) -> Self {
        Self {
            agents: Vec::new(),
            ct_threshold,
        }
    }

    pub fn tick_until_ready(&self, state: &mut TacticalState) -> Vec<AgentId> {
        loop {
            let mut ready = Vec::new();
            for (&id, agent) in state.agents.iter() {
                if agent.ct >= self.ct_threshold {
                    ready.push(id);
                }
            }

            if !ready.is_empty() {
                // Sort by CT (descending) then by WITS (descending) for stability
                ready.sort_by(|a, b| {
                    let agent_a = &state.agents[a];
                    let agent_b = &state.agents[b];
                    let pos_a = self.agents.iter().position(|&id| id == *a).unwrap_or(usize::MAX);
                    let pos_b = self.agents.iter().position(|&id| id == *b).unwrap_or(usize::MAX);
                    agent_b.ct.cmp(&agent_a.ct)
                        .then(agent_b.stats.wits.cmp(&agent_a.stats.wits))
                        .then(pos_a.cmp(&pos_b)) // flipped because we want the lower value to take precedence
                });
                return ready;
            }

            // Tick
            for agent in state.agents.values_mut() {
                agent.ct += agent.stats.speed;
            }
        }
    }

    pub fn initialize_ct(&self, state: &mut TacticalState) {
        for agent in state.agents.values_mut() {
            agent.ct = agent.stats.wits;
        }
    }

    pub fn calculate_deduction(&self, ap_spent: i32, ap_max: i32) -> i32 {
        if ap_max == 0 { return self.ct_threshold; }
        (self.ct_threshold * ap_spent) / ap_max
    }
}
