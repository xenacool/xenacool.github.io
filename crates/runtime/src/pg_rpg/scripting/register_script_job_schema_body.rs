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
            range: None,
            delivery: None,
            area_radius: None,
            scaling: Vec::new(),
            emit_tags: Vec::new(),
            consume_tags: Vec::new(),
            programs: RPGPrograms::new(),
        })
        .register_fn(
            "set_ap_cost",
            |ability: &mut ScriptAbilityDef, cost: i64| ability.ap_cost = Some(cost as u8),
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
        });
    engine
        .register_type_with_name::<ScriptReactionDef>("ScriptReactionDef")
        .register_fn("new_script_reaction", |name: &str| ScriptReactionDef {
            name: name.to_string(),
            ap_cost: None,
        })
        .register_fn(
            "set_ap_cost",
            |reaction: &mut ScriptReactionDef, cost: i64| reaction.ap_cost = Some(cost as u8),
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
        );
}
