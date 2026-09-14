use super::{RenderMapFrame, RenderMaterialFrame, RenderTileFrame};
use pystral_core::log::{PropertyValue, WorldState};
use std::collections::BTreeMap;

pub(super) fn scene_data(
    state: &WorldState,
) -> (
    Option<RenderMapFrame>,
    BTreeMap<String, RenderMaterialFrame>,
) {
    let map = state
        .entities
        .iter()
        .find(|entity| entity.kind == "world")
        .and_then(|entity| match entity.properties.get("map") {
            Some(PropertyValue::HexMap(map)) => Some(RenderMapFrame {
                orientation: format!("{:?}", map.orientation),
                hex_size: map.hex_size.to_array(),
                tiles: map
                    .tiles
                    .iter()
                    .map(|tile| RenderTileFrame {
                        q: tile.hex.x,
                        r: tile.hex.y,
                        layer: tile.layer,
                        bottom: tile.bottom,
                        height: tile.height,
                        material: tile.material.clone(),
                    })
                    .collect(),
            }),
            _ => None,
        });
    let materials = state
        .materials
        .iter()
        .map(|(name, material)| {
            (
                name.clone(),
                RenderMaterialFrame {
                    color: material.color,
                    roughness: material.roughness,
                    metalness: material.metalness,
                    emissive: material.emissive,
                },
            )
        })
        .collect();
    (map, materials)
}
