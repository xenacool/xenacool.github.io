pub mod ability_targets;
pub mod animation;
pub mod bundle;
mod presentation;
pub mod scripting;
pub mod simulation;

pub use bundle::{
    AssetManifest, NamedBinaryAsset, NamedTextAsset, ScenarioBundle, VirtualRhaiWorkspace,
};
use pystral_core::history::HistoryManager;
use pystral_core::log::Event;

pub fn generate_pg_rpg_log(
    history: &mut HistoryManager,
    atlas_json: &str,
    spritesheet_rgba: &[u8],
    spritesheet_width: u32,
) {
    match runtime_bundle() {
        Ok(bundle) => generate_pg_rpg_log_bundle(
            history,
            &bundle,
            atlas_json,
            spritesheet_rgba,
            spritesheet_width,
        ),
        Err(error) => history.push_and_apply(Event::Log {
            msg: format!("Runtime asset loading failed: {error}"),
        }),
    }
}

pub fn generate_pg_rpg_log_bundle(
    history: &mut HistoryManager,
    bundle: &ScenarioBundle,
    atlas_json: &str,
    spritesheet_rgba: &[u8],
    spritesheet_width: u32,
) {
    let script = match bundle.root_rhai() {
        Ok(script) => script,
        Err(e) => {
            history.push_and_apply(Event::Log {
                msg: format!("Rhai bundle loading failed: {e}"),
            });
            return;
        }
    };
    if let Err(e) = scripting::generate_pg_rpg_log_rhai(
        history,
        &script,
        atlas_json,
        spritesheet_rgba,
        spritesheet_width,
    ) {
        history.push_and_apply(Event::Log {
            msg: format!("Rhai execution failed: {}", e),
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn runtime_bundle() -> Result<ScenarioBundle, String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web");
    ScenarioBundle::from_web_directory(&root)
}

#[cfg(target_arch = "wasm32")]
fn runtime_bundle() -> Result<ScenarioBundle, String> {
    Err("Wasm callers must provide the fetched ScenarioBundle".to_string())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use pystral_compiler::assets::AssetCollection;
    use pystral_core::log::Event;

    const TEST_ATLAS: &str = r#"{
        "width": 1,
        "height": 1,
        "spritestacks": {
            "Skeleton_Minion": [{"x": 0, "y": 0, "w": 1, "h": 1}],
            "Necromancer": [{"x": 0, "y": 0, "w": 1, "h": 1}],
            "Caveman": [{"x": 0, "y": 0, "w": 1, "h": 1}],
            "Mage": [{"x": 0, "y": 0, "w": 1, "h": 1}]
        }
    }"#;

    #[test]
    fn manifest_pg_rpg_promotes_arrow_and_rock_generation_to_rhai() {
        let mut history = HistoryManager::new();
        generate_pg_rpg_log(&mut history, TEST_ATLAS, &[255, 255, 255, 255], 1);
        assert!(history.log.iter().any(|event| matches!(
            event,
            Event::Log { msg } if msg.contains("Rhai pg_rpg NPC playout")
        )));
        for event in &history.log {
            if let Event::Log { msg } = event {
                eprintln!("pg_rpg log: {msg}");
            }
        }

        let collection_data = history
            .log
            .iter()
            .find_map(|event| {
                if let Event::DefineAssetCollection { name, data } = event {
                    (name == "primitives").then_some(data)
                } else {
                    None
                }
            })
            .expect("the Rhai pg_rpg must define the primitives collection");

        let collection = AssetCollection::from_binary(collection_data);
        let arrow = collection
            .spritestacks
            .get("Arrow")
            .expect("Rhai must define Arrow");
        let rock = collection
            .spritestacks
            .get("Rock")
            .expect("Rhai must define Rock");
        assert_eq!(
            (arrow.width, arrow.height, arrow.slices.len()),
            (32, 32, 32)
        );
        assert_eq!((rock.width, rock.height, rock.slices.len()), (64, 64, 64));
        assert!(
            arrow
                .slices
                .iter()
                .any(|slice| slice.color_data.iter().any(|byte| *byte != 0))
        );
        assert!(
            rock.slices
                .iter()
                .any(|slice| slice.color_data.iter().any(|byte| *byte != 0))
        );
        assert_eq!(
            history
                .current_state
                .entities
                .iter()
                .filter(|entity| {
                    entity.properties.get("asset")
                        == Some(&pystral_core::log::PropertyValue::String("Rock".into()))
                })
                .count(),
            5
        );
    }

    #[test]
    fn production_rhai_authors_generic_passive_reaction_and_movement_effects() {
        let script = runtime_bundle().unwrap().root_rhai().unwrap();
        let mut session = crate::rhai_session::RhaiSession::new(
            &script,
            HistoryManager::new(),
            TEST_ATLAS.to_string(),
            vec![255, 255, 255, 255],
            1,
        )
        .unwrap();
        let simulation = session.simulation().unwrap();
        let passive = |name: &str| {
            simulation
                .state
                .passive_registry
                .values()
                .find(|definition| definition.name == name)
                .unwrap()
        };
        assert_eq!(passive("Spell Echo").damage_multiplier, (2, 1));
        assert_eq!(passive("Death's Embrace").on_kill_heal, 20);
        assert_eq!(passive("Undead Resilience").stat_modifiers.len(), 2);
        let mana_shield = simulation
            .state
            .reaction_registry
            .values()
            .find(|definition| definition.name == "Mana Shield")
            .unwrap();
        assert_eq!((mana_shield.mana_cost, mana_shield.self_heal), (10, 5));
        let manafeet = simulation
            .state
            .movement_registry
            .values()
            .find(|definition| definition.name == "Manafeet")
            .unwrap();
        assert_eq!(manafeet.mana_gain, 5);
    }
}
