use crate::{
    DerivedStat, ModifierStacking, RPGBytecode, RPGHook, RPGProgram, RPGPrograms, ScriptAbilityDef,
    ScriptJobDef, ScriptMovementDef, ScriptPassiveDef, ScriptReactionDef, ScriptTagDef,
    SkirmishConfig, SlotType, UnitStats, add_rpg_program,
};

pub(super) fn builtin_script_abilities() -> Vec<ScriptAbilityDef> {
    vec![
        script_ability("Club Smash", 2, 1, "Melee", vec![("STR", 1.5)], vec![]),
        script_ability(
            "Rock Throw",
            2,
            6,
            "StraightProjectile",
            vec![("STR", 1.0), ("DEX", 0.5)],
            vec![],
        ),
        script_ability(
            "Primal Roar",
            3,
            3,
            "Area",
            vec![("CHA", 0.5)],
            vec![("Stun", 1)],
        ),
        script_ability(
            "Fireball",
            3,
            8,
            "ArcProjectile",
            vec![("INT", 2.0)],
            vec![],
        ),
        script_ability(
            "Frost Bolt",
            2,
            6,
            "StraightProjectile",
            vec![("INT", 1.0), ("WIS", 0.5)],
            vec![("Slow", 1)],
        ),
        script_ability(
            "Arcane Shield",
            2,
            0,
            "SelfTarget",
            vec![("INT", 0.5), ("WIS", 1.0)],
            vec![("Shield", 2)],
        ),
        script_ability("Raise Skeleton", 2, 1, "Area", vec![], vec![]),
        script_ability(
            "Harvest Skeleton",
            1,
            4,
            "Melee",
            vec![],
            vec![("Fresh Soul", 1)],
        ),
        script_ability(
            "Soul Drain",
            3,
            4,
            "StraightProjectile",
            vec![("INT", 1.0), ("CHA", 1.0)],
            vec![],
        ),
        script_ability(
            "Bone Armor",
            2,
            0,
            "SelfTarget",
            vec![("INT", 1.0)],
            vec![("Armor", 2)],
        ),
        script_ability("Bony Strike", 1, 1, "Melee", vec![("STR", 1.0)], vec![]),
        script_ability(
            "Shield Bash",
            2,
            1,
            "Melee",
            vec![("STR", 0.5), ("CON", 0.5)],
            vec![("Stun", 1)],
        ),
        script_ability(
            "Screech",
            2,
            3,
            "Area",
            vec![("CHA", 0.2)],
            vec![("Fear", 1)],
        ),
    ]
}

