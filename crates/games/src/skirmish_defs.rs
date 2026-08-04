use super::*;

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
        // Summoning is self-originating for now; it is not a cell-area effect.
        script_ability(
            "Raise Skeleton",
            4,
            0,
            "SelfTarget",
            vec![("INT", 0.5)],
            vec![],
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

fn script_ability(
    name: &str,
    ap_cost: u8,
    range: u8,
    delivery: &str,
    scaling: Vec<(&str, f32)>,
    emit_tags: Vec<(&str, u8)>,
) -> ScriptAbilityDef {
    ScriptAbilityDef {
        name: name.into(),
        ap_cost: Some(ap_cost),
        range: Some(range),
        delivery: Some(delivery.into()),
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
    }
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
            ["Raise Skeleton", "Soul Drain", "Bone Armor"],
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
    abilities: [&str; 3],
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
