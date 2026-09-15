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
    pub animation_crossfade_ms: f32,
    pub attack_turn_in_ms: f32,
    pub attack_turn_out_ms: f32,
    pub reduced_motion_policy: String,
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
            animation_crossfade_ms: 120.0,
            attack_turn_in_ms: 150.0,
            attack_turn_out_ms: 180.0,
            reduced_motion_policy: "snap".to_string(),
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
    let string = |name, fallback: &str| match world.properties.get(name) {
        Some(PropertyValue::String(value)) => value.clone(),
        _ => fallback.to_string(),
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
        animation_crossfade_ms: float(
            "presentation_animation_crossfade_ms",
            defaults.animation_crossfade_ms,
        )
        .max(0.0),
        attack_turn_in_ms: float("presentation_attack_turn_in_ms", defaults.attack_turn_in_ms)
            .max(0.0),
        attack_turn_out_ms: float(
            "presentation_attack_turn_out_ms",
            defaults.attack_turn_out_ms,
        )
        .max(0.0),
        reduced_motion_policy: string(
            "presentation_reduced_motion_policy",
            &defaults.reduced_motion_policy,
        ),
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
        // The source is necessary for route semantics but belongs to the
        // acting unit, not the viewport overlay. Waypoint materials ignore
        // depth by design, so drawing it would visibly pierce that unit.
        path: preview
            .path
            .iter()
            .filter(|waypoint| {
                waypoint.hex != preview.source.hex || waypoint.layer != preview.source.layer
            })
            .map(render_move)
            .collect(),
        selected_destination: preview.selected_destination.as_ref().map(render_move),
    }
}

#[cfg(test)]
mod tests {
    use super::waypoint_preview;
    use crate::MovePreview;
    use hexx::Hex;
    use pystral_core::log::{AvailableMove, WorldState};

    fn cell(q: i32, r: i32) -> AvailableMove {
        AvailableMove {
            hex: Hex::new(q, r),
            layer: 0,
            ap_cost: 1,
        }
    }

    #[test]
    fn preview_projection_omits_the_actor_source_waypoint() {
        let source = cell(0, 0);
        let destination = cell(1, 0);
        let frame = waypoint_preview(
            &WorldState::default(),
            &MovePreview {
                request_id: 1,
                unit_id: 7,
                source: source.clone(),
                reachable: vec![destination.clone()],
                selected_destination: Some(destination.clone()),
                path: vec![source, destination],
            },
        );
        assert_eq!(frame.path.len(), 1);
        assert_eq!((frame.path[0].q, frame.path[0].r), (1, 0));
    }
}
