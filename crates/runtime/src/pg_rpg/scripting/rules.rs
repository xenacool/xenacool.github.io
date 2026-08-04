use pystral_games::{
    DerivedStat, ModifierStacking, RPGBytecode, RPGHook, RPGProgram, RPGPrograms, ScriptAbilityDef,
    ScriptJobDef, ScriptMovementDef, ScriptPassiveDef, ScriptReactionDef, ScriptTagDef, SlotType,
    UnitStats, add_rpg_program,
};
use rhai::Engine;

pub fn register_script_job_schema(engine: &mut Engine) {
    include!("register_script_job_schema_body.rs");
}
fn script_error(error: impl Into<String>) -> Box<rhai::EvalAltResult> {
    Box::new(rhai::EvalAltResult::ErrorRuntime(
        error.into().into(),
        rhai::Position::NONE,
    ))
}

fn parse_hook(hook: &str) -> Result<RPGHook, Box<rhai::EvalAltResult>> {
    match hook {
        "OnAbilityResolve" => Ok(RPGHook::OnAbilityResolve),
        "OnKill" => Ok(RPGHook::OnKill),
        "OnMoveComplete" => Ok(RPGHook::OnMoveComplete),
        "OnDamageTaken" => Ok(RPGHook::OnDamageTaken),
        "PassiveStats" => Ok(RPGHook::PassiveStats),
        _ => Err(script_error(format!("Unknown RPG hook: {hook}"))),
    }
}