pub(super) fn populate(mut config: SkirmishConfig) -> SkirmishConfig {
    for tag in [
        ("Stun", 1),
        ("Slow", 1),
        ("Shield", 2),
        ("Armor", 2),
        ("Fear", 1),
        ("Mana", 1),
        ("Fresh Soul", 1),
    ] {
        config.script_tags.push(ScriptTagDef {
            name: tag.0.to_string(),
            max_stacks: Some(tag.1),
        });
    }
    for passive in [
        "Thick Skin",
        "Spell Echo",
        "Death's Embrace",
        "Undead Resilience",
    ] {
        let mut definition = ScriptPassiveDef {
            name: passive.to_string(),
            stat_modifiers: Vec::new(),
            damage_multiplier: None,
            on_kill_heal: 0,
        };
        match passive {
            "Thick Skin" => definition.stat_modifiers.push((DerivedStat::ArmorClass, 5)),
            "Spell Echo" => definition.damage_multiplier = Some((2, 1)),
            "Death's Embrace" => definition.on_kill_heal = 20,
            "Undead Resilience" => definition
                .stat_modifiers
                .extend([(DerivedStat::ArmorClass, 2), (DerivedStat::Constitution, 2)]),
            _ => {}
        }
        config.script_passives.push(definition);
    }
    for (name, ap_cost) in [
        ("Counter-Swing", 1),
        ("Mana Shield", 0),
        ("Vengeful Spirit", 1),
        ("Bone Splinter", 0),
    ] {
        let (mana_cost, target_damage, self_heal) = match name {
            "Counter-Swing" => (0, 10, 0),
            "Mana Shield" => (10, 0, 5),
            "Vengeful Spirit" => (0, 15, 0),
            "Bone Splinter" => (0, 5, 0),
            _ => (0, 0, 0),
        };
        config.script_reactions.push(ScriptReactionDef {
            name: name.to_string(),
            ap_cost: Some(ap_cost),
            mana_cost,
            target_damage,
            self_heal,
        });
    }
    config.script_movements = vec![
        ScriptMovementDef {
            name: "Plain Move".into(),
            steps_ap_cost: vec![(1, 1)],
            vertical_deltas: vec![],
            crosses_holes: false,
            crosses_occupied: false,
            teleport_range: None,
            emit_tags: vec![],
            consume_tags: vec![],
            mana_gain: 0,
        },
        ScriptMovementDef {
            name: "Manafeet".into(),
            steps_ap_cost: vec![(1, 1)],
            vertical_deltas: vec![],
            crosses_holes: false,
            crosses_occupied: false,
            teleport_range: None,
            emit_tags: vec![("Mana".into(), 1)],
            consume_tags: vec![],
            mana_gain: 5,
        },
        ScriptMovementDef {
            name: "Shadow Step".into(),
            steps_ap_cost: vec![(1, 2)],
            vertical_deltas: vec![],
            crosses_holes: true,
            crosses_occupied: false,
            teleport_range: Some(2),
            emit_tags: vec![],
            consume_tags: vec![],
            mana_gain: 0,
        },
        ScriptMovementDef {
            name: "Rattle Dash".into(),
            steps_ap_cost: vec![(1, 1), (4, 2)],
            vertical_deltas: vec![],
            crosses_holes: false,
            crosses_occupied: false,
            teleport_range: None,
            emit_tags: vec![],
            consume_tags: vec![],
            mana_gain: 0,
        },
    ];
    config.script_abilities = builtin_script_abilities();
    config.script_jobs = builtin_script_jobs();
    config
}

fn script_ability(
    name: &str,
    ap_cost: u8,
    range: u8,
    delivery: &str,
    scaling: Vec<(&str, f32)>,
    emit_tags: Vec<(&str, u8)>,
) -> ScriptAbilityDef {
    let mut programs = RPGPrograms::new();
    if name == "Arcane Shield" {
        let mut shield = RPGProgram::new();
        shield
            .add_timed_modifier(RPGBytecode::AddTimedModifier {
                stat: DerivedStat::ArmorClass,
                amount: 4,
                duration_turns: 2,
                stacking: ModifierStacking::RefreshReplace,
            })
            .expect("builtin effect is valid");
        add_rpg_program(&mut programs, RPGHook::OnAbilityResolve, shield)
            .expect("builtin effect hook is unique");
    }
    let mut definition = ScriptAbilityDef {
        name: name.into(),
        ap_cost: Some(ap_cost),
        health_cost: None,
        mana_cost: None,
        health_floor: None,
        range: Some(range),
        delivery: Some(delivery.into()),
        target_rule: None,
        projectile_profile: match name {
            "Rock Throw" => Some("Rock".into()),
            "Fireball" => Some("FireballOrb".into()),
            "Frost Bolt" => Some("FrostBolt".into()),
            "Soul Drain" => Some("SoulOrb".into()),
            _ => None,
        },
        area_radius: None,
        scaling: scaling
            .into_iter()
            .map(|(name, factor)| (name.into(), factor))
            .collect(),
        emit_tags: emit_tags
            .into_iter()
            .map(|(name, stacks)| (name.into(), stacks))
            .collect(),
        consume_tags: vec![],
        facing_rule: None,
        front_damage_multiplier: None,
        side_damage_multiplier: None,
        rear_damage_multiplier: None,
        programs,
    };
    match name {
        "Raise Skeleton" => {
            definition.health_cost = Some(20);
            definition.health_floor = Some(1);
            definition.target_rule = Some("EmptyCell".into());
            let mut program = RPGProgram::new();
            program
                .add_op(RPGBytecode::SpawnOwnedUnit {
                    job: "Skeleton_Minion".into(),
                    maximum_active: 2,
                })
                .expect("builtin spawn effect is valid");
            add_rpg_program(&mut definition.programs, RPGHook::OnAbilityResolve, program)
                .expect("builtin effect hook is unique");
        }
        "Harvest Skeleton" => {
            definition.target_rule = Some("OwnedSummon".into());
            let mut program = RPGProgram::new();
            program
                .add_op(RPGBytecode::DestroyOwnedSummon)
                .expect("builtin destroy effect is valid");
            program
                .add_op(RPGBytecode::HealCaster { amount: 10 })
                .expect("builtin heal effect is valid");
            program
                .add_op(RPGBytecode::ModifyCasterMana { amount: 10 })
                .expect("builtin mana effect is valid");
            add_rpg_program(&mut definition.programs, RPGHook::OnAbilityResolve, program)
                .expect("builtin effect hook is unique");
        }
        "Soul Drain" => {
            definition.mana_cost = Some(10);
            definition.consume_tags.push(("Fresh Soul".into(), 1, 1));
            let mut program = RPGProgram::new();
            program
                .add_op(RPGBytecode::HealCasterFromActualDamage {
                    numerator: 1,
                    denominator: 2,
                })
                .expect("builtin lifesteal effect is valid");
            add_rpg_program(&mut definition.programs, RPGHook::OnAbilityResolve, program)
                .expect("builtin effect hook is unique");
        }
        _ => {}
    }
    definition
}

