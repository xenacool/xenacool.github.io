use pystral_compiler::assets::{AssetCollection, SpriteAtlas};
use pystral_compiler::spritestack_processing::process_slice;
use pystral_core::domain::{Spritestack, SpritestackSlice};
use pystral_core::history::HistoryManager;
use pystral_core::log::Event;
use rhai::Engine;
use std::collections::HashMap;

fn add_to_sprite_stacks(
    collection: &mut AssetCollection,
    name: &str,
    spacing: f32,
    width: u32,
    height: u32,
    slices: Vec<SpritestackSlice>,
) {
    collection.spritestacks.insert(
        name.to_string(),
        Spritestack {
            width,
            height,
            spacing,
            aabb: glam::Vec3::new(
                (width as f32 - 0.5) * spacing,
                (slices.len() as f32 - 1.0) * spacing,
                (height as f32 - 0.5) * spacing,
            ),
            slices,
        },
    );
}

pub(super) fn register_asset_management(engine: &mut Engine) {
    engine
        .register_type_with_name::<SpriteAtlas>("SpriteAtlas")
        .register_fn("load_atlas", |json: &str| {
            SpriteAtlas::from_json(json).unwrap_or_else(|_| SpriteAtlas {
                width: 0,
                height: 0,
                spritestacks: HashMap::new(),
            })
        });

    engine
        .register_type_with_name::<AssetCollection>("AssetCollection")
        .register_fn("new_asset_collection", || AssetCollection::new())
        .register_fn(
            "add_spritestack",
            |collection: &mut AssetCollection,
             name: &str,
             spacing: f64,
             width: i64,
             height: i64,
             values: rhai::Array|
             -> Result<(), Box<rhai::EvalAltResult>> {
                let mut slices = Vec::with_capacity(values.len());
                for value in values {
                    let map = value.try_cast::<rhai::Map>().ok_or_else(|| {
                        Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "Spritestack slice must be a map".into(),
                            rhai::Position::NONE,
                        ))
                    })?;
                    let color_data = map
                        .get("color_data")
                        .and_then(|value| value.clone().into_blob().ok())
                        .ok_or_else(|| {
                            Box::new(rhai::EvalAltResult::ErrorRuntime(
                                "Spritestack slice is missing color_data blob".into(),
                                rhai::Position::NONE,
                            ))
                        })?;
                    let normal_data = map
                        .get("normal_data")
                        .and_then(|value| value.clone().into_blob().ok())
                        .ok_or_else(|| {
                            Box::new(rhai::EvalAltResult::ErrorRuntime(
                                "Spritestack slice is missing normal_data blob".into(),
                                rhai::Position::NONE,
                            ))
                        })?;
                    slices.push(SpritestackSlice {
                        color_data,
                        normal_data,
                    });
                }
                let width = u32::try_from(width).map_err(|_| {
                    Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "Spritestack width must be non-negative".into(),
                        rhai::Position::NONE,
                    ))
                })?;
                let height = u32::try_from(height).map_err(|_| {
                    Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "Spritestack height must be non-negative".into(),
                        rhai::Position::NONE,
                    ))
                })?;
                let expected_buffer = (width as usize)
                    .checked_mul(height as usize)
                    .and_then(|pixels| pixels.checked_mul(4))
                    .ok_or_else(|| {
                        Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "Spritestack dimensions overflow buffer size".into(),
                            rhai::Position::NONE,
                        ))
                    })?;
                for slice in &mut slices {
                    if slice.normal_data.len() != expected_buffer {
                        return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            format!(
                                "Spritestack normal buffer has {} bytes; expected {}",
                                slice.normal_data.len(),
                                expected_buffer
                            )
                            .into(),
                            rhai::Position::NONE,
                        )));
                    }
                    process_slice(
                        width as usize,
                        height as usize,
                        &mut slice.color_data,
                        Default::default(),
                    )
                    .map_err(|error| {
                        Box::new(rhai::EvalAltResult::ErrorRuntime(
                            error.to_string().into(),
                            rhai::Position::NONE,
                        ))
                    })?;
                }
                add_to_sprite_stacks(collection, name, spacing as f32, width, height, slices);
                Ok(())
            },
        )
        .register_fn(
            "add_atlas_spritestack",
            |collection: &mut AssetCollection,
             name: &str,
             spacing: f64,
             atlas: SpriteAtlas,
             spritesheet_rgba: rhai::Blob,
             width: i64|
             -> Result<(), Box<rhai::EvalAltResult>> {
                let width = u32::try_from(width).map_err(|_| {
                    Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "Spritesheet width must be non-negative".into(),
                        rhai::Position::NONE,
                    ))
                })?;
                collection
                    .add_atlas_spritestack(name, spacing as f32, &atlas, &spritesheet_rgba, width)
                    .map_err(|error| {
                        Box::new(rhai::EvalAltResult::ErrorRuntime(
                            error.to_string().into(),
                            rhai::Position::NONE,
                        ))
                    })
            },
        );

    engine.register_fn(
        "define_asset_collection",
        |history: &mut HistoryManager, name: &str, collection: AssetCollection| {
            history.push_and_apply(Event::DefineAssetCollection {
                name: name.to_string(),
                data: collection.to_binary(),
            });
        },
    );
}
