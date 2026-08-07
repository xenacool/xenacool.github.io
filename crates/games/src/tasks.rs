pub use npc_engine_core::{Behavior, Context, ContextMut, Task, TaskDuration, impl_task_boxed_methods, AgentId};
use crate::{TacticalDomain, TacticalAccess, TacticalAccessMut, ReactionId, GridCell, TacticalDisplayAction, AbilityId, TagRegistry, Logger, ModifierCard, calculate_damage};

pub struct ReactionBehavior;
impl Behavior<TacticalDomain> for ReactionBehavior {
    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        !ctx.state_diff.initial_state.reaction_queue.is_empty() || !ctx.state_diff.diff.reaction_queue.is_empty()
    }

    fn add_own_tasks(&self, ctx: Context<TacticalDomain>, tasks: &mut Vec<Box<dyn Task<TacticalDomain>>>) {
        let state = ctx.state_diff.initial_state;
        let diff = ctx.state_diff.diff;
        
        let reaction = diff.reaction_queue.iter()
            .chain(state.reaction_queue.iter())
            .find(|(agent_id, _, _)| *agent_id == ctx.agent);
            
        if let Some((_, reaction_id, target_id)) = reaction {
             tasks.push(Box::new(ReactionTask {
                 agent: ctx.agent,
                 reaction_id: *reaction_id,
                 target: *target_id,
             }));
        }
    }
}

pub struct MoveBehavior;
impl Behavior<TacticalDomain> for MoveBehavior {
    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        if !ctx.state_diff.initial_state.reaction_queue.is_empty() || !ctx.state_diff.diff.reaction_queue.is_empty() {
            return false;
        }
        if let Some(unit) = ctx.state_diff.get_agent(ctx.agent) {
            unit.action_points > 0
        } else {
            false
        }
    }

    fn add_own_tasks(&self, ctx: Context<TacticalDomain>, tasks: &mut Vec<Box<dyn Task<TacticalDomain>>>) {
        let state = ctx.state_diff.initial_state;
        let unit = ctx.state_diff.get_agent(ctx.agent).unwrap();
        let prog = &state.movement_registry[&unit.movement_ability];
        
        if unit.movement_ability.0 == 301 { // Shadow Step (Teleport)
            // Can teleport to any cell in range 2 for simplicity
            for q in -2..=2 {
                for r in -2..=2 {
                    let s: i32 = -q - r;
                    if s.abs() <= 2 {
                        if q == 0 && r == 0 && s == 0 { continue; }
                        let target_pos = (unit.position.0 + q, unit.position.1 + r, unit.position.2 + s);
                        let mut tag_bag = unit.turn_tags.clone();
                        let cost = prog.get_ap_cost(0, &mut tag_bag);
                        if unit.action_points >= cost as i32 {
                            tasks.push(Box::new(MoveTask { agent: ctx.agent, to: target_pos, ap_cost: cost }));
                        }
                    }
                }
            }
        } else {
            // Simple 1-cell move in 6 directions
            let directions = [(1, -1, 0), (1, 0, -1), (0, 1, -1), (-1, 1, 0), (-1, 0, 1), (0, -1, 1)];
            for (dq, dr, ds) in directions {
                let target_pos = (unit.position.0 + dq, unit.position.1 + dr, unit.position.2 + ds);
                let mut tag_bag = unit.turn_tags.clone();
                let cost = prog.get_ap_cost(0, &mut tag_bag);
                if unit.action_points >= cost as i32 {
                    tasks.push(Box::new(MoveTask { agent: ctx.agent, to: target_pos, ap_cost: cost }));
                }
            }
        }
    }
}

pub struct AbilityBehavior;
impl Behavior<TacticalDomain> for AbilityBehavior {
    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        if !ctx.state_diff.initial_state.reaction_queue.is_empty() || !ctx.state_diff.diff.reaction_queue.is_empty() {
            return false;
        }
        if let Some(unit) = ctx.state_diff.get_agent(ctx.agent) {
            unit.action_points > 0
        } else {
            false
        }
    }

    fn add_own_tasks(&self, ctx: Context<TacticalDomain>, tasks: &mut Vec<Box<dyn Task<TacticalDomain>>>) {
        let state = ctx.state_diff.initial_state;
        let unit = ctx.state_diff.get_agent(ctx.agent).unwrap();
        
        let job = &state.job_registry[&unit.primary_job];
        for &ability_id in &job.abilities {
            let ability_def = &state.ability_registry[&ability_id];
            let mut tag_bag = unit.turn_tags.clone();
            let cost = ability_def.get_ap_cost(&mut tag_bag);
            
            if unit.action_points >= cost as i32 {
                // Find targets (all other units for now)
                for (&target_id, target_unit) in &state.agents {
                    if target_id != ctx.agent && target_unit.team_id != unit.team_id {
                        tasks.push(Box::new(AbilityTask {
                            agent: ctx.agent,
                            target: target_id,
                            ability_id,
                            ap_cost: cost,
                        }));
                    }
                }
            }
        }
    }
}

