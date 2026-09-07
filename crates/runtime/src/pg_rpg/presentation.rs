use pystral_core::history::HistoryManager;
use pystral_core::log::{Event, PropertyValue};
use rhai::{Dynamic, Engine};

fn runtime_error(error: impl Into<String>) -> Box<rhai::EvalAltResult> {
    Box::new(rhai::EvalAltResult::ErrorRuntime(
        error.into().into(),
        rhai::Position::NONE,
    ))
}

#[derive(Clone, Debug)]
struct PresentationConfig {
    reachable_color: [f32; 3],
    path_color: [f32; 3],
    selected_color: [f32; 3],
    reachable_opacity: f32,
    path_opacity: f32,
    selected_opacity: f32,
    marker_scale: f32,
    animation_crossfade_ms: f32,
    reduced_motion_policy: String,
    reachable_color_valid: bool,
    path_color_valid: bool,
    selected_color_valid: bool,
}

impl Default for PresentationConfig {
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
            reduced_motion_policy: "snap".to_string(),
            reachable_color_valid: true,
            path_color_valid: true,
            selected_color_valid: true,
        }
    }
}

fn parse_color(value: rhai::Array) -> ([f32; 3], bool) {
    let values: Vec<f32> = value
        .into_iter()
        .filter_map(|item| {
            item.as_float()
                .ok()
                .or_else(|| item.as_int().ok().map(|v| v as f64))
        })
        .map(|value| value as f32)
        .collect();
    if values.len() == 3
        && values
            .iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
    {
        ([values[0], values[1], values[2]], true)
    } else {
        ([0.0; 3], false)
    }
}

fn install(
    history: &mut HistoryManager,
    config: PresentationConfig,
) -> Result<(), Box<rhai::EvalAltResult>> {
    if !config.reachable_color_valid || !config.path_color_valid || !config.selected_color_valid {
        return Err(runtime_error(
            "Presentation colors must be RGB arrays with values from 0 to 1",
        ));
    }
    if !matches!(
        config.reduced_motion_policy.to_ascii_lowercase().as_str(),
        "snap" | "crossfade"
    ) {
        return Err(runtime_error(format!(
            "Unknown reduced motion policy: {}",
            config.reduced_motion_policy
        )));
    }
    let Some(world_id) = history
        .current_state
        .entities
        .iter()
        .find(|entity| entity.kind == "world")
        .map(|entity| entity.id)
    else {
        return Err(runtime_error(
            "Cannot install presentation config before spawning the world",
        ));
    };
    let values = [
        (
            "presentation_waypoint_reachable_color",
            PropertyValue::Color(config.reachable_color),
        ),
        (
            "presentation_waypoint_path_color",
            PropertyValue::Color(config.path_color),
        ),
        (
            "presentation_waypoint_selected_color",
            PropertyValue::Color(config.selected_color),
        ),
        (
            "presentation_waypoint_reachable_opacity",
            PropertyValue::Float(config.reachable_opacity.clamp(0.0, 1.0)),
        ),
        (
            "presentation_waypoint_path_opacity",
            PropertyValue::Float(config.path_opacity.clamp(0.0, 1.0)),
        ),
        (
            "presentation_waypoint_selected_opacity",
            PropertyValue::Float(config.selected_opacity.clamp(0.0, 1.0)),
        ),
        (
            "presentation_waypoint_marker_scale",
            PropertyValue::Float(config.marker_scale.max(0.01)),
        ),
        (
            "presentation_animation_crossfade_ms",
            PropertyValue::Float(config.animation_crossfade_ms.max(0.0)),
        ),
        (
            "presentation_reduced_motion_policy",
            PropertyValue::String(config.reduced_motion_policy.to_ascii_lowercase()),
        ),
    ];
    for (property, value) in values {
        history.push_and_apply(Event::UpdateProperty {
            id: world_id,
            property: property.to_string(),
            value,
        });
    }
    Ok(())
}

pub fn register_rhai(engine: &mut Engine) {
    engine
        .register_type_with_name::<PresentationConfig>("PresentationConfig")
        .register_fn("new_presentation_config", PresentationConfig::default)
        .register_get("reachable_color", |config: &mut PresentationConfig| {
            config
                .reachable_color
                .into_iter()
                .map(|v| Dynamic::from(v as f64))
                .collect::<rhai::Array>()
        })
        .register_set(
            "reachable_color",
            |config: &mut PresentationConfig, value: rhai::Array| {
                let (color, valid) = parse_color(value);
                config.reachable_color = color;
                config.reachable_color_valid = valid;
            },
        )
        .register_get("path_color", |config: &mut PresentationConfig| {
            config
                .path_color
                .into_iter()
                .map(|v| Dynamic::from(v as f64))
                .collect::<rhai::Array>()
        })
        .register_set(
            "path_color",
            |config: &mut PresentationConfig, value: rhai::Array| {
                let (color, valid) = parse_color(value);
                config.path_color = color;
                config.path_color_valid = valid;
            },
        )
        .register_get("selected_color", |config: &mut PresentationConfig| {
            config
                .selected_color
                .into_iter()
                .map(|v| Dynamic::from(v as f64))
                .collect::<rhai::Array>()
        })
        .register_set(
            "selected_color",
            |config: &mut PresentationConfig, value: rhai::Array| {
                let (color, valid) = parse_color(value);
                config.selected_color = color;
                config.selected_color_valid = valid;
            },
        )
        .register_get("reachable_opacity", |c: &mut PresentationConfig| {
            c.reachable_opacity as f64
        })
        .register_set("reachable_opacity", |c: &mut PresentationConfig, v: f64| {
            c.reachable_opacity = v as f32
        })
        .register_get("path_opacity", |c: &mut PresentationConfig| {
            c.path_opacity as f64
        })
        .register_set("path_opacity", |c: &mut PresentationConfig, v: f64| {
            c.path_opacity = v as f32
        })
        .register_get("selected_opacity", |c: &mut PresentationConfig| {
            c.selected_opacity as f64
        })
        .register_set("selected_opacity", |c: &mut PresentationConfig, v: f64| {
            c.selected_opacity = v as f32
        })
        .register_get("marker_scale", |c: &mut PresentationConfig| {
            c.marker_scale as f64
        })
        .register_set("marker_scale", |c: &mut PresentationConfig, v: f64| {
            c.marker_scale = v as f32
        })
        .register_get("animation_crossfade_ms", |c: &mut PresentationConfig| {
            c.animation_crossfade_ms as f64
        })
        .register_set(
            "animation_crossfade_ms",
            |c: &mut PresentationConfig, v: f64| c.animation_crossfade_ms = v as f32,
        )
        .register_set(
            "animation_crossfade_ms",
            |c: &mut PresentationConfig, v: i64| c.animation_crossfade_ms = v as f32,
        )
        .register_get("reduced_motion_policy", |c: &mut PresentationConfig| {
            c.reduced_motion_policy.clone()
        })
        .register_set(
            "reduced_motion_policy",
            |c: &mut PresentationConfig, v: String| c.reduced_motion_policy = v,
        );
    engine.register_fn("install_presentation_config", install);
}
