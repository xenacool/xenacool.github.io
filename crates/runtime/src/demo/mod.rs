pub mod animation;
pub mod scripting;
pub mod world;

use pystral_compiler::assets::AssetCollection;
use pystral_core::history::HistoryManager;
use pystral_core::log::Event;

pub fn setup_spritestack_assets(history: &mut HistoryManager) {
    let mut collection = AssetCollection::new();

    // Create a few simple shapes
    collection.add_arrow("Arrow", [200, 100, 50, 255], 0.05);

    // Character assets (SkeletonMinion, Necromancer, Caveman, Mage)
    // crate::character::register_assets(&mut collection);

    history.push_and_apply(Event::DefineAssetCollection {
        name: "primitives".to_string(),
        data: collection.to_binary(),
    });
}

pub fn generate_demo_log(history: &mut HistoryManager) {
    let script = include_str!("../../../../assets/scripts/demo.rhai");
    scripting::generate_demo_log_rhai(history, script).expect("Rhai execution failed");
}