pub struct WaitBehavior;
impl Behavior<TacticalDomain> for WaitBehavior {
    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        ctx.state_diff.initial_state.reaction_queue.is_empty() && ctx.state_diff.diff.reaction_queue.is_empty()
    }

    fn add_own_tasks(&self, ctx: Context<TacticalDomain>, tasks: &mut Vec<Box<dyn Task<TacticalDomain>>>) {
        tasks.push(Box::new(WaitTask { agent: ctx.agent }));
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ReactionTask {
    pub agent: AgentId,
    pub reaction_id: ReactionId,
    pub target: AgentId,
}

impl Task<TacticalDomain> for ReactionTask {
    fn duration(&self, _ctx: Context<TacticalDomain>) -> TaskDuration {
        0
    }

    fn execute(&self, mut ctx: ContextMut<TacticalDomain>) -> Option<Box<dyn Task<TacticalDomain>>> {
        let reaction_def = ctx.state_diff.initial_state.reaction_registry.get(&self.reaction_id).cloned();
        let mut queue = ctx.state_diff.initial_state.reaction_queue.clone();
        queue.extend(ctx.state_diff.diff.reaction_queue.iter().cloned());
        
        if let Some(pos) = queue.iter().position(|r| *r == (self.agent, self.reaction_id, self.target)) {
            queue.remove(pos);
        }
        ctx.state_diff.diff.reaction_queue = queue;

        // Reaction logic
        if let Some(unit) = ctx.state_diff.get_agent_mut(self.agent) {
             if let Some(def) = reaction_def {
                 unit.action_points -= def.ap_cost as i32;
                 
                 let attacker_id = self.target;
                 match self.reaction_id.0 {
                     101 => { // Counter-Swing (Caveman)
                         if let Some(target_unit) = ctx.state_diff.get_agent_mut(attacker_id) {
                             target_unit.health -= 10;
                         }
                     },
                     201 => { // Mana Shield (Mage)
                         unit.health += 5;
                         unit.mana -= 10;
                     },
                     301 => { // Vengeful Spirit (Necromancer)
                         if let Some(target_unit) = ctx.state_diff.get_agent_mut(attacker_id) {
                             target_unit.health -= 15;
                         }
                     },
                     401 => { // Bone Splinter (Skeleton)
                         if let Some(target_unit) = ctx.state_diff.get_agent_mut(attacker_id) {
                             target_unit.health -= 5;
                         }
                     },
                     _ => {}
                 }
             }
        }
        
        None
    }

    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        let state = ctx.state_diff.initial_state;
        let diff = ctx.state_diff.diff;
        diff.reaction_queue.iter().chain(state.reaction_queue.iter()).any(|r| *r == (self.agent, self.reaction_id, self.target))
    }

    fn display_action(&self) -> TacticalDisplayAction {
        TacticalDisplayAction::Wait // Placeholder
    }

    impl_task_boxed_methods!(TacticalDomain);
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MoveTask {
    pub agent: AgentId,
    pub to: (i32, i32, i32),
    pub ap_cost: u8,
}

impl Task<TacticalDomain> for MoveTask {
    fn duration(&self, _ctx: Context<TacticalDomain>) -> TaskDuration {
        0
    }

    fn execute(&self, mut ctx: ContextMut<TacticalDomain>) -> Option<Box<dyn Task<TacticalDomain>>> {
        let tag_registry = ctx.state_diff.initial_state.tag_registry.clone();
        let movement_registry = ctx.state_diff.initial_state.movement_registry.clone();
        
        if let Some(unit) = ctx.state_diff.get_agent_mut(self.agent) {
            unit.position = self.to;
            unit.action_points -= self.ap_cost as i32;
            
            let move_ability_id = unit.movement_ability;
            let prog = movement_registry.get(&move_ability_id).cloned();
            if let Some(prog) = prog {
                for (tag, n) in prog.emit_tags {
                    let mut dummy_logger = Logger::default();
                    unit.turn_tags.emit(tag, n, &TagRegistry { defs: tag_registry.defs.clone() }, &mut dummy_logger);
                    
                    // Manafeet logic: if tag 10 is emitted, grant MP
                    if tag.0 == 10 {
                         unit.mana = (unit.mana + 5).min(unit.derived_stats.mana_max);
                    }
                }
            }
        }
        None
    }

    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        let unit = ctx.state_diff.get_agent(self.agent);
        if let Some(unit) = unit {
            unit.action_points >= self.ap_cost as i32
        } else {
            false
        }
    }

    fn display_action(&self) -> TacticalDisplayAction {
        TacticalDisplayAction::Move { to: GridCell { q: self.to.0, r: self.to.1, s: self.to.2 } }
    }

    impl_task_boxed_methods!(TacticalDomain);
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct AbilityTask {
    pub agent: AgentId,
    pub target: AgentId,
    pub ability_id: AbilityId,
    pub ap_cost: u8,
}

impl Task<TacticalDomain> for AbilityTask {
    fn duration(&self, _ctx: Context<TacticalDomain>) -> TaskDuration {
        0
    }

    fn execute(&self, mut ctx: ContextMut<TacticalDomain>) -> Option<Box<dyn Task<TacticalDomain>>> {
        let ability_def = ctx.state_diff.initial_state.ability_registry.get(&self.ability_id).cloned();
        let tag_registry = ctx.state_diff.initial_state.tag_registry.clone();
        
        if let Some(ability_def) = ability_def {
            let health_change;
            
            let mut card = ModifierCard::Plus0;
            let mut rng = ctx.state_diff.initial_state.rng.clone(); 
            
            if let Some(attacker) = ctx.state_diff.get_agent_mut(self.agent) {
                card = attacker.modifier_deck.draw(&mut rng);
                attacker.action_points -= self.ap_cost as i32;
                let mut dummy_logger = Logger::default();
                for (tag, n) in &ability_def.emit_tags {
                    attacker.turn_tags.emit(*tag, *n, &TagRegistry { defs: tag_registry.defs.clone() }, &mut dummy_logger);
                }
            }
            
            ctx.state_diff.diff.rng_update = Some(rng);
            
            let attacker_unit = ctx.state_diff.get_agent(self.agent).unwrap();
            let defender_unit = ctx.state_diff.get_agent(self.target).unwrap();
            let mut dummy_logger = Logger::default();
            let mut final_damage = calculate_damage(attacker_unit, defender_unit, &ability_def, card, "CON", &mut dummy_logger);
            
            // Spell Echo (201)
            if attacker_unit.passive_abilities.iter().any(|p| p.0 == 201) {
                final_damage *= 2;
            }
            
            health_change = -final_damage;

            let mut reaction_to_push = None;
            if let Some(defender) = ctx.state_diff.get_agent_mut(self.target) {
                defender.health += health_change;
                if health_change < 0 && !defender.reaction_abilities.is_empty() {
                    reaction_to_push = Some((self.target, defender.reaction_abilities[0], self.agent));
                }
                
                // Death's Embrace (301)
                if defender.health <= 0 {
                    if let Some(attacker) = ctx.state_diff.get_agent_mut(self.agent) {
                        if attacker.passive_abilities.iter().any(|p| p.0 == 301) {
                            attacker.health = (attacker.health + 20).min(attacker.derived_stats.health_max);
                        }
                    }
                }
            }
            if let Some(reaction) = reaction_to_push {
                ctx.state_diff.diff.reaction_queue.push(reaction);
            }
        }
        None
    }

    fn is_valid(&self, ctx: Context<TacticalDomain>) -> bool {
        if let Some(unit) = ctx.state_diff.get_agent(self.agent) {
            unit.action_points >= self.ap_cost as i32
        } else {
            false
        }
    }

    fn display_action(&self) -> TacticalDisplayAction {
        TacticalDisplayAction::Ability { target: self.target, ability: self.ability_id }
    }

    impl_task_boxed_methods!(TacticalDomain);
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct WaitTask {
    pub agent: AgentId,
}

impl Task<TacticalDomain> for WaitTask {
    fn duration(&self, _ctx: Context<TacticalDomain>) -> TaskDuration {
        10 // Duration until next CT fire
    }

    fn execute(&self, mut ctx: ContextMut<TacticalDomain>) -> Option<Box<dyn Task<TacticalDomain>>> {
        if let Some(unit) = ctx.state_diff.get_agent_mut(self.agent) {
            unit.action_points = unit.derived_stats.action_points_max;
            unit.turn_tags.counts.clear();
            unit.ct = 0;
        }
        None
    }

    fn is_valid(&self, _ctx: Context<TacticalDomain>) -> bool {
        true
    }

    fn display_action(&self) -> TacticalDisplayAction {
        TacticalDisplayAction::Wait
    }

    impl_task_boxed_methods!(TacticalDomain);
}
