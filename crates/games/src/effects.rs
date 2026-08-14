use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{DerivedStat, ModifierStacking};

const MAX_RPG_PROGRAM_OPS: usize = 64;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum RPGHook {
    OnAbilityResolve,
    OnKill,
    OnMoveComplete,
    OnDamageTaken,
    PassiveStats,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum RPGBytecode {
    AddTimedModifier {
        stat: DerivedStat,
        amount: i32,
        duration_turns: u16,
        stacking: ModifierStacking,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RPGProgram {
    pub ops: Vec<RPGBytecode>,
}

impl RPGProgram {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    pub fn add_timed_modifier(&mut self, modifier: RPGBytecode) -> Result<(), String> {
        match modifier {
            RPGBytecode::AddTimedModifier { duration_turns, .. } if duration_turns == 0 => {
                return Err("Timed modifier duration must be greater than zero".to_string());
            }
            _ => {}
        }
        self.ops.push(modifier);
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.ops.len() > MAX_RPG_PROGRAM_OPS {
            return Err(format!(
                "RPG program has too many operations: {} > {MAX_RPG_PROGRAM_OPS}",
                self.ops.len()
            ));
        }
        for op in &self.ops {
            let RPGBytecode::AddTimedModifier { duration_turns, .. } = op;
            if *duration_turns == 0 {
                return Err("Timed modifier duration must be greater than zero".to_string());
            }
        }
        Ok(())
    }

    /// Compile the supported hook operations into typed runtime effects.
    /// Execution returns data for the caller's `TacticalDiff` boundary; the
    /// program never receives or mutates tactical state directly.
    pub fn execute_on_ability_resolve(&self) -> Result<Vec<crate::TimedModifier>, String> {
        self.validate()?;
        Ok(self
            .ops
            .iter()
            .map(|op| match op {
                RPGBytecode::AddTimedModifier {
                    stat,
                    amount,
                    duration_turns,
                    stacking,
                } => crate::TimedModifier {
                    stat: *stat,
                    amount: *amount,
                    remaining_turns: *duration_turns,
                    stacking: *stacking,
                },
            })
            .collect())
    }
}

pub type RPGPrograms = HashMap<RPGHook, Vec<RPGProgram>>;

pub fn add_rpg_program(
    programs: &mut RPGPrograms,
    hook: RPGHook,
    program: RPGProgram,
) -> Result<(), String> {
    program.validate()?;
    let entries = programs.entry(hook).or_default();
    if !entries.is_empty() {
        return Err(format!("RPG hook {hook:?} already has a program"));
    }
    entries.push(program);
    Ok(())
}

pub fn validate_rpg_programs(programs: &RPGPrograms) -> Result<(), String> {
    for (hook, entries) in programs {
        if !matches!(hook, RPGHook::OnAbilityResolve)
            && entries.iter().any(|program| !program.ops.is_empty())
        {
            return Err(format!("RPG bytecode is not supported for hook {hook:?}"));
        }
        for program in entries {
            program
                .validate()
                .map_err(|error| format!("Invalid RPG program for {hook:?}: {error}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_timed_modifier_programs_and_policy() {
        let mut program = RPGProgram::new();
        program
            .add_timed_modifier(RPGBytecode::AddTimedModifier {
                stat: DerivedStat::ArmorClass,
                amount: 3,
                duration_turns: 2,
                stacking: ModifierStacking::AdditiveIndependent,
            })
            .unwrap();
        assert!(program.validate().is_ok());
        assert!(
            program
                .add_timed_modifier(RPGBytecode::AddTimedModifier {
                    stat: DerivedStat::ArmorClass,
                    amount: 1,
                    duration_turns: 0,
                    stacking: ModifierStacking::RefreshReplace,
                })
                .is_err()
        );
        let mut programs = RPGPrograms::new();
        add_rpg_program(&mut programs, RPGHook::OnAbilityResolve, program).unwrap();
        assert!(validate_rpg_programs(&programs).is_ok());
    }

    #[test]
    fn rejects_duplicate_hooks_and_unbounded_programs() {
        let mut programs = RPGPrograms::new();
        let mut program = RPGProgram::new();
        program
            .add_timed_modifier(RPGBytecode::AddTimedModifier {
                stat: DerivedStat::ArmorClass,
                amount: 1,
                duration_turns: 1,
                stacking: ModifierStacking::RefreshReplace,
            })
            .unwrap();
        add_rpg_program(&mut programs, RPGHook::OnAbilityResolve, program.clone()).unwrap();
        assert!(add_rpg_program(&mut programs, RPGHook::OnAbilityResolve, program).is_err());

        let mut oversized = RPGProgram::new();
        oversized.ops = (0..65)
            .map(|_| RPGBytecode::AddTimedModifier {
                stat: DerivedStat::ArmorClass,
                amount: 1,
                duration_turns: 1,
                stacking: ModifierStacking::RefreshReplace,
            })
            .collect();
        assert!(oversized.validate().is_err());
    }
}
