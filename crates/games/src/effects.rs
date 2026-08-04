use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{DerivedStat, ModifierStacking};

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
        for op in &self.ops {
            let RPGBytecode::AddTimedModifier { duration_turns, .. } = op;
            if *duration_turns == 0 {
                return Err("Timed modifier duration must be greater than zero".to_string());
            }
        }
        Ok(())
    }
}

pub type RPGPrograms = HashMap<RPGHook, Vec<RPGProgram>>;

pub fn add_rpg_program(
    programs: &mut RPGPrograms,
    hook: RPGHook,
    program: RPGProgram,
) -> Result<(), String> {
    program.validate()?;
    programs.entry(hook).or_default().push(program);
    Ok(())
}

pub fn validate_rpg_programs(programs: &RPGPrograms) -> Result<(), String> {
    for (hook, entries) in programs {
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
}
