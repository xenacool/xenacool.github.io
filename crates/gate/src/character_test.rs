#[cfg(test)]
mod tests {
    use pystral_compiler::assets::AssetCollection;
    use pystral_core::history::HistoryManager;
    use pystral_core::log::PropertyValue;
    use pystral_runtime::demo::generate_demo_log;

    #[test]
    fn test_characters_visibility() {
        let (atlas_json, spritesheet_rgba, width) = crate::load_test_assets();
        let mut history = HistoryManager::new();
        generate_demo_log(&mut history, &atlas_json, &spritesheet_rgba, width);

        // We want to check the state after characters are spawned and asset collection is defined.
        // In demo.rhai, it seems to happen early.
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

            if !characters.is_empty() && state.asset_collections.contains_key("primitives") {
                found_characters = true;

                // Check asset collection
                let collection_data = state
                    .asset_collections
                    .get("primitives")
                    .expect("primitives asset collection should be defined");

                let collection = AssetCollection::from_binary(collection_data);

                for char_entity in characters {
                    let asset_prop = match char_entity.properties.get("asset") {
                        Some(p) => p,
                        None => {
                            found_characters = false; // Not fully initialized yet
                            break;
                        }
                    };

                    if let PropertyValue::String(asset_name) = asset_prop {
                        assert!(
                            collection.spritestacks.contains_key(asset_name),
                            "Asset {} not found in primitives collection for character {}",
                            asset_name,
                            char_entity.kind
                        );

                        let stack = &collection.spritestacks[asset_name];
                        assert!(
                            !stack.slices.is_empty(),
                            "Asset {} for character {} has no slices",
                            asset_name,
                            char_entity.kind
                        );

                        println!(
                            "Character {} (kind: {}) has asset {} with {} slices",
                            char_entity.id,
                            char_entity.kind,
                            asset_name,
                            stack.slices.len()
                        );
                    } else {
                        panic!(
                            "Asset property is not a string for character {}",
                            char_entity.kind
                        );
                    }
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

        assert!(found_characters, "No character entities found in demo log");
    }
}
