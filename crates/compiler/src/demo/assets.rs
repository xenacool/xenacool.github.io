use pystral_core::history::HistoryManager;
use pystral_core::log::Event;
use crate::assets::AssetCollection;

pub fn setup_spritestack_assets(history: &mut HistoryManager) {
    let mut collection = AssetCollection::new();
    
    // Create a few simple shapes
    collection.add_cube("CubeRed", 16, [255, 50, 50, 255], 0.05);
    collection.add_cube("CubeBlue", 16, [50, 50, 255, 255], 0.05);
    collection.add_cube("CubeGreen", 16, [50, 255, 50, 255], 0.05);
    collection.add_cube("CubeGray", 16, [150, 150, 150, 255], 0.05);

    history.push_and_apply(Event::DefineAssetCollection {
        name: "primitives".to_string(),
        data: collection.to_binary(),
    });
}
