//! All examples as modules. Each provides `pub fn run(app: &mut App)`.
use bevy::prelude::*;

// bevy_gradmesh examples
#[path = "../../libs/bevy_gradmesh/examples/shapes.rs"]
pub mod shapes;
#[path = "../../libs/bevy_gradmesh/examples/styles.rs"]
pub mod styles;
#[path = "../../libs/bevy_gradmesh/examples/mix_spaces.rs"]
pub mod mix_spaces;
#[path = "../../libs/bevy_gradmesh/examples/pbr_effects.rs"]
pub mod pbr_effects;
#[path = "../../libs/bevy_gradmesh/examples/gradient_2d.rs"]
pub mod gradient_2d;
#[path = "../../libs/bevy_gradmesh/examples/sdf_2d.rs"]
pub mod sdf_2d;
#[path = "../../libs/bevy_gradmesh/examples/sdf_dash_gallery.rs"]
pub mod sdf_dash_gallery;
#[path = "../../libs/bevy_gradmesh/examples/immediate.rs"]
pub mod immediate;
#[path = "../../libs/bevy_gradmesh/examples/retained.rs"]
pub mod retained;
#[path = "../../libs/bevy_gradmesh/examples/instanced.rs"]
pub mod instanced;
#[path = "../../libs/bevy_gradmesh/examples/standalone.rs"]
pub mod standalone;
#[path = "../../libs/bevy_gradmesh/examples/curves.rs"]
pub mod curves;
#[path = "../../libs/bevy_gradmesh/examples/polyline.rs"]
pub mod polyline;
#[path = "../../libs/bevy_gradmesh/examples/sweep.rs"]
pub mod sweep;
#[path = "../../libs/bevy_gradmesh/examples/road.rs"]
pub mod road;
#[path = "../../libs/bevy_gradmesh/examples/composed_shapes.rs"]
pub mod composed_shapes;
#[path = "../../libs/bevy_gradmesh/examples/terrain.rs"]
pub mod terrain;
#[path = "../../libs/bevy_gradmesh/examples/deformation.rs"]
pub mod deformation;
#[path = "../../libs/bevy_gradmesh/examples/appearance.rs"]
pub mod appearance;
#[path = "../../libs/bevy_gradmesh/examples/arbitrary_shapes.rs"]
pub mod arbitrary_shapes;

/// Dispatch to the named example
pub fn dispatch(name: &str, app: &mut App) {
    match name {
        "shapes" => shapes::run(app),
        "styles" => styles::run(app),
        "mix_spaces" => mix_spaces::run(app),
        "pbr_effects" => pbr_effects::run(app),
        "gradient_2d" => gradient_2d::run(app),
        "sdf_2d" => sdf_2d::run(app),
        "sdf_dash_gallery" => sdf_dash_gallery::run(app),
        "immediate" => immediate::run(app),
        "retained" => retained::run(app),
        "instanced" => instanced::run(app),
        "standalone" => standalone::run(app),
        "curves" => curves::run(app),
        "polyline" => polyline::run(app),
        "sweep" => sweep::run(app),
        "road" => road::run(app),
        "composed_shapes" => composed_shapes::run(app),
        "terrain" => terrain::run(app),
        "deformation" => deformation::run(app),
        "appearance" => appearance::run(app),
        "arbitrary_shapes" => arbitrary_shapes::run(app),
        _ => {
            bevy::log::warn!("Unknown example: {}. Showing shapes.", name);
            shapes::run(app);
        }
    }
}
