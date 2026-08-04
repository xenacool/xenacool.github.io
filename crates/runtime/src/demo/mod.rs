pub mod ability_targets;
pub mod animation;
pub mod bundle;
pub mod scripting;
pub mod simulation;

pub use bundle::{AssetManifest, NamedBinaryAsset, NamedTextAsset, ScenarioBundle};
use pystral_core::history::HistoryManager;
use pystral_core::log::Event;

pub fn generate_demo_log(
    history: &mut HistoryManager,
    atlas_json: &str,
    spritesheet_rgba: &[u8],
    spritesheet_width: u32,
) {
    match runtime_bundle() {
        Ok(bundle) => generate_demo_log_bundle(
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

pub fn generate_demo_log_bundle(
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
    if let Err(e) = scripting::generate_demo_log_rhai(
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

    #[test]
    fn manifest_demo_promotes_arrow_and_rock_generation_to_rhai() {
        let mut history = HistoryManager::new();
        let atlas_json = r#"{
            "width": 1,
            "height": 1,
            "spritestacks": {
                "Skeleton_Minion": [{"x": 0, "y": 0, "w": 1, "h": 1}],
                "Necromancer": [{"x": 0, "y": 0, "w": 1, "h": 1}],
                "Caveman": [{"x": 0, "y": 0, "w": 1, "h": 1}],
                "Mage": [{"x": 0, "y": 0, "w": 1, "h": 1}]
            }
        }"#;
        generate_demo_log(&mut history, atlas_json, &[255, 255, 255, 255], 1);
        assert!(history.log.iter().any(|event| matches!(
            event,
            Event::Log { msg } if msg.contains("Rhai demo NPC playout")
        )));
        for event in &history.log {
            if let Event::Log { msg } = event {
                eprintln!("demo log: {msg}");
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
            .expect("the Rhai demo must define the primitives collection");

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
}
