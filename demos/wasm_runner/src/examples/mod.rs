//! All examples as wrapper modules.
//! Each wrapper provides `mod common` via #[path] before including the example file.
use bevy::prelude::*;

pub mod gm_appearance;
pub mod gm_arbitrary_shapes;
pub mod gm_character_creator;
pub mod gm_composed_shapes;
pub mod gm_curves;
pub mod gm_deformation;
pub mod gm_gradient_2d;
pub mod gm_immediate;
pub mod gm_instanced;
pub mod gm_mix_spaces;
pub mod gm_pbr_effects;
pub mod gm_polyline;
pub mod gm_retained;
pub mod gm_road;
pub mod gm_sdf_2d;
pub mod gm_sdf_dash_gallery;
pub mod gm_shapes;
pub mod gm_standalone;
pub mod gm_styles;
pub mod gm_sweep;
pub mod gm_terrain;
pub mod cd_blend_square;
pub mod cd_chain_of_cells;
pub mod cd_clock;
pub mod cd_gizmo_3d;
pub mod cd_gradient_hanoi_3d;
pub mod cd_gradient_kanban_2d;
pub mod cd_hello;
pub mod cd_lists_in_lists;
pub mod cd_reorder_list;
pub mod cd_ring_of_beads;
pub mod cd_scheduler;
pub mod cd_slider;
pub mod cd_tactile_tessellations;
pub mod cd_tessellations;
pub mod cd_three_way;
pub mod cd_toggle;

/// Dispatch to the named example
pub fn dispatch(name: &str, app: &mut App) {
    match name {
        "appearance" => gm_appearance::run(app),
        "arbitrary_shapes" => gm_arbitrary_shapes::run(app),
        "character_creator" => gm_character_creator::run(app),
        "composed_shapes" => gm_composed_shapes::run(app),
        "curves" => gm_curves::run(app),
        "deformation" => gm_deformation::run(app),
        "gradient_2d" => gm_gradient_2d::run(app),
        "immediate" => gm_immediate::run(app),
        "instanced" => gm_instanced::run(app),
        "mix_spaces" => gm_mix_spaces::run(app),
        "pbr_effects" => gm_pbr_effects::run(app),
        "polyline" => gm_polyline::run(app),
        "retained" => gm_retained::run(app),
        "road" => gm_road::run(app),
        "sdf_2d" => gm_sdf_2d::run(app),
        "sdf_dash_gallery" => gm_sdf_dash_gallery::run(app),
        "shapes" => gm_shapes::run(app),
        "standalone" => gm_standalone::run(app),
        "styles" => gm_styles::run(app),
        "sweep" => gm_sweep::run(app),
        "terrain" => gm_terrain::run(app),
        "blend_square" => cd_blend_square::run(app),
        "chain_of_cells" => cd_chain_of_cells::run(app),
        "clock" => cd_clock::run(app),
        "gizmo_3d" => cd_gizmo_3d::run(app),
        "gradient_hanoi_3d" => cd_gradient_hanoi_3d::run(app),
        "gradient_kanban_2d" => cd_gradient_kanban_2d::run(app),
        "hello" => cd_hello::run(app),
        "lists_in_lists" => cd_lists_in_lists::run(app),
        "reorder_list" => cd_reorder_list::run(app),
        "ring_of_beads" => cd_ring_of_beads::run(app),
        "scheduler" => cd_scheduler::run(app),
        "slider" => cd_slider::run(app),
        "tactile_tessellations" => cd_tactile_tessellations::run(app),
        "tessellations" => cd_tessellations::run(app),
        "three_way" => cd_three_way::run(app),
        "toggle" => cd_toggle::run(app),
        _ => {
            bevy::log::warn!("Unknown example: {}. Showing shapes.", name);
            gm_shapes::run(app);
        }
    }
}
