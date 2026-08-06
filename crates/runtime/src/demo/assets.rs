use pystral_core::history::HistoryManager;
use pystral_core::log::Event;
use pystral_compiler::assets::AssetCollection;

pub fn setup_spritestack_assets(history: &mut HistoryManager) {
    let mut collection = AssetCollection::new();
    
    // Create a few simple shapes
    collection.add_arrow("Arrow", [200, 100, 50, 255], 0.05);
    collection.add_rock("Rock", 64, [100, 100, 100, 255], 0.05);
    
    // Spider assets
    collection.add_sphere("SpiderThorax", 18, [20, 20, 20, 255], 0.05, 0.8);
    collection.add_sphere("SpiderAbdomen", 27, [15, 15, 15, 255], 0.05, 0.9);
    collection.add_sphere("SpiderJoint", 9, [40, 40, 40, 255], 0.05, 0.5);

    history.push_and_apply(Event::DefineAssetCollection {
        name: "primitives".to_string(),
        data: collection.to_binary(),
    });
}
