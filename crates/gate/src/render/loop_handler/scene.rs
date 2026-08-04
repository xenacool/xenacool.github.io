use crate::render::context::RenderContext;
use crate::render::utils::{EntityExt, RenderResultExt};
use pystral_core::log::{EntityState, PropertyValue, WorldState};
use std::collections::HashMap;
use tween::{SineInOut, Tweener};

fn tile_top_height(map: &pystral_core::domain::HexMap, hex: hexx::Hex, layer: i32) -> f32 {
    map.tiles
        .iter()
        .filter(|tile| tile.hex == hex && tile.layer == layer)
        .map(|tile| tile.bottom + tile.height)
        .fold(0.0, f32::max)
}

/// Units attach to their cell's terrain. `z` belongs exclusively to flight.
fn resolved_vertical_anchor(
    entity: &EntityState,
    terrain_top: f32,
    worker_tx: &futures::channel::mpsc::UnboundedSender<crate::WorkerInput>,
) -> f32 {
    if entity.kind != "projectile" {
        return terrain_top;
    }
    match entity.properties.get("z") {
        Some(PropertyValue::Float(value)) => *value,
        Some(_) => entity.get_float("z", terrain_top).log_fallback(worker_tx),
        None => terrain_top,
    }
}

/// The sole Rust-to-Three position resolver. JavaScript only applies these
/// positions and asset slices; it does not infer tactical elevation.
pub fn resolved_entity_world_positions(
    ctx: &mut RenderContext,
    worker_tx: &futures::channel::mpsc::UnboundedSender<crate::WorkerInput>,
    state: &WorldState,
    now: f64,
) -> HashMap<u64, [f32; 3]> {
    let map = state
        .entities
        .iter()
        .find(|entity| entity.kind == "world")
        .map(|world| world.get_hex_map().log_fallback(worker_tx));
    let layout = map.as_ref().map_or_else(
        hexx::HexLayout::default,
        pystral_core::domain::HexMap::layout,
    );
    state
        .entities
        .iter()
        .filter(|entity| !matches!(entity.kind.as_str(), "world" | "camera"))
        .map(|entity| {
            let mut position = layout.hex_to_world_pos(entity.hex);
            let mut terrain = (
                entity.hex,
                entity
                    .properties
                    .get("layer")
                    .and_then(|value| match value {
                        PropertyValue::Float(value) => Some(*value as i32),
                        _ => None,
                    })
                    .unwrap_or(0),
            );
            let mut transition = None;
            if let Some(tween) = ctx.movement_tweens.get_mut(&entity.id) {
                let start = layout.hex_to_world_pos(tween.from_hex);
                let end = layout.hex_to_world_pos(tween.to_hex);
                let values = tween.tweeners.get_or_insert_with(|| {
                    std::array::from_fn(|axis| {
                        Tweener::new_at(
                            [start.x, 0.0, start.y][axis],
                            [end.x, 0.0, end.y][axis],
                            tween.duration_ms,
                            SineInOut,
                            0.0,
                        )
                    })
                });
                let elapsed = (now - tween.start_time_ms).max(0.0);
                position.x = values[0].move_to(elapsed);
                position.y = values[2].move_to(elapsed);
                let progress = (elapsed / tween.duration_ms.max(1.0)).clamp(0.0, 1.0) as f32;
                terrain = if progress < 0.5 {
                    (tween.from_hex, tween.from_layer)
                } else {
                    (tween.to_hex, tween.to_layer)
                };
                transition = Some((
                    tween.from_hex,
                    tween.from_layer,
                    tween.to_hex,
                    tween.to_layer,
                    progress,
                ));
            }
            if let Some(PropertyValue::Float(value)) = entity.properties.get("world_x") {
                position.x = *value;
            }
            if let Some(PropertyValue::Float(value)) = entity.properties.get("world_y") {
                position.y = *value;
            }
            let terrain_top = map
                .as_ref()
                .map(|map| {
                    transition.map_or_else(
                        || tile_top_height(map, terrain.0, terrain.1),
                        |(from_hex, from_layer, to_hex, to_layer, progress)| {
                            let from = tile_top_height(map, from_hex, from_layer);
                            let to = tile_top_height(map, to_hex, to_layer);
                            from + (to - from) * progress
                        },
                    )
                })
                .unwrap_or(0.0);
            (
                entity.id,
                [
                    position.x,
                    resolved_vertical_anchor(entity, terrain_top, worker_tx),
                    position.y,
                ],
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{resolved_vertical_anchor, tile_top_height};
    use futures::channel::mpsc;
    use hexx::Hex;
    use pystral_core::domain::{HexMap, HexTile};
    use pystral_core::log::{EntityState, PropertyValue};
    #[test]
    fn character_ignores_stale_z_and_uses_terrain_anchor() {
        let mut entity = EntityState::new(1, "character".into(), Hex::ZERO, &[]);
        entity
            .properties
            .insert("z".into(), PropertyValue::Float(0.0));
        let (tx, _) = mpsc::unbounded();
        assert_eq!(resolved_vertical_anchor(&entity, 3.5, &tx), 3.5);
    }
    #[test]
    fn low_layer_never_uses_wall_cap_height() {
        let map = HexMap {
            tiles: vec![
                HexTile {
                    hex: Hex::ZERO,
                    layer: 0,
                    bottom: 0.0,
                    height: 0.1,
                    material: "dirt".into(),
                },
                HexTile {
                    hex: Hex::ZERO,
                    layer: 9,
                    bottom: 4.5,
                    height: 0.5,
                    material: "rock".into(),
                },
            ],
            ..HexMap::default()
        };
        assert_eq!(tile_top_height(&map, Hex::ZERO, 0), 0.1);
        assert_eq!(tile_top_height(&map, Hex::ZERO, 9), 5.0);
    }
}
