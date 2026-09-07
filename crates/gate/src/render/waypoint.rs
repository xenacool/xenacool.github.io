use crate::MovePreview;
use pystral_core::log::{AvailableMove, PropertyValue, WorldState};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RenderWaypointFrame {
    pub q: i32,
    pub r: i32,
    pub layer: i32,
    pub ap_cost: u8,
    pub world_position: [f32; 3],
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RenderWaypointPreviewFrame {
    pub unit_id: u64,
    pub reachable: Vec<RenderWaypointFrame>,
    pub path: Vec<RenderWaypointFrame>,
    pub selected_destination: Option<RenderWaypointFrame>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RenderPresentationConfig {
    pub reachable_color: [f32; 3],
    pub path_color: [f32; 3],
    pub selected_color: [f32; 3],
    pub reachable_opacity: f32,
    pub path_opacity: f32,
    pub selected_opacity: f32,
    pub marker_scale: f32,
}

impl Default for RenderPresentationConfig {
    fn default() -> Self {
        Self {
            reachable_color: [0.25, 0.65, 1.0],
            path_color: [0.35, 0.9, 1.0],
            selected_color: [1.0, 0.9, 0.25],
            reachable_opacity: 0.28,
            path_opacity: 0.62,
            selected_opacity: 0.95,
            marker_scale: 1.0,
        }
    }
}

pub fn presentation_config(state: &WorldState) -> RenderPresentationConfig {
    let defaults = RenderPresentationConfig::default();
    let Some(world) = state.entities.iter().find(|entity| entity.kind == "world") else {
        return defaults;
    };
    let color = |name, fallback| match world.properties.get(name) {
        Some(PropertyValue::Color(value)) => *value,
        _ => fallback,
    };
    let float = |name, fallback| match world.properties.get(name) {
        Some(PropertyValue::Float(value)) if value.is_finite() => *value,
        _ => fallback,
    };
    RenderPresentationConfig {
        reachable_color: color(
            "presentation_waypoint_reachable_color",
            defaults.reachable_color,
        ),
        path_color: color("presentation_waypoint_path_color", defaults.path_color),
        selected_color: color(
            "presentation_waypoint_selected_color",
            defaults.selected_color,
        ),
        reachable_opacity: float(
            "presentation_waypoint_reachable_opacity",
            defaults.reachable_opacity,
        )
        .clamp(0.0, 1.0),
        path_opacity: float("presentation_waypoint_path_opacity", defaults.path_opacity)
            .clamp(0.0, 1.0),
        selected_opacity: float(
            "presentation_waypoint_selected_opacity",
            defaults.selected_opacity,
        )
        .clamp(0.0, 1.0),
        marker_scale: float("presentation_waypoint_marker_scale", defaults.marker_scale).max(0.01),
    }
}

pub fn waypoint_preview(state: &WorldState, preview: &MovePreview) -> RenderWaypointPreviewFrame {
    let map = state
        .entities
        .iter()
        .find(|entity| entity.kind == "world")
        .and_then(|world| match world.properties.get("map") {
            Some(PropertyValue::HexMap(map)) => Some(map),
            _ => None,
        });
    let layout = map.map_or_else(hexx::HexLayout::default, |map| map.layout());
    let render_move = |available: &AvailableMove| RenderWaypointFrame {
        q: available.hex.x,
        r: available.hex.y,
        layer: available.layer,
        ap_cost: available.ap_cost,
        world_position: {
            let p = layout.hex_to_world_pos(available.hex);
            [p.x, 0.0, p.y]
        },
    };
    RenderWaypointPreviewFrame {
        unit_id: preview.unit_id,
        reachable: preview.reachable.iter().map(render_move).collect(),
        path: preview.path.iter().map(render_move).collect(),
        selected_destination: preview.selected_destination.as_ref().map(render_move),
    }
}
