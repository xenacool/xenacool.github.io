use crate::pg_rpg::scripting;
use crate::pg_rpg::simulation::TacticalSimulation;
use npc_engine_core::AgentId;
use pystral_core::history::HistoryManager;
use rhai::{AST, CallFnOptions, Engine, Scope};

pub struct RhaiSession {
    engine: Engine,
    ast: AST,
    scope: Scope<'static>,
}

impl RhaiSession {
    #[cfg(test)]
    pub fn from_simulation(
        history: HistoryManager,
        simulation: TacticalSimulation,
    ) -> Result<Self, String> {
        Self::new(
            r#"
                fn resume_game() {
                    return sim.advance_to_boundary();
                }
                fn resume_game_budgeted(budget) {
                    return sim.advance_to_boundary_budgeted(budget);
                }
            "#,
            history,
            String::new(),
            Vec::new(),
            1,
        )
        .map(|mut session| {
            session.scope.set_value("sim", simulation);
            session
        })
    }

    pub fn new(
        script: &str,
        history: HistoryManager,
        atlas_json: String,
        spritesheet_rgba: Vec<u8>,
        spritesheet_width: u32,
    ) -> Result<Self, String> {
        let mut engine = Engine::new();
        scripting::register_all(&mut engine);
        let ast = engine.compile(script).map_err(|error| error.to_string())?;
        let mut scope = Scope::new();
        scope.push("history", history);
        scope.push("atlas_json", atlas_json);
        scope.push("spritesheet_rgba", rhai::Blob::from(spritesheet_rgba));
        scope.push("spritesheet_width", spritesheet_width as i64);
        let _: rhai::Dynamic = engine
            .eval_ast_with_scope(&mut scope, &ast)
            .map_err(|error| error.to_string())?;
        Ok(Self { engine, ast, scope })
    }

    #[allow(dead_code)]
    pub fn resume_game(&mut self) -> Result<Vec<AgentId>, String> {
        self.resume_game_fn("resume_game", ())
    }

    pub fn resume_game_budgeted(&mut self, budget: usize) -> Result<Vec<AgentId>, String> {
        self.resume_game_fn("resume_game_budgeted", (budget as i64,))
    }

    #[cfg(test)]
    pub fn run_authored_case(&mut self, case_name: &str) -> Result<(), String> {
        self.engine
            .call_fn_with_options(
                CallFnOptions::new().rewind_scope(true),
                &mut self.scope,
                &self.ast,
                case_name,
                (),
            )
            .map(|_: rhai::Dynamic| ())
            .map_err(|error| error.to_string())
    }

    fn resume_game_fn<T: rhai::FuncArgs>(
        &mut self,
        function: &str,
        args: T,
    ) -> Result<Vec<AgentId>, String> {
        let ready: rhai::Array = self
            .engine
            .call_fn_with_options(
                CallFnOptions::new().rewind_scope(true),
                &mut self.scope,
                &self.ast,
                function,
                args,
            )
            .map_err(|error| error.to_string())?;
        ready
            .into_iter()
            .map(|value| {
                value
                    .as_int()
                    .map(|id| AgentId(id as u32))
                    .map_err(|error| error.to_string())
            })
            .collect()
    }

    pub fn history(&mut self) -> Result<HistoryManager, String> {
        self.scope
            .get_value("history")
            .ok_or_else(|| "Rhai session has no history state".to_string())
    }

    pub fn simulation(&mut self) -> Result<TacticalSimulation, String> {
        self.scope
            .get_value("sim")
            .ok_or_else(|| "Rhai session has no simulation state".to_string())
    }

    pub fn set_simulation(&mut self, simulation: TacticalSimulation) {
        self.scope.set_value("sim", simulation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resumable_script_reaches_the_player_boundary() {
        let mut scenario = pystral_games::SkirmishConfig::new(42);
        scenario
            .add_unit(
                1,
                1,
                "Caveman",
                pystral_games::GridCell::new(hexx::Hex::new(0, 0), 0),
            )
            .unwrap();
        scenario
            .add_unit(
                2,
                1,
                "Mage",
                pystral_games::GridCell::new(hexx::Hex::new(1, -1), 0),
            )
            .unwrap();
        scenario
            .add_unit(
                3,
                2,
                "Necromancer",
                pystral_games::GridCell::new(hexx::Hex::new(5, -5), 0),
            )
            .unwrap();
        scenario
            .add_unit(
                4,
                2,
                "Skeleton_Minion",
                pystral_games::GridCell::new(hexx::Hex::new(4, -4), 0),
            )
            .unwrap();
        let simulation = TacticalSimulation::from_scenario(
            scenario,
            npc_engine_core::MCTSConfiguration {
                visits: 50,
                depth: 10,
                seed: Some(42),
                ..Default::default()
            },
        );
        let mut session = RhaiSession::from_simulation(HistoryManager::new(), simulation).unwrap();

        let mut reached = false;
        for _ in 0..100 {
            let ready = session.resume_game().unwrap();
            if !ready.is_empty() {
                reached = true;
                break;
            }
        }
        assert!(reached);
        for _ in 0..1_000 {
            session.resume_game().unwrap();
        }
    }
}
