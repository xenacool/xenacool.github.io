#[cfg(test)]
mod tests {
    use pystral_core::history::HistoryManager;
    use pystral_runtime::pg_rpg::generate_pg_rpg_log;

    #[test]
    fn test_characters_visibility() {
        let (atlas_json, spritesheet_rgba, width) = crate::load_test_assets();
        let mut history = HistoryManager::new();
        generate_pg_rpg_log(&mut history, &atlas_json, &spritesheet_rgba, width);

        // We want to check the state after characters are spawned and asset collection is defined.
        // In pg_rpg.rhai, it seems to happen early.
        // Let's find a step where at least one character is spawned.

        let mut found_characters = false;
        let total_steps = history.log.len();

        for i in 0..total_steps {
            history.jump_to(i);
            let state = &history.current_state;

            println!("Step {}: {} entities", i, state.entities.len());
            for e in &state.entities {
                println!("  Entity {}: kind={}", e.id, e.kind);
            }

            let characters: Vec<_> = state
                .entities
                .iter()
                .filter(|e| {
                    e.kind == "Skeleton_Minion"
                        || e.kind == "Necromancer"
                        || e.kind == "Caveman"
                        || e.kind == "Mage"
                })
                .collect();

            if !characters.is_empty() {
                found_characters = true;

                for char_entity in characters {
                    if !char_entity.properties.contains_key("asset") {
                        found_characters = false;
                        break;
                    }
                    assert_eq!(
                        char_entity.properties.get("asset").and_then(|value| match value {
                            pystral_core::log::PropertyValue::String(name) => Some(name),
                            pystral_core::log::PropertyValue::AssetRef(name) => Some(name),
                            _ => None,
                        }),
                        Some(&char_entity.kind),
                        "character asset must be a direct GLB manifest key"
                    );
                }

                if found_characters {
                    break; // Checked one state where they are present and initialized
                }
            }
        }

        if !found_characters {
            println!("History log length: {}", history.log.len());
            for (i, event) in history.log.iter().enumerate() {
                println!("Event {}: {:?}", i, event);
            }
        }

        assert!(
            found_characters,
            "No character entities found in pg_rpg log"
        );
    }
}
