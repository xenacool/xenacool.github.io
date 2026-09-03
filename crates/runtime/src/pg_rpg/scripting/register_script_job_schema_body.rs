{
    engine
        .register_type_with_name::<RPGProgram>("RPGProgram")
        .register_fn("new_rpg_program", RPGProgram::new)
        .register_fn(
            "add_timed_modifier",
            |program: &mut RPGProgram,
             stat: &str,
             amount: i64,
             duration: i64,
             stacking: &str|
             -> Result<(), Box<rhai::EvalAltResult>> {
                let stat = match stat {
                    "ArmorClass" => DerivedStat::ArmorClass,
                    _ => return Err(script_error(format!("Unknown derived stat: {stat}"))),
                };
                let stacking = match stacking {
                    "AdditiveIndependent" => ModifierStacking::AdditiveIndependent,
                    "RefreshReplace" => ModifierStacking::RefreshReplace,
                    _ => {
                        return Err(script_error(format!(
                            "Unknown modifier stacking policy: {stacking}"
                        )));
                    }
                };
                let amount = i32::try_from(amount)
                    .map_err(|_| script_error("Modifier amount is out of range"))?;
                let duration = u16::try_from(duration)
                    .map_err(|_| script_error("Modifier duration is out of range"))?;
                program
                    .add_timed_modifier(RPGBytecode::AddTimedModifier {
                        stat,
                        amount,
                        duration_turns: duration,
                        stacking,
                    })
                    .map_err(script_error)
            },
        )
        .register_fn(
            "add_heal_caster",
            |program: &mut RPGProgram,
             amount: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                program
                    .add_op(RPGBytecode::HealCaster {
                        amount: i32::try_from(amount)
                            .map_err(|_| script_error("Heal amount is out of range"))?,
                    })
                    .map_err(script_error)
            },
        )
        .register_fn(
            "add_caster_mana",
            |program: &mut RPGProgram,
             amount: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                program
                    .add_op(RPGBytecode::ModifyCasterMana {
                        amount: i32::try_from(amount)
                            .map_err(|_| script_error("Mana amount is out of range"))?,
                    })
                    .map_err(script_error)
            },
        )
        .register_fn(
            "add_heal_from_actual_damage",
            |program: &mut RPGProgram,
             numerator: i64,
             denominator: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                program
                    .add_op(RPGBytecode::HealCasterFromActualDamage {
                        numerator: u16::try_from(numerator)
                            .map_err(|_| script_error("Heal numerator is out of range"))?,
                        denominator: u16::try_from(denominator)
                            .map_err(|_| script_error("Heal denominator is out of range"))?,
                    })
                    .map_err(script_error)
            },
        )
        .register_fn(
            "add_spawn_owned_unit",
            |program: &mut RPGProgram,
             job: &str,
             maximum_active: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                program
                    .add_op(RPGBytecode::SpawnOwnedUnit {
                        job: job.to_string(),
                        maximum_active: u8::try_from(maximum_active)
                            .map_err(|_| script_error("Summon cap is out of range"))?,
                    })
                    .map_err(script_error)
            },
        )
        .register_fn(
            "add_destroy_owned_summon",
            |program: &mut RPGProgram| {
                program
                    .add_op(RPGBytecode::DestroyOwnedSummon)
                    .map_err(script_error)
            },
        );
    engine
        .register_type_with_name::<RPGPrograms>("RPGPrograms")
        .register_fn("new_rpg_programs", RPGPrograms::new)
        .register_fn(
            "add_program",
            |programs: &mut RPGPrograms,
             hook: &str,
             program: RPGProgram|
             -> Result<(), Box<rhai::EvalAltResult>> {
                let hook = parse_hook(hook)?;
                add_rpg_program(programs, hook, program).map_err(script_error)
            },
        );
    engine
        .register_type_with_name::<ScriptTagDef>("ScriptTagDef")
        .register_fn("new_script_tag", |name: &str| ScriptTagDef {
            name: name.to_string(),
            max_stacks: None,
        })
        .register_fn(
            "set_max_stacks",
            |tag: &mut ScriptTagDef, max_stacks: i64| tag.max_stacks = Some(max_stacks as u8),
        );
    engine
        .register_type_with_name::<ScriptAbilityDef>("ScriptAbilityDef")
        .register_fn("new_script_ability", |name: &str| ScriptAbilityDef {
            name: name.to_string(),
            ap_cost: None,
            health_cost: None,
            mana_cost: None,
            health_floor: None,
            range: None,
            delivery: None,
            target_rule: None,
            projectile_profile: None,
            area_radius: None,
            scaling: Vec::new(),
            emit_tags: Vec::new(),
            consume_tags: Vec::new(),
            programs: RPGPrograms::new(),
            facing_rule: None,
            front_damage_multiplier: None,
            side_damage_multiplier: None,
            rear_damage_multiplier: None,
        })
        .register_fn(
            "set_ap_cost",
            |ability: &mut ScriptAbilityDef,
             cost: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                ability.ap_cost = Some(
                    u8::try_from(cost)
                        .map_err(|_| script_error("AP cost must be in the range 0..=255"))?,
                );
                Ok(())
            },
        )
        .register_fn(
            "set_health_cost",
            |ability: &mut ScriptAbilityDef,
             cost: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                ability.health_cost = Some(
                    u16::try_from(cost)
                        .map_err(|_| script_error("Health cost must be in the range 0..=65535"))?,
                );
                Ok(())
            },
        )
        .register_fn(
            "set_mana_cost",
            |ability: &mut ScriptAbilityDef,
             cost: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                ability.mana_cost = Some(
                    u16::try_from(cost)
                        .map_err(|_| script_error("Mana cost must be in the range 0..=65535"))?,
                );
                Ok(())
            },
        )
        .register_fn(
            "set_health_floor",
            |ability: &mut ScriptAbilityDef,
             floor: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                ability.health_floor = Some(
                    i32::try_from(floor)
                        .map_err(|_| script_error("Health floor is out of range"))?,
                );
                Ok(())
            },
        )
        .register_fn("set_range", |ability: &mut ScriptAbilityDef, range: i64| {
            ability.range = Some(range as u8)
        })
        .register_fn(
            "set_delivery",
            |ability: &mut ScriptAbilityDef, delivery: &str| {
                ability.delivery = Some(delivery.to_string())
            },
        )
        .register_fn(
            "set_target_rule",
            |ability: &mut ScriptAbilityDef, target_rule: &str| {
                ability.target_rule = Some(target_rule.to_string())
            },
        )
        .register_fn("set_facing_rule", |ability: &mut ScriptAbilityDef, rule: &str| {
            ability.facing_rule = Some(rule.to_string())
        })
        .register_fn("set_front_damage_multiplier", |ability: &mut ScriptAbilityDef, n: i64, d: i64| {
            ability.front_damage_multiplier = Some((n as u16, d as u16))
        })
        .register_fn("set_side_damage_multiplier", |ability: &mut ScriptAbilityDef, n: i64, d: i64| {
            ability.side_damage_multiplier = Some((n as u16, d as u16))
        })
        .register_fn("set_rear_damage_multiplier", |ability: &mut ScriptAbilityDef, n: i64, d: i64| {
            ability.rear_damage_multiplier = Some((n as u16, d as u16))
        })
        .register_fn(
            "set_projectile_profile",
            |ability: &mut ScriptAbilityDef, profile: &str| {
                ability.projectile_profile = Some(profile.to_string())
            },
        )
        .register_fn("set_area_radius", |ability: &mut ScriptAbilityDef, radius: i64| {
            ability.area_radius = Some(radius as u8)
        })
        .register_fn(
            "add_scaling",
            |ability: &mut ScriptAbilityDef, stat: &str, factor: f64| {
                ability.scaling.push((stat.to_string(), factor as f32))
            },
        )
        .register_fn(
            "add_emit_tag",
            |ability: &mut ScriptAbilityDef, tag: &str, stacks: i64| {
                ability.emit_tags.push((tag.to_string(), stacks as u8))
            },
        )
        .register_fn(
            "add_consume_tag",
            |ability: &mut ScriptAbilityDef, tag: &str, stacks: i64, discount: i64| {
                ability
                    .consume_tags
                    .push((tag.to_string(), stacks as u8, discount as u8))
            },
        );
    engine.register_fn(
        "set_programs",
        |ability: &mut ScriptAbilityDef, programs: RPGPrograms| ability.programs = programs,
    );
    engine
        .register_type_with_name::<UnitStats>("UnitStats")
        .register_fn(
            "unit_stats",
            |strength: i64,
             dexterity: i64,
             intelligence: i64,
             wisdom: i64,
             charisma: i64,
             constitution: i64,
             wits: i64,
             stamina: i64,
             armor_class: i64,
             speed: i64| UnitStats {
                strength: strength as i32,
                dexterity: dexterity as i32,
                intelligence: intelligence as i32,
                wisdom: wisdom as i32,
                charisma: charisma as i32,
                constitution: constitution as i32,
                wits: wits as i32,
                stamina: stamina as i32,
                armor_class: armor_class as i32,
                speed: speed as i32,
            },
        );
    engine
        .register_type_with_name::<ScriptJobDef>("ScriptJobDef")
        .register_fn("new_script_job", |name: &str| ScriptJobDef::new(name))
        .register_fn(
            "set_base_stats",
            |job: &mut ScriptJobDef, stats: UnitStats| job.base_stats = Some(stats),
        )
        .register_fn("set_passive_slots", |job: &mut ScriptJobDef, count: i64| {
            job.passive_slots_count = Some(count as u8)
        })
        .register_fn(
            "set_reaction_slots",
            |job: &mut ScriptJobDef, count: i64| job.reaction_slots_count = Some(count as u8),
        )
        .register_fn(
            "set_secondary_job_slots",
            |job: &mut ScriptJobDef, count: i64| job.secondary_job_slots_count = Some(count as u8),
        )
        .register_fn("add_ability", |job: &mut ScriptJobDef, name: &str| {
            job.ability_names.push(name.to_string())
        })
        .register_fn("add_passive", |job: &mut ScriptJobDef, name: &str| {
            job.passive_names.push(name.to_string())
        })
        .register_fn("add_reaction", |job: &mut ScriptJobDef, name: &str| {
            job.reaction_names.push(name.to_string())
        })
        .register_fn("set_movement", |job: &mut ScriptJobDef, name: &str| {
            job.movement_name = Some(name.to_string())
        })
        .register_fn(
            "add_equipment_slot",
            |job: &mut ScriptJobDef, name: &str| -> Result<(), Box<rhai::EvalAltResult>> {
                let slot = match name {
                    "MainHand" => SlotType::MainHand,
                    "OffHand" => SlotType::OffHand,
                    "Head" => SlotType::Head,
                    "Body" => SlotType::Body,
                    "Accessory" => SlotType::Accessory,
                    _ => {
                        return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            format!("Unknown equipment slot: {name}").into(),
                            rhai::Position::NONE,
                        )));
                    }
                };
                job.equipment_slots.push(slot);
                Ok(())
            },
        );
    engine
        .register_type_with_name::<ScriptPassiveDef>("ScriptPassiveDef")
        .register_fn("new_script_passive", |name: &str| ScriptPassiveDef {
            name: name.to_string(),
            stat_modifiers: Vec::new(),
            damage_multiplier: None,
            on_kill_heal: 0,
        })
        .register_fn(
            "add_stat_modifier",
            |passive: &mut ScriptPassiveDef, stat: &str, amount: i64| -> Result<(), Box<rhai::EvalAltResult>> {
                let stat = match stat {
                    "ArmorClass" => DerivedStat::ArmorClass,
                    "Constitution" => DerivedStat::Constitution,
                    _ => return Err(script_error(format!("Unknown passive stat: {stat}"))),
                };
                passive.stat_modifiers.push((stat, i32::try_from(amount).map_err(|_| script_error("Passive modifier is out of range"))?));
                Ok(())
            },
        )
        .register_fn(
            "set_damage_multiplier",
            |passive: &mut ScriptPassiveDef, numerator: i64, denominator: i64| -> Result<(), Box<rhai::EvalAltResult>> {
                passive.damage_multiplier = Some((
                    u16::try_from(numerator).map_err(|_| script_error("Damage multiplier numerator is out of range"))?,
                    u16::try_from(denominator).map_err(|_| script_error("Damage multiplier denominator is out of range"))?,
                ));
                Ok(())
            },
        )
        .register_fn(
            "set_on_kill_heal",
            |passive: &mut ScriptPassiveDef,
             amount: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                passive.on_kill_heal = i32::try_from(amount)
                    .map_err(|_| script_error("On-kill healing is out of range"))?;
                Ok(())
            },
        );
    engine
        .register_type_with_name::<ScriptReactionDef>("ScriptReactionDef")
        .register_fn("new_script_reaction", |name: &str| ScriptReactionDef {
            name: name.to_string(),
            ap_cost: None,
            mana_cost: 0,
            target_damage: 0,
            self_heal: 0,
        })
        .register_fn(
            "set_ap_cost",
            |reaction: &mut ScriptReactionDef,
             cost: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                reaction.ap_cost = Some(
                    u8::try_from(cost)
                        .map_err(|_| script_error("Reaction AP cost is out of range"))?,
                );
                Ok(())
            },
        )
        .register_fn(
            "set_mana_cost",
            |reaction: &mut ScriptReactionDef,
             cost: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                reaction.mana_cost = u16::try_from(cost)
                    .map_err(|_| script_error("Reaction mana cost is out of range"))?;
                Ok(())
            },
        )
        .register_fn(
            "set_target_damage",
            |reaction: &mut ScriptReactionDef,
             amount: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                reaction.target_damage = i32::try_from(amount)
                    .map_err(|_| script_error("Reaction damage is out of range"))?;
                Ok(())
            },
        )
        .register_fn(
            "set_self_heal",
            |reaction: &mut ScriptReactionDef,
             amount: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                reaction.self_heal = i32::try_from(amount)
                    .map_err(|_| script_error("Reaction healing is out of range"))?;
                Ok(())
            },
        );
    engine
        .register_type_with_name::<ScriptMovementDef>("ScriptMovementDef")
        .register_fn("new_script_movement", |name: &str| ScriptMovementDef {
            name: name.to_string(),
            steps_ap_cost: Vec::new(),
            vertical_deltas: Vec::new(),
            crosses_holes: false,
            crosses_occupied: false,
            teleport_range: None,
            emit_tags: Vec::new(),
            consume_tags: Vec::new(),
            mana_gain: 0,
            mana_cost: 0,
            health_cost: 0,
            health_floor: 1,
        })
        .register_fn(
            "add_step_cost",
            |movement: &mut ScriptMovementDef, threshold: i64, cost: i64| {
                movement.steps_ap_cost.push((threshold as u8, cost as u8))
            },
        )
        .register_fn(
            "add_vertical_delta",
            |movement: &mut ScriptMovementDef, delta: i64| {
                movement.vertical_deltas.push(delta as i32)
            },
        )
        .register_fn(
            "set_crosses_holes",
            |movement: &mut ScriptMovementDef, value: bool| movement.crosses_holes = value,
        )
        .register_fn(
            "set_crosses_occupied",
            |movement: &mut ScriptMovementDef, value: bool| movement.crosses_occupied = value,
        )
        .register_fn(
            "set_teleport_range",
            |movement: &mut ScriptMovementDef, range: i64| {
                movement.teleport_range = Some(range as u32)
            },
        )
        .register_fn(
            "add_emit_tag",
            |movement: &mut ScriptMovementDef, tag: &str, stacks: i64| {
                movement.emit_tags.push((tag.to_string(), stacks as u8))
            },
        )
        .register_fn(
            "add_consume_tag",
            |movement: &mut ScriptMovementDef,
             tag: &str,
             stacks: i64,
             discount: i64| {
                movement
                    .consume_tags
                    .push((tag.to_string(), stacks as u8, discount as u8))
            },
        )
        .register_fn(
            "set_mana_gain",
            |movement: &mut ScriptMovementDef,
             amount: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                movement.mana_gain = i32::try_from(amount)
                    .map_err(|_| script_error("Movement mana gain is out of range"))?;
                Ok(())
            },
        )
        .register_fn("set_mana_cost", |movement: &mut ScriptMovementDef, amount: i64| {
            movement.mana_cost = amount as u16;
        })
        .register_fn("set_health_cost", |movement: &mut ScriptMovementDef, amount: i64| {
            movement.health_cost = amount as u16;
        });
}