pub(super) fn builtin_script_jobs() -> Vec<ScriptJobDef> {
    vec![
        script_job(
            "Caveman",
            (15, 8, 4, 4, 6, 14, 8, 12, 12, 4),
            ["Club Smash", "Rock Throw", "Primal Roar"],
            "Thick Skin",
            "Counter-Swing",
            "Plain Move",
            vec!["MainHand", "Body", "Accessory"],
        ),
        script_job(
            "Mage",
            (4, 6, 16, 12, 8, 6, 12, 8, 8, 5),
            ["Fireball", "Frost Bolt", "Arcane Shield"],
            "Spell Echo",
            "Mana Shield",
            "Manafeet",
            vec!["MainHand", "Body", "Accessory"],
        ),
        script_job(
            "Necromancer",
            (6, 8, 14, 10, 12, 10, 10, 10, 10, 4),
            vec![
                "Raise Skeleton",
                "Harvest Skeleton",
                "Soul Drain",
                "Bone Armor",
            ],
            "Death's Embrace",
            "Vengeful Spirit",
            "Shadow Step",
            vec!["MainHand", "Body", "Accessory"],
        ),
        script_job(
            "Skeleton_Minion",
            (10, 12, 2, 2, 2, 8, 8, 8, 10, 6),
            ["Bony Strike", "Shield Bash", "Screech"],
            "Undead Resilience",
            "Bone Splinter",
            "Rattle Dash",
            vec!["MainHand", "OffHand", "Body"],
        ),
    ]
}

fn script_job(
    name: &str,
    stats: (i32, i32, i32, i32, i32, i32, i32, i32, i32, i32),
    abilities: impl IntoIterator<Item = &'static str>,
    passive: &str,
    reaction: &str,
    movement: &str,
    equipment_slots: Vec<&str>,
) -> ScriptJobDef {
    ScriptJobDef {
        name: name.into(),
        base_stats: Some(UnitStats {
            strength: stats.0,
            dexterity: stats.1,
            intelligence: stats.2,
            wisdom: stats.3,
            charisma: stats.4,
            constitution: stats.5,
            wits: stats.6,
            stamina: stats.7,
            armor_class: stats.8,
            speed: stats.9,
        }),
        equipment_slots: equipment_slots
            .into_iter()
            .map(|slot| match slot {
                "MainHand" => SlotType::MainHand,
                "OffHand" => SlotType::OffHand,
                "Head" => SlotType::Head,
                "Body" => SlotType::Body,
                _ => SlotType::Accessory,
            })
            .collect(),
        passive_slots_count: Some(1),
        reaction_slots_count: Some(1),
        secondary_job_slots_count: Some(1),
        ability_names: abilities.into_iter().map(String::from).collect(),
        passive_names: vec![passive.into()],
        reaction_names: vec![reaction.into()],
        movement_name: Some(movement.into()),
    }
}
