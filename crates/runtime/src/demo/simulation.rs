use pystral_games::*;
use npc_engine_core::{MCTS, MCTSConfiguration, AgentId, ContextMut, StateDiffRefMut};
use npc_engine_utils::GlobalDomain;
use std::collections::HashMap;

#[derive(Clone)]
pub struct TacticalSimulation {
    pub state: TacticalState,
    pub scheduler: CTScheduler,
    pub config: MCTSConfiguration,
}

impl TacticalSimulation {
    pub fn new() -> Self {
        let mut state = setup_2v2_skirmish();
        let scheduler = CTScheduler::new(100);
        scheduler.initialize_ct(&mut state);
        
        let mut config = MCTSConfiguration::default();
        config.visits = 50; 
        config.depth = 10;
        config.seed = Some(42);
        
        Self {
            state,
            scheduler,
            config,
        }
    }

    pub fn step(&mut self) -> Vec<AgentId> {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&"Simulating step...".into());

        let ready_agents = self.scheduler.tick_until_ready(&mut self.state);
        
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("Ready agents: {:?}", ready_agents).into());

        for &agent_id in &ready_agents {
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&format!("Running MCTS for agent {:?}", agent_id).into());

            let mut mcts = MCTS::<TacticalDomain>::new(self.state.clone(), agent_id, self.config.clone());
            if let Some(task) = mcts.run() {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"MCTS finished, executing task".into());

                let mut diff = TacticalDiff::default();
                {
                    let ctx_mut = ContextMut {
                        tick: 0,
                        state_diff: StateDiffRefMut { initial_state: &self.state, diff: &mut diff },
                        agent: agent_id,
                    };
                    task.execute(ctx_mut);
                }
                let dummy_state = self.state.clone();
                TacticalDomain::apply(&mut self.state, &dummy_state, &diff);
            }
        }
        ready_agents
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
        }
        prompts
    }

    pub fn get_agent_position(&self, agent_id: i64) -> (i32, i32, i32) {
        self.state.agents.get(&AgentId(agent_id as u32)).map(|u| u.position).unwrap_or((0, 0, 0))
    }

    pub fn get_agent_health(&self, agent_id: i64) -> i32 {
        self.state.agents.get(&AgentId(agent_id as u32)).map(|u| u.health).unwrap_or(0)
    }

    pub fn list_agents(&self) -> Vec<i64> {
        self.state.agents.keys().map(|id| id.0 as i64).collect()
    }
}
