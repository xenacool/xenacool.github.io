#[cfg(test)]
mod tests {
    use pystral_core::history::HistoryManager;
    use pystral_runtime::demo::generate_demo_log;
    use pystral_core::log::PropertyValue;
    use pystral_compiler::assets::AssetCollection;

    #[test]
    fn test_characters_visibility() {
        let mut history = HistoryManager::new();
        generate_demo_log(&mut history);

        // We want to check the state after characters are spawned and asset collection is defined.
        // In demo.rhai, it seems to happen early.
        // Let's find a step where at least one character is spawned.
        
        let mut found_characters = false;
        let total_steps = history.log.len();
        
        for i in 0..total_steps {
            history.jump_to(i);
            let state = &history.current_state;
            
            let characters: Vec<_> = state.entities.iter()
                .filter(|e| {
                    e.kind == "skeleton_minion" || e.kind == "necromancer" || e.kind == "caveman" || e.kind == "mage"
                })
                .collect();
            
            if !characters.is_empty() && state.asset_collections.contains_key("primitives") {
                found_characters = true;
                
                // Check asset collection
                let collection_data = state.asset_collections.get("primitives")
                    .expect("primitives asset collection should be defined");
                
                let collection = AssetCollection::from_binary(collection_data);
                
                for char_entity in characters {
                    let asset_prop = char_entity.properties.get("asset")
                        .expect(&format!("Character {} ({}) missing asset property", char_entity.id, char_entity.kind));
                    
                    if let PropertyValue::String(asset_name) = asset_prop {
                        assert!(collection.spritestacks.contains_key(asset_name), 
                            "Asset {} not found in primitives collection for character {}", asset_name, char_entity.kind);
                        
                        let stack = &collection.spritestacks[asset_name];
                        assert!(!stack.slices.is_empty(), "Asset {} for character {} has no slices", asset_name, char_entity.kind);
                        
                        println!("Character {} (kind: {}) has asset {} with {} slices", char_entity.id, char_entity.kind, asset_name, stack.slices.len());
                    } else {
                        panic!("Asset property is not a string for character {}", char_entity.kind);
                    }
                }
                
                break; // Checked one state where they are present
            }
        }
        
        assert!(found_characters, "No character entities found in demo log");
    }
}
